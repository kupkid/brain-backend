use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use argon2::Argon2;
use rand::RngCore;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use thiserror::Error;
use zeroize::Zeroize;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("encryption failed")]
    EncryptionFailed,
    #[error("decryption failed — authentication tag mismatch")]
    DecryptionFailed,
    #[error("key derivation failed: {0}")]
    KeyDerivationFailed(String),
    #[error("invalid passphrase")]
    PassphraseVerificationFailed,
    #[error("invalid key length")]
    #[allow(dead_code)]
    InvalidKeyLength { expected: usize, actual: usize },
    #[error("KDF parameters out of allowed range")]
    KdfParamsOutOfRange,
}

const NONCE_LEN: usize = 12;
const DEK_LEN: usize = 32;
const SALT_LEN: usize = 16;
const KEY_LEN: usize = 32;

/// Maximum allowed memory_cost for Argon2id (256 MiB).
/// Prevents corrupted DB from requesting gigabytes of RAM.
const MAX_MEMORY_COST: u32 = 262144;
/// Maximum allowed time_cost (iterations).
const MAX_TIME_COST: u32 = 16;
/// Maximum allowed parallelism.
const MAX_PARALLELISM: u32 = 16;

/// Argon2id parameters for master key derivation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Argon2Params {
    pub memory_cost: u32, // in KiB (65536 = 64 MB)
    pub time_cost: u32,   // iterations
    pub parallelism: u32, // threads
}

impl Default for Argon2Params {
    fn default() -> Self {
        Self {
            memory_cost: 65536, // 64 MB
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

    /// Validate parameters are within safe bounds before allocating memory.
    pub fn validate(&self) -> Result<(), CryptoError> {
        if self.memory_cost == 0 || self.memory_cost > MAX_MEMORY_COST {
            return Err(CryptoError::KdfParamsOutOfRange);
        }
        if self.time_cost == 0 || self.time_cost > MAX_TIME_COST {
            return Err(CryptoError::KdfParamsOutOfRange);
        }
        if self.parallelism == 0 || self.parallelism > MAX_PARALLELISM {
            return Err(CryptoError::KdfParamsOutOfRange);
        }
        // Argon2 requires memory >= 8 * parallelism
        if self.memory_cost < 8 * self.parallelism {
            return Err(CryptoError::KdfParamsOutOfRange);
        }
        Ok(())
    }

    /// Test-only parameters: 1 MiB, 1 iteration, 1 thread. Fast but NOT for production.
    #[cfg(test)]
    pub fn test_fast() -> Self {
        Self {
            memory_cost: 1024, // 1 MiB
            time_cost: 1,
            parallelism: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EncryptedPayload {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

#[derive(Debug)]
pub struct MasterKeyMaterial {
    pub key: [u8; KEY_LEN],
    pub salt: Vec<u8>,
    #[allow(dead_code)] // params used during init, stored separately in DB
    pub params: Argon2Params,
    pub key_hash: Vec<u8>,
}

impl Drop for MasterKeyMaterial {
    fn drop(&mut self) {
        self.key.zeroize();
        self.salt.zeroize();
        self.key_hash.zeroize();
    }
}

pub struct VaultCrypto;

impl VaultCrypto {
    /// Generate random salt for Argon2id (CSPRNG via OsRng inside rand::thread_rng)
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

    /// Derive master key from passphrase using Argon2id.
    /// Validates parameters before allocation.
    pub fn derive_master_key(
        passphrase: &[u8],
        salt: &[u8],
        params: &Argon2Params,
    ) -> Result<MasterKeyMaterial, CryptoError> {
        params.validate()?;

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

        // Hash the derived key for fast verification (not a security boundary)
        let key_hash = Self::hash_key(&key);

        Ok(MasterKeyMaterial {
            key,
            salt: salt.to_vec(),
            params: params.clone(),
            key_hash,
        })
    }

    /// Verify passphrase against stored hash using constant-time comparison.
    /// Returns the derived master key on success.
    pub fn verify_passphrase(
        passphrase: &[u8],
        salt: &[u8],
        params: &Argon2Params,
        expected_hash: &[u8],
    ) -> Result<[u8; KEY_LEN], CryptoError> {
        let material = Self::derive_master_key(passphrase, salt, params)?;

        // Constant-time comparison to prevent timing side-channel
        if material.key_hash.ct_eq(expected_hash).into() {
            Ok(material.key)
        } else {
            Err(CryptoError::PassphraseVerificationFailed)
        }
    }

    /// SHA-256 hash of the key for verification purposes.
    /// This is not a security boundary — it's a fast check that the correct key was derived.
    /// The actual security comes from AES-256-GCM authentication tags on encrypted data.
    fn hash_key(key: &[u8; KEY_LEN]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.finalize().to_vec()
    }

    /// Encrypt DEK with master key → (encrypted_dek, nonce)
    pub fn encrypt_dek(
        master_key: &[u8; KEY_LEN],
        dek: &[u8],
    ) -> Result<EncryptedPayload, CryptoError> {
        let cipher =
            Aes256Gcm::new_from_slice(master_key).map_err(|_| CryptoError::EncryptionFailed)?;
        let nonce_bytes = Self::generate_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, dek.as_ref())
            .map_err(|_| CryptoError::EncryptionFailed)?;
        Ok(EncryptedPayload {
            ciphertext,
            nonce: nonce_bytes,
        })
    }

    /// Decrypt DEK with master key. Fails on wrong key or tampered data.
    pub fn decrypt_dek(
        master_key: &[u8; KEY_LEN],
        encrypted_dek: &[u8],
        nonce: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let cipher =
            Aes256Gcm::new_from_slice(master_key).map_err(|_| CryptoError::DecryptionFailed)?;
        let nonce = Nonce::from_slice(nonce);
        cipher
            .decrypt(nonce, encrypted_dek)
            .map_err(|_| CryptoError::DecryptionFailed)
    }

    /// Encrypt value with DEK
    pub fn encrypt_value(dek: &[u8], plaintext: &[u8]) -> Result<EncryptedPayload, CryptoError> {
        let cipher = Aes256Gcm::new_from_slice(dek).map_err(|_| CryptoError::EncryptionFailed)?;
        let nonce_bytes = Self::generate_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::EncryptionFailed)?;
        Ok(EncryptedPayload {
            ciphertext,
            nonce: nonce_bytes,
        })
    }

    /// Decrypt value with DEK. Fails on wrong key or tampered data.
    pub fn decrypt_value(
        dek: &[u8],
        ciphertext: &[u8],
        nonce: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let cipher = Aes256Gcm::new_from_slice(dek).map_err(|_| CryptoError::DecryptionFailed)?;
        let nonce = Nonce::from_slice(nonce);
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argon2id_derivation_deterministic() {
        let passphrase = b"my secure passphrase";
        let salt = vec![0x42u8; 16];
        let params = Argon2Params::test_fast();

        let k1 = VaultCrypto::derive_master_key(passphrase, &salt, &params).unwrap();
        let k2 = VaultCrypto::derive_master_key(passphrase, &salt, &params).unwrap();
        assert_eq!(k1.key, k2.key);
        assert_eq!(k1.key_hash, k2.key_hash);
    }

    #[test]
    fn argon2id_different_salt_different_key() {
        let passphrase = b"my secure passphrase";
        let params = Argon2Params::test_fast();

        let k1 = VaultCrypto::derive_master_key(passphrase, &[0x01u8; 16], &params).unwrap();
        let k2 = VaultCrypto::derive_master_key(passphrase, &[0x02u8; 16], &params).unwrap();
        assert_ne!(k1.key, k2.key);
    }

    #[test]
    fn verify_passphrase_roundtrip() {
        let passphrase = b"test passphrase 123";
        let salt = VaultCrypto::generate_salt();
        let params = Argon2Params::test_fast();

        let material = VaultCrypto::derive_master_key(passphrase, &salt, &params).unwrap();
        let key =
            VaultCrypto::verify_passphrase(passphrase, &salt, &params, &material.key_hash).unwrap();
        assert_eq!(key, material.key);
    }

    #[test]
    fn verify_wrong_passphrase_fails() {
        let salt = VaultCrypto::generate_salt();
        let params = Argon2Params::test_fast();

        let material = VaultCrypto::derive_master_key(b"correct", &salt, &params).unwrap();
        let result = VaultCrypto::verify_passphrase(b"wrong", &salt, &params, &material.key_hash);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(CryptoError::PassphraseVerificationFailed)
        ));
    }

    #[test]
    fn envelope_encryption_roundtrip() {
        let dek = VaultCrypto::generate_dek();
        let plaintext = b"super secret api key: sk-abc123";

        let enc_value = VaultCrypto::encrypt_value(&dek, plaintext).unwrap();
        let dec_value =
            VaultCrypto::decrypt_value(&dek, &enc_value.ciphertext, &enc_value.nonce).unwrap();
        assert_eq!(dec_value, plaintext);
    }

    #[test]
    fn key_rotation_roundtrip() {
        let old_key = [0x11u8; 32];
        let new_key = [0x22u8; 32];
        let dek = VaultCrypto::generate_dek();
        let plaintext = b"rotate me";

        let enc_dek = VaultCrypto::encrypt_dek(&old_key, &dek).unwrap();
        let dec_dek =
            VaultCrypto::decrypt_dek(&old_key, &enc_dek.ciphertext, &enc_dek.nonce).unwrap();
        let re_enc_dek = VaultCrypto::encrypt_dek(&new_key, &dec_dek).unwrap();
        let final_dek =
            VaultCrypto::decrypt_dek(&new_key, &re_enc_dek.ciphertext, &re_enc_dek.nonce).unwrap();
        assert_eq!(final_dek, dek);

        let enc_value = VaultCrypto::encrypt_value(&dek, plaintext).unwrap();
        let dec_value =
            VaultCrypto::decrypt_value(&final_dek, &enc_value.ciphertext, &enc_value.nonce)
                .unwrap();
        assert_eq!(dec_value, plaintext);
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let key = [0x42u8; 32];
        let plaintext = b"authenticated data";

        let enc = VaultCrypto::encrypt_value(&key, plaintext).unwrap();
        let mut tampered = enc.ciphertext.clone();
        tampered[0] ^= 0xff;

        let result = VaultCrypto::decrypt_value(&key, &tampered, &enc.nonce);
        assert!(result.is_err());
    }

    #[test]
    fn tampered_nonce_fails() {
        let key = [0x42u8; 32];
        let plaintext = b"authenticated data";

        let enc = VaultCrypto::encrypt_value(&key, plaintext).unwrap();
        let mut tampered = enc.nonce.clone();
        tampered[0] ^= 0xff;

        let result = VaultCrypto::decrypt_value(&key, &enc.ciphertext, &tampered);
        assert!(result.is_err());
    }

    #[test]
    fn tampered_wrapped_dek_fails() {
        let master_key = [0x42u8; 32];
        let dek = VaultCrypto::generate_dek();

        let enc_dek = VaultCrypto::encrypt_dek(&master_key, &dek).unwrap();
        let mut tampered = enc_dek.ciphertext.clone();
        tampered[0] ^= 0xff;

        let result = VaultCrypto::decrypt_dek(&master_key, &tampered, &enc_dek.nonce);
        assert!(result.is_err());
    }

    #[test]
    fn kdf_params_out_of_range_rejected() {
        let params = Argon2Params {
            memory_cost: 999_999_999,
            time_cost: 1,
            parallelism: 1,
        };
        let result = VaultCrypto::derive_master_key(b"pass", &[0u8; 16], &params);
        assert!(result.is_err());
        assert!(matches!(result, Err(CryptoError::KdfParamsOutOfRange)));
    }

    #[test]
    fn every_encryption_uses_unique_nonce() {
        let key = VaultCrypto::generate_dek();
        let plaintext = b"same data every time";
        let mut nonces = std::collections::HashSet::new();

        for _ in 0..100 {
            let enc = VaultCrypto::encrypt_value(&key, plaintext).unwrap();
            assert!(nonces.insert(enc.nonce), "duplicate nonce detected!");
        }
    }

    #[test]
    fn wrong_key_fails_decryption() {
        let key1 = [0x01u8; 32];
        let key2 = [0x02u8; 32];
        let plaintext = b"secret";

        let enc = VaultCrypto::encrypt_value(&key1, plaintext).unwrap();
        let result = VaultCrypto::decrypt_value(&key2, &enc.ciphertext, &enc.nonce);
        assert!(result.is_err());
    }

    #[test]
    fn argon2_params_validation() {
        // Valid params
        assert!(Argon2Params::default().validate().is_ok());
        assert!(Argon2Params::test_fast().validate().is_ok());

        // Zero memory
        assert!(
            Argon2Params {
                memory_cost: 0,
                time_cost: 3,
                parallelism: 4
            }
            .validate()
            .is_err()
        );
        // Too much memory
        assert!(
            Argon2Params {
                memory_cost: 999_999,
                time_cost: 3,
                parallelism: 4
            }
            .validate()
            .is_err()
        );
        // Zero iterations
        assert!(
            Argon2Params {
                memory_cost: 65536,
                time_cost: 0,
                parallelism: 4
            }
            .validate()
            .is_err()
        );
        // Zero parallelism
        assert!(
            Argon2Params {
                memory_cost: 65536,
                time_cost: 3,
                parallelism: 0
            }
            .validate()
            .is_err()
        );
        // Memory < 8 * parallelism
        assert!(
            Argon2Params {
                memory_cost: 8,
                time_cost: 3,
                parallelism: 2
            }
            .validate()
            .is_err()
        );
    }
}
