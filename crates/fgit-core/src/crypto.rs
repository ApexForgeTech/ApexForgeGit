use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use rand::RngCore;

pub struct EncryptionEngine;

impl EncryptionEngine {
    /// Encrypt data with the given key (key must be exactly 32 bytes)
    pub fn encrypt(data: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
        if key.len() != 32 {
            return Err("Encryption key must be exactly 32 bytes".to_string());
        }

        let key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key);

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes); // 96-bits; unique per message

        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|e| format!("Encryption failed: {}", e))?;

        // Prepend the nonce to the ciphertext
        let mut final_data: Vec<u8> = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
        final_data.extend_from_slice(&nonce_bytes);
        final_data.extend_from_slice(&ciphertext);
        
        Ok(final_data)
    }

    /// Decrypt data with the given key (key must be exactly 32 bytes)
    pub fn decrypt(encrypted_data: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
        if key.len() != 32 {
            return Err("Decryption key must be exactly 32 bytes".to_string());
        }
        if encrypted_data.len() < 12 + 16 {
            // nonce + auth tag minimum length
            return Err("Invalid encrypted data format or too short".to_string());
        }

        let key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key);
        
        let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        cipher.decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {}", e))
    }

    /// Generate a new random AES-256 key
    pub fn generate_key() -> [u8; 32] {
        Aes256Gcm::generate_key(OsRng).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_roundtrip() {
        let key = EncryptionEngine::generate_key();
        let message = b"Confidential ApexForge GIT data!";
        let cipher = EncryptionEngine::encrypt(message, &key).unwrap();
        let decrypted = EncryptionEngine::decrypt(&cipher, &key).unwrap();
        
        assert_eq!(message, decrypted.as_slice());
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = EncryptionEngine::generate_key();
        let key2 = EncryptionEngine::generate_key();
        
        let message = b"Secret text";
        let cipher = EncryptionEngine::encrypt(message, &key1).unwrap();
        
        let result = EncryptionEngine::decrypt(&cipher, &key2);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_key_length() {
        let key = [0u8; 16]; // Only 16 bytes instead of 32
        let result = EncryptionEngine::encrypt(b"test", &key);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("32 bytes"));
    }
}
