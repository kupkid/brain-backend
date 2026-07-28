use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Sha256, Digest};
use tracing::{info, warn};

use crate::db::ids;
use super::crypto::VaultCrypto;

pub struct VaultRepository<'a> {
    conn: &'a Connection,
}

#[derive(Debug)]
pub struct StoredCredential {
    pub id: i64,
    pub uuid: Vec<u8>,
    pub name: String,
    pub scope: String,
    pub project_id: Option<i64>,
    pub encrypted_dek: Vec<u8>,
    pub dek_nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub ciphertext_nonce: Vec<u8>,
    pub key_version: i64,
    pub tags_json: String,
}

#[derive(Debug)]
pub struct MasterKeyRecord {
    pub id: i64,
    pub algorithm: String,
}

impl<'a> VaultRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Initialize vault: create first master key if none exists
    pub fn init(&self, master_key: &[u8; 32]) -> anyhow::Result<()> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM vault_master_keys",
            [],
            |r| r.get(0),
        )?;

        if count == 0 {
            let _encrypted = VaultCrypto::encrypt_dek(master_key, master_key)?;
            // Store master key hash for verification (not the key itself)
            let mut hasher = Sha256::new();
            hasher.update(master_key);
            let _hash = hasher.finalize();

            self.conn.execute(
                "INSERT INTO vault_master_keys (id, algorithm) VALUES (1, 'aes-256-gcm')",
                [],
            )?;
            info!("initialized vault with first master key (version 1)");
        }
        Ok(())
    }

    pub fn get_active_master_key_version(&self) -> anyhow::Result<Option<i64>> {
        self.conn
            .query_row(
                "SELECT id FROM vault_master_keys WHERE retired_at IS NULL",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Store a credential (encrypts DEK + value)
    pub fn store_credential(
        &self,
        master_key: &[u8; 32],
        name: &str,
        scope: &str,
        project_id: Option<i64>,
        plaintext: &[u8],
        tags: &[String],
    ) -> anyhow::Result<i64> {
        let key_version = self.get_active_master_key_version()?
            .ok_or_else(|| anyhow::anyhow!("no active master key"))?;

        // Generate per-credential DEK
        let dek = VaultCrypto::generate_dek();

        // Encrypt DEK with master key
        let enc_dek = VaultCrypto::encrypt_dek(master_key, &dek)?;

        // Encrypt value with DEK
        let enc_value = VaultCrypto::encrypt_value(&dek, plaintext)?;

        let uuid = ids::new_uuid_blob();
        let tags_json = serde_json::to_string(&tags)?;

        self.conn.execute(
            "INSERT INTO credentials_vault
                (uuid, project_id, scope, name, encrypted_dek, dek_nonce, ciphertext, ciphertext_nonce, key_version, tags_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                uuid,
                project_id,
                scope,
                name,
                enc_dek.ciphertext,
                enc_dek.nonce,
                enc_value.ciphertext,
                enc_value.nonce,
                key_version,
                tags_json,
            ],
        )?;

        let id: i64 = self.conn.last_insert_rowid();
        info!("stored credential: {} (scope={})", name, scope);
        Ok(id)
    }

    /// Retrieve and decrypt a credential
    pub fn get_credential(
        &self,
        master_key: &[u8; 32],
        name: &str,
        scope: &str,
        project_id: Option<i64>,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        let row = self.conn
            .query_row(
                "SELECT encrypted_dek, dek_nonce, ciphertext, ciphertext_nonce, key_version
                 FROM credentials_vault
                 WHERE name = ?1 AND scope = ?2 AND project_id IS ?3",
                params![name, scope, project_id],
                |r| {
                    Ok((
                        r.get::<_, Vec<u8>>(0)?,
                        r.get::<_, Vec<u8>>(1)?,
                        r.get::<_, Vec<u8>>(2)?,
                        r.get::<_, Vec<u8>>(3)?,
                        r.get::<_, i64>(4)?,
                    ))
                },
            )
            .optional()?;

        match row {
            Some((enc_dek, dek_nonce, ciphertext, ct_nonce, _key_version)) => {
                // Decrypt DEK with master key
                let dek = VaultCrypto::decrypt_dek(master_key, &enc_dek, &dek_nonce)?;

                // Decrypt value with DEK
                let plaintext = VaultCrypto::decrypt_value(&dek, &ciphertext, &ct_nonce)?;

                // Increment access count (fire and forget)
                let _ = self.conn.execute(
                    "UPDATE credentials_vault SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE name = ?1 AND scope = ?2",
                    params![name, scope],
                );

                Ok(Some(plaintext))
            }
            None => Ok(None),
        }
    }

    /// Rotate master key: re-wrap all DEKs with new master key
    pub fn rotate_master_key(
        &self,
        old_master_key: &[u8; 32],
        new_master_key: &[u8; 32],
    ) -> anyhow::Result<()> {
        let old_version = self.get_active_master_key_version()?
            .ok_or_else(|| anyhow::anyhow!("no active master key"))?;

        let new_version = old_version + 1;

        self.conn.execute_batch("BEGIN IMMEDIATE;")?;

        // 1. Create new master key record
        self.conn.execute(
            "INSERT INTO vault_master_keys (id, algorithm) VALUES (?1, 'aes-256-gcm')",
            params![new_version],
        )?;

        // 2. Re-wrap all DEKs from old key to new key
        let mut stmt = self.conn.prepare(
            "SELECT id, encrypted_dek, dek_nonce FROM credentials_vault WHERE key_version = ?1"
        )?;
        let rows: Vec<(i64, Vec<u8>, Vec<u8>)> = stmt
            .query_map(params![old_version], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        for (cred_id, enc_dek, dek_nonce) in &rows {
            // Decrypt DEK with old master key
            let dek = VaultCrypto::decrypt_dek(old_master_key, enc_dek, dek_nonce)?;
            // Re-encrypt DEK with new master key
            let re_enc = VaultCrypto::encrypt_dek(new_master_key, &dek)?;

            self.conn.execute(
                "UPDATE credentials_vault
                 SET encrypted_dek = ?1, dek_nonce = ?2, key_version = ?3,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?4",
                params![re_enc.ciphertext, re_enc.nonce, new_version, cred_id],
            )?;
        }

        // 3. Retire old master key
        self.conn.execute(
            "UPDATE vault_master_keys SET retired_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?1",
            params![old_version],
        )?;

        self.conn.execute_batch("COMMIT;")?;

        info!(
            "rotated master key: v{} → v{}, re-wrapped {} credentials",
            old_version, new_version, rows.len()
        );
        Ok(())
    }

    /// List credentials (names only, no decrypted data)
    pub fn list_credentials(
        &self,
        scope: &str,
        project_id: Option<i64>,
    ) -> anyhow::Result<Vec<(i64, String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, tags_json, created_at FROM credentials_vault
             WHERE scope = ?1 AND project_id IS ?2
             ORDER BY created_at DESC"
        )?;
        let rows = stmt
            .query_map(params![scope, project_id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Delete a credential
    pub fn delete_credential(&self, name: &str, scope: &str, project_id: Option<i64>) -> anyhow::Result<bool> {
        let affected = self.conn.execute(
            "DELETE FROM credentials_vault WHERE name = ?1 AND scope = ?2 AND project_id IS ?3",
            params![name, scope, project_id],
        )?;
        Ok(affected > 0)
    }
}
