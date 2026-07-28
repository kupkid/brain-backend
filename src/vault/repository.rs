use rusqlite::{params, Connection, OptionalExtension};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tracing::info;

use crate::db::ids;
use super::crypto::{VaultCrypto, Argon2Params, MasterKeyMaterial, CryptoError};

/// Global lock to ensure only one Argon2id operation runs at a time.
/// Prevents RAM exhaustion from concurrent KDF computations.
static ARGON2_LOCK: Mutex<()> = Mutex::new(());
/// Track if vault is currently being initialized (for diagnostic purposes).
static VAULT_INITIALIZING: AtomicBool = AtomicBool::new(false);

pub struct VaultRepository<'a> {
    conn: &'a Connection,
}

#[derive(Debug)]
pub struct CredentialMetadata {
    pub id: i64,
    pub name: String,
    pub scope: String,
    #[allow(dead_code)]
    pub project_id: Option<i64>,
    pub key_version: i64,
    pub tags_json: String,
    pub created_at: String,
    #[allow(dead_code)]
    pub updated_at: String,
}

#[derive(Debug)]
pub struct MasterKeyRecord {
    pub id: i64,
    #[allow(dead_code)]
    pub algorithm: String,
    pub salt: Vec<u8>,
    pub params: Argon2Params,
    pub key_hash: Vec<u8>,
    #[allow(dead_code)]
    pub created_at: String,
    #[allow(dead_code)]
    pub retired_at: Option<String>,
}

impl<'a> VaultRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Initialize vault with a new passphrase (first run).
    /// Generates salt, derives key via Argon2id, stores salt/params/hash.
    /// Serialized via ARGON2_LOCK to prevent concurrent KDF operations.
    pub fn init(&self, passphrase: &[u8]) -> Result<MasterKeyMaterial, anyhow::Error> {
        let _guard = ARGON2_LOCK.lock()
            .map_err(|_| anyhow::anyhow!("argon2 lock poisoned"))?;

        if VAULT_INITIALIZING.swap(true, Ordering::SeqCst) {
            return Err(anyhow::anyhow!("vault initialization already in progress"));
        }
        let _flag_guard = scopeguard::guard((), |_| {
            VAULT_INITIALIZING.store(false, Ordering::SeqCst);
        });

        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM vault_master_keys",
            [],
            |r| r.get(0),
        )?;

        anyhow::ensure!(count == 0, "vault already initialized — use unlock() instead");

        let salt = VaultCrypto::generate_salt();
        let params = Argon2Params::default();

        let material = VaultCrypto::derive_master_key(passphrase, &salt, &params)?;

        self.conn.execute(
            "INSERT INTO vault_master_keys (id, algorithm, salt, params_json, key_hash)
             VALUES (1, 'aes-256-gcm', ?1, ?2, ?3)",
            params![salt, params.to_json(), material.key_hash],
        )?;

        info!("vault initialized with Argon2id");
        Ok(material)
    }

    /// Unlock vault with passphrase (subsequent runs).
    /// Reads stored salt/params, derives key, verifies hash.
    /// Serialized via ARGON2_LOCK to prevent concurrent KDF operations.
    pub fn unlock(&self, passphrase: &[u8]) -> Result<MasterKeyMaterial, anyhow::Error> {
        let _guard = ARGON2_LOCK.lock()
            .map_err(|_| anyhow::anyhow!("argon2 lock poisoned"))?;

        let record = self.conn
            .query_row(
                "SELECT id, salt, params_json, key_hash FROM vault_master_keys
                 WHERE retired_at IS NULL ORDER BY id DESC LIMIT 1",
                [],
                |r| {
                    Ok(MasterKeyRecord {
                        id: r.get(0)?,
                        algorithm: "aes-256-gcm".to_string(),
                        salt: r.get(1)?,
                        params: Argon2Params::from_json(&r.get::<_, String>(2)?),
                        key_hash: r.get(3)?,
                        created_at: String::new(),
                        retired_at: None,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| anyhow::anyhow!("no active vault master key found"))?;

        let material = VaultCrypto::verify_passphrase(
            passphrase,
            &record.salt,
            &record.params,
            &record.key_hash,
        ).map_err(|e| match e {
            CryptoError::PassphraseVerificationFailed => {
                anyhow::anyhow!("invalid passphrase")
            }
            CryptoError::KdfParamsOutOfRange => {
                anyhow::anyhow!("corrupted KDF parameters in database")
            }
            other => anyhow::anyhow!("key derivation failed: {}", other),
        })?;

        info!("vault unlocked (master key version {})", record.id);

        Ok(MasterKeyMaterial {
            key: material,
            salt: record.salt,
            params: record.params,
            key_hash: record.key_hash,
        })
    }

    /// Get active master key version
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

    /// Store a credential (encrypts DEK + value).
    /// Never logs or returns the plaintext value.
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

        let dek = VaultCrypto::generate_dek();
        let enc_dek = VaultCrypto::encrypt_dek(master_key, &dek)?;
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
        // Log credential name for audit, never the value
        info!("stored credential: name={} scope={}", name, scope);
        Ok(id)
    }

    /// Get credential metadata WITHOUT decrypting the value.
    pub fn get_credential_metadata(
        &self,
        name: &str,
        scope: &str,
        project_id: Option<i64>,
    ) -> anyhow::Result<Option<CredentialMetadata>> {
        self.conn
            .query_row(
                "SELECT id, name, scope, project_id, key_version, tags_json, created_at, updated_at
                 FROM credentials_vault
                 WHERE name = ?1 AND scope = ?2 AND project_id IS ?3",
                params![name, scope, project_id],
                |r| {
                    Ok(CredentialMetadata {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        scope: r.get(2)?,
                        project_id: r.get(3)?,
                        key_version: r.get(4)?,
                        tags_json: r.get(5)?,
                        created_at: r.get(6)?,
                        updated_at: r.get(7)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Retrieve and decrypt a credential value.
    /// FOR INTERNAL USE ONLY — never expose via API.
    /// Returns (plaintext, key_version) tuple.
    #[allow(dead_code)] // Used by future provider/agent runtime
    pub fn decrypt_credential(
        &self,
        master_key: &[u8; 32],
        name: &str,
        scope: &str,
        project_id: Option<i64>,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        let row = self.conn
            .query_row(
                "SELECT encrypted_dek, dek_nonce, ciphertext, ciphertext_nonce
                 FROM credentials_vault
                 WHERE name = ?1 AND scope = ?2 AND project_id IS ?3",
                params![name, scope, project_id],
                |r| {
                    Ok((
                        r.get::<_, Vec<u8>>(0)?,
                        r.get::<_, Vec<u8>>(1)?,
                        r.get::<_, Vec<u8>>(2)?,
                        r.get::<_, Vec<u8>>(3)?,
                    ))
                },
            )
            .optional()?;

        match row {
            Some((enc_dek, dek_nonce, ciphertext, ct_nonce)) => {
                let dek = VaultCrypto::decrypt_dek(master_key, &enc_dek, &dek_nonce)?;
                let plaintext = VaultCrypto::decrypt_value(&dek, &ciphertext, &ct_nonce)?;

                // Update access timestamp (fire and forget)
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

    /// Rotate master key: change passphrase and re-wrap all DEKs.
    /// Atomic operation in a single SQLite transaction.
    /// On failure, rollback leaves all credentials accessible with old passphrase.
    #[allow(dead_code)] // Used by future rotation API endpoint
    pub fn rotate_master_key(
        &self,
        old_master_key: &[u8; 32],
        new_passphrase: &[u8],
    ) -> anyhow::Result<()> {
        let _guard = ARGON2_LOCK.lock()
            .map_err(|_| anyhow::anyhow!("argon2 lock poisoned"))?;

        let old_version = self.get_active_master_key_version()?
            .ok_or_else(|| anyhow::anyhow!("no active master key"))?;

        let new_salt = VaultCrypto::generate_salt();
        let new_params = Argon2Params::default();
        let new_material = VaultCrypto::derive_master_key(new_passphrase, &new_salt, &new_params)?;

        let new_version = old_version + 1;

        // Transactional rotation — all or nothing
        let tx = self.conn.unchecked_transaction()?;

        // 1. Create new master key record (not yet committed)
        tx.execute(
            "INSERT INTO vault_master_keys (id, algorithm, salt, params_json, key_hash)
             VALUES (?1, 'aes-256-gcm', ?2, ?3, ?4)",
            params![new_version, new_salt, new_params.to_json(), new_material.key_hash],
        )?;

        // 2. Re-wrap all DEKs from old key to new key
        let rows: Vec<(i64, Vec<u8>, Vec<u8>)> = {
            let mut stmt = tx.prepare(
                "SELECT id, encrypted_dek, dek_nonce FROM credentials_vault WHERE key_version = ?1"
            )?;
            stmt.query_map(params![old_version], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?
            .collect::<Result<Vec<_>, _>>()?
        };

        for (cred_id, enc_dek, dek_nonce) in &rows {
            let dek = VaultCrypto::decrypt_dek(old_master_key, enc_dek, dek_nonce)?;
            let re_enc = VaultCrypto::encrypt_dek(&new_material.key, &dek)?;

            tx.execute(
                "UPDATE credentials_vault
                 SET encrypted_dek = ?1, dek_nonce = ?2, key_version = ?3,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?4",
                params![re_enc.ciphertext, re_enc.nonce, new_version, cred_id],
            )?;
        }

        // 3. Retire old master key — only after all DEKs successfully re-wrapped
        tx.execute(
            "UPDATE vault_master_keys SET retired_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?1",
            params![old_version],
        )?;

        tx.commit()?;

        info!(
            "rotated master key: v{} -> v{}, re-wrapped {} credentials",
            old_version, new_version, rows.len()
        );
        Ok(())
    }

    /// List credentials (metadata only, no decrypted data)
    pub fn list_credentials(
        &self,
        scope: &str,
        project_id: Option<i64>,
    ) -> anyhow::Result<Vec<CredentialMetadata>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, scope, project_id, key_version, tags_json, created_at, updated_at
             FROM credentials_vault
             WHERE scope = ?1 AND project_id IS ?2
             ORDER BY created_at DESC"
        )?;
        let rows = stmt
            .query_map(params![scope, project_id], |r| {
                Ok(CredentialMetadata {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    scope: r.get(2)?,
                    project_id: r.get(3)?,
                    key_version: r.get(4)?,
                    tags_json: r.get(5)?,
                    created_at: r.get(6)?,
                    updated_at: r.get(7)?,
                })
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
