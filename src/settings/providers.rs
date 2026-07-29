use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::vault::VaultRepository;
use crate::vault::crypto::VaultCrypto;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: i64,
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub api_key_set: bool,
    pub enabled: bool,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModel {
    pub id: i64,
    pub provider_id: i64,
    pub model_id: String,
    pub model_type: String,
    pub display_name: Option<String>,
    pub context_window: Option<i64>,
    pub max_output: Option<i64>,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_reasoning: bool,
    pub supports_audio: bool,
    pub supports_video: bool,
    pub input_modalities: String,
    pub output_modalities: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub enabled: Option<bool>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProviderRequest {
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub enabled: Option<bool>,
    pub is_default: Option<bool>,
}

pub struct ProvidersRepository<'a> {
    conn: &'a Connection,
}

impl<'a> ProvidersRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn list(&self) -> anyhow::Result<Vec<Provider>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, type, base_url,
                    CASE WHEN api_key_ciphertext IS NOT NULL AND length(api_key_ciphertext) > 0 THEN 1 ELSE 0 END,
                    enabled, is_default, created_at, updated_at
             FROM providers ORDER BY is_default DESC, name"
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Provider {
                id: r.get(0)?,
                name: r.get(1)?,
                provider_type: r.get(2)?,
                base_url: r.get(3)?,
                api_key_set: r.get::<_, i64>(4)? != 0,
                enabled: r.get::<_, i64>(5)? != 0,
                is_default: r.get::<_, i64>(6)? != 0,
                created_at: r.get(7)?,
                updated_at: r.get(8)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn get(&self, id: i64) -> anyhow::Result<Option<Provider>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, type, base_url,
                    CASE WHEN api_key_ciphertext IS NOT NULL AND length(api_key_ciphertext) > 0 THEN 1 ELSE 0 END,
                    enabled, is_default, created_at, updated_at
             FROM providers WHERE id = ?1"
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Provider {
                id: row.get(0)?,
                name: row.get(1)?,
                provider_type: row.get(2)?,
                base_url: row.get(3)?,
                api_key_set: row.get::<_, i64>(4)? != 0,
                enabled: row.get::<_, i64>(5)? != 0,
                is_default: row.get::<_, i64>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn create(&self, master_key: &[u8; 32], req: &CreateProviderRequest) -> anyhow::Result<i64> {
        let uuid = crate::db::ids::new_uuid_blob();
        let vault = VaultRepository::new(self.conn);
        let key_version = vault.get_active_master_key_version()?
            .ok_or_else(|| anyhow::anyhow!("no active vault key"))?;

        let (enc_dek, dek_nonce, ciphertext, ct_nonce) = self.encrypt_api_key(master_key, &req.api_key, key_version)?;

        // If setting as default, unset other defaults first
        if req.is_default.unwrap_or(false) {
            self.conn.execute("UPDATE providers SET is_default = 0", [])?;
        }

        self.conn.execute(
            "INSERT INTO providers (uuid, name, type, base_url, api_key_encrypted, api_key_dek_nonce, api_key_ciphertext, api_key_ciphertext_nonce, api_key_key_version, enabled, is_default)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                uuid, req.name, req.provider_type, req.base_url,
                enc_dek, dek_nonce, ciphertext, ct_nonce, key_version,
                req.enabled.unwrap_or(true) as i64,
                req.is_default.unwrap_or(false) as i64,
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        info!("provider created: id={id} name={}", req.name);
        Ok(id)
    }

    pub fn update(&self, master_key: &[u8; 32], id: i64, req: &UpdateProviderRequest) -> anyhow::Result<bool> {
        let existing = self.get(id)?;
        if existing.is_none() {
            return Ok(false);
        }

        let vault = VaultRepository::new(self.conn);
        let key_version = vault.get_active_master_key_version()?
            .ok_or_else(|| anyhow::anyhow!("no active vault key"))?;

        // Handle API key update
        let (enc_dek, dek_nonce, ciphertext, ct_nonce) = if req.api_key.is_some() {
            self.encrypt_api_key(master_key, &req.api_key, key_version)?
        } else {
            (Vec::new(), Vec::new(), Vec::new(), Vec::new())
        };

        if req.is_default.unwrap_or(false) {
            self.conn.execute("UPDATE providers SET is_default = 0", [])?;
        }

        let mut sets = Vec::new();
        let mut p: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;

        if let Some(ref name) = req.name {
            sets.push(format!("name = ?{idx}"));
            p.push(Box::new(name.clone()));
            idx += 1;
        }
        if let Some(ref url) = req.base_url {
            sets.push(format!("base_url = ?{idx}"));
            p.push(Box::new(url.clone()));
            idx += 1;
        }
        if req.api_key.is_some() {
            sets.push(format!("api_key_encrypted = ?{idx}"));
            p.push(Box::new(enc_dek));
            idx += 1;
            sets.push(format!("api_key_dek_nonce = ?{idx}"));
            p.push(Box::new(dek_nonce));
            idx += 1;
            sets.push(format!("api_key_ciphertext = ?{idx}"));
            p.push(Box::new(ciphertext));
            idx += 1;
            sets.push(format!("api_key_ciphertext_nonce = ?{idx}"));
            p.push(Box::new(ct_nonce));
            idx += 1;
        }
        if let Some(enabled) = req.enabled {
            sets.push(format!("enabled = ?{idx}"));
            p.push(Box::new(enabled as i64));
            idx += 1;
        }
        if let Some(def) = req.is_default {
            sets.push(format!("is_default = ?{idx}"));
            p.push(Box::new(def as i64));
            idx += 1;
        }

        if sets.is_empty() {
            return Ok(true);
        }

        sets.push(format!("updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')"));
        let sql = format!("UPDATE providers SET {} WHERE id = ?{idx}", sets.join(", "));
        p.push(Box::new(id));

        let params_ref: Vec<&dyn rusqlite::types::ToSql> = p.iter().map(|b| b.as_ref()).collect();
        let affected = self.conn.execute(&sql, params_ref.as_slice())?;
        Ok(affected > 0)
    }

    pub fn delete(&self, id: i64) -> anyhow::Result<bool> {
        let affected = self.conn.execute("DELETE FROM providers WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn get_default(&self) -> anyhow::Result<Option<Provider>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, type, base_url,
                    CASE WHEN api_key_ciphertext IS NOT NULL AND length(api_key_ciphertext) > 0 THEN 1 ELSE 0 END,
                    enabled, is_default, created_at, updated_at
             FROM providers WHERE is_default = 1 AND enabled = 1"
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Provider {
                id: row.get(0)?,
                name: row.get(1)?,
                provider_type: row.get(2)?,
                base_url: row.get(3)?,
                api_key_set: row.get::<_, i64>(4)? != 0,
                enabled: row.get::<_, i64>(5)? != 0,
                is_default: row.get::<_, i64>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_api_key(&self, master_key: &[u8; 32], id: i64) -> anyhow::Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT api_key_encrypted, api_key_dek_nonce, api_key_ciphertext, api_key_ciphertext_nonce
             FROM providers WHERE id = ?1"
        )?;
        let mut rows = stmt.query(params![id])?;

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

    // --- Model CRUD ---

    pub fn list_models(&self, provider_id: i64) -> anyhow::Result<Vec<ProviderModel>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, provider_id, model_id, model_type, display_name, context_window, max_output,
                    supports_tools, supports_vision, supports_reasoning, supports_audio, supports_video,
                    input_modalities, output_modalities
             FROM provider_models WHERE provider_id = ?1 ORDER BY model_type, model_id"
        )?;
        let rows = stmt.query_map(params![provider_id], |r| {
            Ok(ProviderModel {
                id: r.get(0)?,
                provider_id: r.get(1)?,
                model_id: r.get(2)?,
                model_type: r.get(3)?,
                display_name: r.get(4)?,
                context_window: r.get(5)?,
                max_output: r.get(6)?,
                supports_tools: r.get::<_, i64>(7)? != 0,
                supports_vision: r.get::<_, i64>(8)? != 0,
                supports_reasoning: r.get::<_, i64>(9)? != 0,
                supports_audio: r.get::<_, i64>(10)? != 0,
                supports_video: r.get::<_, i64>(11)? != 0,
                input_modalities: r.get(12)?,
                output_modalities: r.get(13)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn upsert_model(&self, provider_id: i64, model: &ProviderModel) -> anyhow::Result<i64> {
        self.conn.execute(
            "INSERT INTO provider_models (provider_id, model_id, model_type, display_name, context_window, max_output, supports_tools, supports_vision, supports_reasoning, supports_audio, supports_video, input_modalities, output_modalities)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(provider_id, model_id) DO UPDATE SET
                model_type = excluded.model_type,
                display_name = excluded.display_name,
                context_window = excluded.context_window,
                max_output = excluded.max_output,
                supports_tools = excluded.supports_tools,
                supports_vision = excluded.supports_vision,
                supports_reasoning = excluded.supports_reasoning,
                supports_audio = excluded.supports_audio,
                supports_video = excluded.supports_video,
                input_modalities = excluded.input_modalities,
                output_modalities = excluded.output_modalities",
            params![
                provider_id, model.model_id, model.model_type, model.display_name,
                model.context_window, model.max_output,
                model.supports_tools as i64, model.supports_vision as i64,
                model.supports_reasoning as i64, model.supports_audio as i64, model.supports_video as i64,
                model.input_modalities, model.output_modalities,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn delete_models(&self, provider_id: i64) -> anyhow::Result<usize> {
        let affected = self.conn.execute("DELETE FROM provider_models WHERE provider_id = ?1", params![provider_id])?;
        Ok(affected)
    }

    fn encrypt_api_key(&self, master_key: &[u8; 32], api_key: &Option<String>, key_version: i64) -> anyhow::Result<(Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>)> {
        match api_key {
            Some(key) if !key.is_empty() => {
                let dek = VaultCrypto::generate_dek();
                let enc_dek = VaultCrypto::encrypt_dek(master_key, &dek)?;
                let enc_val = VaultCrypto::encrypt_value(&dek, key.as_bytes())?;
                let _ = key_version; // used in parent for the FK
                Ok((enc_dek.ciphertext, enc_dek.nonce, enc_val.ciphertext, enc_val.nonce))
            }
            _ => Ok((Vec::new(), Vec::new(), Vec::new(), Vec::new()))
        }
    }
}
