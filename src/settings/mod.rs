pub mod providers;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::vault::VaultRepository;
use crate::vault::crypto::VaultCrypto;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSettings {
    pub base_url: String,
    pub llm_model: String,
    pub llm_max_tokens: i64,
    pub embedding_model: String,
    pub embedding_dimensions: i64,
    pub embedding_endpoint: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveProviderRequest {
    pub base_url: String,
    pub api_key: Option<String>,
    pub llm_model: String,
    pub llm_max_tokens: Option<i64>,
    pub embedding_model: String,
    pub embedding_dimensions: Option<i64>,
    pub embedding_endpoint: Option<String>,
}

pub struct ProviderSettingsRepository<'a> {
    conn: &'a Connection,
}

impl<'a> ProviderSettingsRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn get(&self) -> anyhow::Result<Option<ProviderSettings>> {
        let mut stmt = self.conn.prepare(
            "SELECT base_url, llm_model, llm_max_tokens, embedding_model, embedding_dimensions, embedding_endpoint
             FROM provider_settings WHERE id = 1"
        )?;
        let mut rows = stmt.query([],)?;
        if let Some(row) = rows.next()? {
            Ok(Some(ProviderSettings {
                base_url: row.get::<_, String>(0)?,
                llm_model: row.get::<_, String>(1)?,
                llm_max_tokens: row.get::<_, i64>(2)?,
                embedding_model: row.get::<_, String>(3)?,
                embedding_dimensions: row.get::<_, i64>(4)?,
                embedding_endpoint: row.get::<_, Option<String>>(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn save(
        &self,
        master_key: &[u8; 32],
        req: &SaveProviderRequest,
    ) -> anyhow::Result<()> {
        let vault = VaultRepository::new(self.conn);
        let key_version = vault.get_active_master_key_version()?
            .ok_or_else(|| anyhow::anyhow!("no active vault key"))?;

        let (enc_dek, dek_nonce, ciphertext, ct_nonce) = if let Some(ref api_key) = req.api_key {
            if api_key.is_empty() {
                (Vec::<u8>::new(), Vec::<u8>::new(), Vec::<u8>::new(), Vec::<u8>::new())
            } else {
                let dek = VaultCrypto::generate_dek();
                let enc_dek = VaultCrypto::encrypt_dek(master_key, &dek)?;
                let enc_val = VaultCrypto::encrypt_value(&dek, api_key.as_bytes())?;
                (enc_dek.ciphertext, enc_dek.nonce, enc_val.ciphertext, enc_val.nonce)
            }
        } else {
            (Vec::<u8>::new(), Vec::<u8>::new(), Vec::<u8>::new(), Vec::<u8>::new())
        };

        let llm_max_tokens = req.llm_max_tokens.unwrap_or(8192);
        let embedding_dimensions = req.embedding_dimensions.unwrap_or(1024);

        self.conn.execute(
            "INSERT INTO provider_settings (id, base_url, api_key_encrypted, api_key_dek_nonce, api_key_ciphertext, api_key_ciphertext_nonce, api_key_key_version, llm_model, llm_max_tokens, embedding_model, embedding_dimensions, embedding_endpoint)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
                base_url = excluded.base_url,
                api_key_encrypted = excluded.api_key_encrypted,
                api_key_dek_nonce = excluded.api_key_dek_nonce,
                api_key_ciphertext = excluded.api_key_ciphertext,
                api_key_ciphertext_nonce = excluded.api_key_ciphertext_nonce,
                api_key_key_version = excluded.api_key_key_version,
                llm_model = excluded.llm_model,
                llm_max_tokens = excluded.llm_max_tokens,
                embedding_model = excluded.embedding_model,
                embedding_dimensions = excluded.embedding_dimensions,
                embedding_endpoint = excluded.embedding_endpoint,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            params![
                req.base_url,
                enc_dek,
                dek_nonce,
                ciphertext,
                ct_nonce,
                key_version,
                req.llm_model,
                llm_max_tokens,
                req.embedding_model,
                embedding_dimensions,
                req.embedding_endpoint,
            ],
        )?;

        info!("provider settings saved: base_url={}", req.base_url);
        Ok(())
    }

    pub fn get_api_key(&self, master_key: &[u8; 32]) -> anyhow::Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT api_key_encrypted, api_key_dek_nonce, api_key_ciphertext, api_key_ciphertext_nonce
             FROM provider_settings WHERE id = 1"
        )?;
        let mut rows = stmt.query([],)?;

        if let Some(row) = rows.next()? {
            let ed: Vec<u8> = row.get(0)?;
            let dn: Vec<u8> = row.get(1)?;
            let ct: Vec<u8> = row.get(2)?;
            let cn: Vec<u8> = row.get(3)?;

            if ct.is_empty() {
                return Ok(None);
            }

            let dek = VaultCrypto::decrypt_dek(master_key, &ed, &dn)?;
            let plaintext = VaultCrypto::decrypt_value(&dek, &ct, &cn)?;
            Ok(Some(String::from_utf8(plaintext)?))
        } else {
            Ok(None)
        }
    }

    pub fn delete(&self) -> anyhow::Result<bool> {
        let affected = self.conn.execute("DELETE FROM provider_settings WHERE id = 1", [])?;
        Ok(affected > 0)
    }
}
