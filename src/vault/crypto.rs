use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },
    #[error("invalid nonce length: expected {expected}, got {actual}")]
    InvalidNonceLength { expected: usize, actual: usize },
}

const NONCE_LEN: usize = 12;
const DEK_LEN: usize = 32;

#[derive(Debug, Clone)]
pub struct EncryptedPayload {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

pub struct VaultCrypto;

impl VaultCrypto {
    pub fn generate_dek() -> Vec<u8> {
        let mut key = vec![0u8; DEK_LEN];
        OsRng.fill_bytes(&mut key);
        key
    }

    pub fn generate_nonce() -> Vec<u8> {
        let mut nonce = vec![0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce);
        nonce
    }

    /// Encrypt DEK with master key → encrypted_dek
    pub fn encrypt_dek(master_key: &[u8; 32], dek: &[u8]) -> Result<EncryptedPayload, CryptoError> {
        let cipher = Aes256Gcm::new_from_slice(master_key)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
        let nonce_bytes = Self::generate_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, dek)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
        Ok(EncryptedPayload {
            ciphertext,
            nonce: nonce_bytes,
        })
    }

    /// Decrypt DEK with master key
    pub fn decrypt_dek(master_key: &[u8; 32], encrypted_dek: &[u8], nonce: &[u8]) -> Result<Vec<u8>, CryptoError> {
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
    fn roundtrip_envelope_encryption() {
        let master_key = [0x42u8; 32];
        let dek = VaultCrypto::generate_dek();
        let plaintext = b"super secret api key: sk-abc123";

        // Encrypt DEK with master key
        let encrypted_dek = VaultCrypto::encrypt_dek(&master_key, &dek).unwrap();
        // Encrypt value with DEK
        let encrypted_value = VaultCrypto::encrypt_value(&dek, plaintext).unwrap();

        // Decrypt DEK with master key
        let decrypted_dek = VaultCrypto::decrypt_dek(
            &master_key,
            &encrypted_dek.ciphertext,
            &encrypted_dek.nonce,
        )
        .unwrap();
        assert_eq!(decrypted_dek, dek);

        // Decrypt value with DEK
        let decrypted_value = VaultCrypto::decrypt_value(
            &decrypted_dek,
            &encrypted_value.ciphertext,
            &encrypted_value.nonce,
        )
        .unwrap();
        assert_eq!(decrypted_value, plaintext);
    }

    #[test]
    fn key_rotation() {
        let old_master_key = [0x11u8; 32];
        let new_master_key = [0x22u8; 32];
        let dek = VaultCrypto::generate_dek();
        let plaintext = b"rotate me";

        // Encrypt with old master key
        let encrypted_dek = VaultCrypto::encrypt_dek(&old_master_key, &dek).unwrap();

        // Rotate: decrypt DEK with old, re-encrypt with new
        let decrypted_dek = VaultCrypto::decrypt_dek(
            &old_master_key,
            &encrypted_dek.ciphertext,
            &encrypted_dek.nonce,
        )
        .unwrap();
        let re_encrypted_dek = VaultCrypto::encrypt_dek(&new_master_key, &decrypted_dek).unwrap();

        // Decrypt with new master key
        let final_dek = VaultCrypto::decrypt_dek(
            &new_master_key,
            &re_encrypted_dek.ciphertext,
            &re_encrypted_dek.nonce,
        )
        .unwrap();
        assert_eq!(final_dek, dek);

        // Value still decrypts with the same DEK
        let encrypted_value = VaultCrypto::encrypt_value(&dek, plaintext).unwrap();
        let decrypted_value = VaultCrypto::decrypt_value(
            &final_dek,
            &encrypted_value.ciphertext,
            &encrypted_value.nonce,
        )
        .unwrap();
        assert_eq!(decrypted_value, plaintext);
    }
}
