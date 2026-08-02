use serde::{Deserialize, Serialize};

use crate::error::RuneError;

/// Parametry Argon2id sterujące kosztem derywacji klucza.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgonParams {
    pub memory_kib: u32,  // pamięć w KiB
    pub iterations: u32,  // liczba iteracji
    pub parallelism: u32, // równoległość
}

impl ArgonParams {
    // Presety wołane z UI (przez serializację JS), w Ruście chwilowo nieużywane.
    #[allow(dead_code)]
    /// Preset: Szybkie (słabsze urządzenia)
    pub fn fast() -> Self {
        Self {
            memory_kib: 19456,
            iterations: 2,
            parallelism: 1,
        }
    }

    #[allow(dead_code)]
    /// Preset: Balans (domyślny)
    pub fn balanced() -> Self {
        Self {
            memory_kib: 65536,
            iterations: 3,
            parallelism: 4,
        }
    }

    #[allow(dead_code)]
    /// Preset: Mocne
    pub fn strong() -> Self {
        Self {
            memory_kib: 262144,
            iterations: 4,
            parallelism: 4,
        }
    }

    #[allow(dead_code)]
    /// Preset: Maksymalne
    pub fn maximum() -> Self {
        Self {
            memory_kib: 1048576,
            iterations: 6,
            parallelism: 4,
        }
    }

    /// Walidacja wartości ręcznych.
    pub fn validate(&self) -> Result<(), String> {
        if self.memory_kib < 8192 {
            return Err("Minimum memory is 8 MB".into());
        }
        if self.iterations < 1 {
            return Err("Minimum iterations is 1".into());
        }
        if self.parallelism < 1 || self.parallelism > 16 {
            return Err("Parallelism must be between 1 and 16".into());
        }
        Ok(())
    }
}

/// Zaszyfrowany blob: sól KDF + nonce AEAD + szyfrogram (zawiera tag GCM).
///
/// Format bajtowy: `[32 salt][12 nonce][reszta = ciphertext]`.
pub struct EncryptedFile {
    pub salt: [u8; 32],
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

impl EncryptedFile {
    /// Serializuje do `[32 salt][12 nonce][ciphertext]`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32 + 12 + self.ciphertext.len());
        out.extend_from_slice(&self.salt);
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(&self.ciphertext);
        out
    }

    /// Parsuje `[32 salt][12 nonce][ciphertext]`. Wymaga co najmniej 44 bajtów.
    pub fn from_bytes(data: &[u8]) -> Result<Self, RuneError> {
        if data.len() < 44 {
            return Err(RuneError::Format);
        }
        let mut salt = [0u8; 32];
        salt.copy_from_slice(&data[..32]);
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&data[32..44]);
        let ciphertext = data[44..].to_vec();
        Ok(Self {
            salt,
            nonce,
            ciphertext,
        })
    }
}

/// Metadane vaultu (szyfrowane wewnątrz pliku vault.rune).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMeta {
    pub version: u8,
    pub created_at: u64,
    pub has_duress: bool,
    pub argon_params: ArgonParams, // zapisane przy tworzeniu vaultu
}
