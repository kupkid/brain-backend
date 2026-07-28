use aes_gcm::{
    aead::Aead,
    Aes256Gcm, KeyInit, Nonce,
};
use argon2::Argon2;
use rand::RngCore;
use sha2::{Sha256, Digest};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("key derivation failed: {0}")]
    KeyDerivationFailed(String),
    #[error("passphrase verification failed")]
    PassphraseVerificationFailed,
    #[error("invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },
}

const NONCE_LEN: usize = 12;
const DEK_LEN: usize = 32;
const SALT_LEN: usize = 16;
const KEY_LEN: usize = 32;

/// Argon2id parameters for master key derivation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Argon2Params {
    pub memory_cost: u32,    // in KiB (65536 = 64 MB)
    pub time_cost: u32,      // iterations
    pub parallelism: u32,    // threads
}

impl Default for Argon2Params {
    fn default() -> Self {
        Self {
            memory_cost: 65536,  // 64 MB
            time_cost: 3,
            parallelism: 4,
        }
    }
}

impl Argon2Params {
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"memory_cost":{},"time_cost":{},"parallelism":{}}}"#,
            self.memory_cost, self.time_cost, self.parallelism
        )
    }

    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub struct EncryptedPayload {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct MasterKeyMaterial {
    pub key: [u8; KEY_LEN],
    pub salt: Vec<u8>,
    pub params: Argon2Params,
    pub key_hash: Vec<u8>,
}

pub struct VaultCrypto;

impl VaultCrypto {
    /// Generate random salt for Argon2id
    pub fn generate_salt() -> Vec<u8> {
        let mut salt = vec![0u8; SALT_LEN];
        rand::thread_rng().fill_bytes(&mut salt);
        salt
    }

    /// Generate random DEK (Data Encryption Key)
    pub fn generate_dek() -> Vec<u8> {
        let mut key = vec![0u8; DEK_LEN];
        rand::thread_rng().fill_bytes(&mut key);
        key
    }

    /// Generate random nonce for AES-256-GCM
    pub fn generate_nonce() -> Vec<u8> {
        let mut nonce = vec![0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce);
        nonce
    }

    /// Derive master key from passphrase using Argon2id
    pub fn derive_master_key(
        passphrase: &[u8],
        salt: &[u8],
        params: &Argon2Params,
    ) -> Result<MasterKeyMaterial, CryptoError> {
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(
                params.memory_cost,
                params.time_cost,
                params.parallelism,
                Some(KEY_LEN),
            )
            .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?,
        );

        let mut key = [0u8; KEY_LEN];
        argon2
            .hash_password_into(passphrase, salt, &mut key)
            .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;

        // Hash the derived key for verification
        let key_hash = Self::hash_key(&key);

        Ok(MasterKeyMaterial {
            key,
            salt: salt.to_vec(),
            params: params.clone(),
            key_hash,
        })
    }

    /// Verify passphrase against stored hash
    pub fn verify_passphrase(
        passphrase: &[u8],
        salt: &[u8],
        params: &Argon2Params,
        expected_hash: &[u8],
    ) -> Result<[u8; KEY_LEN], CryptoError> {
        let material = Self::derive_master_key(passphrase, salt, params)?;

        if material.key_hash != expected_hash {
            return Err(CryptoError::PassphraseVerificationFailed);
        }

        Ok(material.key)
    }

    /// SHA-256 hash of the key for verification
    fn hash_key(key: &[u8; KEY_LEN]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.finalize().to_vec()
    }

    /// Encrypt DEK with master key → (encrypted_dek, nonce)
    pub fn encrypt_dek(master_key: &[u8; KEY_LEN], dek: &[u8]) -> Result<EncryptedPayload, CryptoError> {
        let cipher = Aes256Gcm::new_from_slice(master_key)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
        let nonce_bytes = Self::generate_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, dek.as_ref())
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
        Ok(EncryptedPayload {
            ciphertext,
            nonce: nonce_bytes,
        })
    }

    /// Decrypt DEK with master key
    pub fn decrypt_dek(master_key: &[u8; KEY_LEN], encrypted_dek: &[u8], nonce: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let cipher = Aes256Gcm::new_from_slice(master_key)
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;
        let nonce = Nonce::from_slice(nonce);
        cipher
            .decrypt(nonce, encrypted_dek)
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))
    }

    /// Encrypt value with DEK
    pub fn encrypt_value(dek: &[u8], plaintext: &[u8]) -> Result<EncryptedPayload, CryptoError> {
        let cipher = Aes256Gcm::new_from_slice(dek)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
        let nonce_bytes = Self::generate_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
        Ok(EncryptedPayload {
            ciphertext,
            nonce: nonce_bytes,
        })
    }

    /// Decrypt value with DEK
    pub fn decrypt_value(dek: &[u8], ciphertext: &[u8], nonce: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let cipher = Aes256Gcm::new_from_slice(dek)
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;
        let nonce = Nonce::from_slice(nonce);
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argon2id_derivation_deterministic() {
        let passphrase = b"my secure passphrase";
        let salt = vec![0x42u8; 16];
        let params = Argon2Params::default();

        let k1 = VaultCrypto::derive_master_key(passphrase, &salt, &params).unwrap();
        let k2 = VaultCrypto::derive_master_key(passphrase, &salt, &params).unwrap();
        assert_eq!(k1.key, k2.key);
        assert_eq!(k1.key_hash, k2.key_hash);
    }

    #[test]
    fn argon2id_different_salt_different_key() {
        let passphrase = b"my secure passphrase";
        let params = Argon2Params::default();

        let k1 = VaultCrypto::derive_master_key(passphrase, &[0x01u8; 16], &params).unwrap();
        let k2 = VaultCrypto::derive_master_key(passphrase, &[0x02u8; 16], &params).unwrap();
        assert_ne!(k1.key, k2.key);
    }

    #[test]
    fn verify_passphrase_roundtrip() {
        let passphrase = b"test passphrase 123";
        let salt = VaultCrypto::generate_salt();
        let params = Argon2Params::default();

        let material = VaultCrypto::derive_master_key(passphrase, &salt, &params).unwrap();
        let key = VaultCrypto::verify_passphrase(passphrase, &salt, &params, &material.key_hash).unwrap();
        assert_eq!(key, material.key);
    }

    #[test]
    fn verify_wrong_passphrase_fails() {
        let salt = VaultCrypto::generate_salt();
        let params = Argon2Params::default();

        let material = VaultCrypto::derive_master_key(b"correct", &salt, &params).unwrap();
        let result = VaultCrypto::verify_passphrase(b"wrong", &salt, &params, &material.key_hash);
        assert!(result.is_err());
    }

    #[test]
    fn envelope_encryption_roundtrip() {
        let passphrase = b"master passphrase";
        let salt = VaultCrypto::generate_salt();
        let params = Argon2Params::default();

        let material = VaultCrypto::derive_master_key(passphrase, &salt, &params).unwrap();
        let dek = VaultCrypto::generate_dek();
        let plaintext = b"super secret api key: sk-abc123";

        // Encrypt DEK with master key
        let enc_dek = VaultCrypto::encrypt_dek(&material.key, &dek).unwrap();
        // Encrypt value with DEK
        let enc_value = VaultCrypto::encrypt_value(&dek, plaintext).unwrap();

        // Decrypt DEK with master key
        let dec_dek = VaultCrypto::decrypt_dek(&material.key, &enc_dek.ciphertext, &enc_dek.nonce).unwrap();
        assert_eq!(dec_dek, dek);

        // Decrypt value with DEK
        let dec_value = VaultCrypto::decrypt_value(&dec_dek, &enc_value.ciphertext, &enc_value.nonce).unwrap();
        assert_eq!(dec_value, plaintext);
    }

    #[test]
    fn key_rotation_roundtrip() {
        let salt = VaultCrypto::generate_salt();
        let params = Argon2Params::default();

        let old_material = VaultCrypto::derive_master_key(b"old passphrase", &salt, &params).unwrap();
        let new_material = VaultCrypto::derive_master_key(b"new passphrase", &salt, &params).unwrap();

        let dek = VaultCrypto::generate_dek();
        let plaintext = b"rotate me";

        // Encrypt DEK with old master key
        let enc_dek = VaultCrypto::encrypt_dek(&old_material.key, &dek).unwrap();

        // Rotate: decrypt DEK with old, re-encrypt with new
        let dec_dek = VaultCrypto::decrypt_dek(&old_material.key, &enc_dek.ciphertext, &enc_dek.nonce).unwrap();
        let re_enc_dek = VaultCrypto::encrypt_dek(&new_material.key, &dec_dek).unwrap();

        // Decrypt with new master key
        let final_dek = VaultCrypto::decrypt_dek(&new_material.key, &re_enc_dek.ciphertext, &re_enc_dek.nonce).unwrap();
        assert_eq!(final_dek, dek);

        // Value still decrypts with the same DEK
        let enc_value = VaultCrypto::encrypt_value(&dek, plaintext).unwrap();
        let dec_value = VaultCrypto::decrypt_value(&final_dek, &enc_value.ciphertext, &enc_value.nonce).unwrap();
        assert_eq!(dec_value, plaintext);
    }
}
