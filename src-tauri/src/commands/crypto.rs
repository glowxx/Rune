//! Prymitywy kryptograficzne: derywacja klucza (Argon2id) i AEAD (AES-256-GCM).
//!
//! Bezpieczeństwo:
//! - Klucz, hasło ani plaintext nigdy nie są logowane.
//! - Każde `encrypt` generuje świeży, losowy nonce (ring::rand).
//! - Błędy `decrypt` są ogólne (RuneError::Crypto) — nie ujawniają, czy
//!   powodem jest złe hasło, czy uszkodzone dane.

use argon2::{Algorithm, Argon2, Params, Version};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};

use crate::error::RuneError;
use crate::models::{ArgonParams, EncryptedFile};

/// Derywuje 32-bajtowy klucz z hasła i soli przy użyciu Argon2id.
pub fn derive_key(
    password: &str,
    salt: &[u8; 32],
    params: &ArgonParams,
) -> Result<[u8; 32], RuneError> {
    let p = Params::new(
        params.memory_kib,
        params.iterations,
        params.parallelism,
        Some(32),
    )
    .map_err(|_| RuneError::Crypto)?;

    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, p);

    let mut key = [0u8; 32];
    argon
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|_| RuneError::Crypto)?;
    Ok(key)
}

/// Szyfruje plaintext AES-256-GCM ze świeżym losowym nonce.
///
/// Pole `salt` zwróconego `EncryptedFile` jest puste — sól KDF ustawia
/// wywołujący (zależy od kontekstu derywacji klucza).
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<EncryptedFile, RuneError> {
    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes).map_err(|_| RuneError::Crypto)?;

    let unbound = UnboundKey::new(&AES_256_GCM, key).map_err(|_| RuneError::Crypto)?;
    let sealing = LessSafeKey::new(unbound);

    let mut in_out = plaintext.to_vec();
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    sealing
        .seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| RuneError::Crypto)?;

    Ok(EncryptedFile {
        salt: [0u8; 32],
        nonce: nonce_bytes,
        ciphertext: in_out, // szyfrogram || tag GCM
    })
}

/// Odszyfrowuje AES-256-GCM. Błąd jest ogólny — nie ujawnia powodu.
pub fn decrypt(key: &[u8; 32], file: &EncryptedFile) -> Result<Vec<u8>, RuneError> {
    let unbound = UnboundKey::new(&AES_256_GCM, key).map_err(|_| RuneError::Crypto)?;
    let opening = LessSafeKey::new(unbound);

    let nonce = Nonce::assume_unique_for_key(file.nonce);
    let mut in_out = file.ciphertext.clone();
    let plaintext = opening
        .open_in_place(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| RuneError::Crypto)?;

    Ok(plaintext.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_params() -> ArgonParams {
        ArgonParams {
            memory_kib: 8_192,
            iterations: 1,
            parallelism: 1,
        }
    }

    #[test]
    fn derive_key_is_deterministic_and_salt_separated() {
        let params = test_params();
        let first = derive_key("correct horse battery staple", &[1u8; 32], &params).unwrap();
        let repeated = derive_key("correct horse battery staple", &[1u8; 32], &params).unwrap();
        let other_salt = derive_key("correct horse battery staple", &[2u8; 32], &params).unwrap();

        assert_eq!(first, repeated);
        assert_ne!(first, other_salt);
    }

    #[test]
    fn encrypt_decrypt_round_trip() {
        let key = [7u8; 32];
        let plaintext = b"private rune note";
        let encrypted = encrypt(&key, plaintext).unwrap();

        assert_ne!(encrypted.ciphertext, plaintext);
        assert_eq!(decrypt(&key, &encrypted).unwrap(), plaintext);
    }

    #[test]
    fn wrong_key_and_tampering_are_rejected() {
        let key = [7u8; 32];
        let encrypted = encrypt(&key, b"authenticated payload").unwrap();

        assert!(decrypt(&[8u8; 32], &encrypted).is_err());

        let mut tampered = encrypted;
        tampered.ciphertext[0] ^= 0x80;
        assert!(decrypt(&key, &tampered).is_err());
    }

    #[test]
    fn every_encryption_uses_a_fresh_nonce() {
        let key = [7u8; 32];
        let first = encrypt(&key, b"same plaintext").unwrap();
        let second = encrypt(&key, b"same plaintext").unwrap();

        assert_ne!(first.nonce, second.nonce);
        assert_ne!(first.ciphertext, second.ciphertext);
    }
}
