use thiserror::Error;

/// Błędy wewnętrzne warstwy kryptografii i vaultu.
///
/// Uwaga bezpieczeństwa: warianty są celowo ogólne. W szczególności `Crypto`
/// pokrywa zarówno błąd derywacji klucza, jak i nieudane odszyfrowanie —
/// nie ujawniamy, czy powodem było złe hasło, czy uszkodzone dane.
#[derive(Debug, Error)]
pub enum RuneError {
    #[error("cryptographic operation failed")]
    Crypto,

    #[error("invalid data format")]
    Format,

    #[error("data directory not found")]
    DataDir,

    #[error("input/output error")]
    Io(#[from] std::io::Error),

    #[error("serialization error")]
    Serde(#[from] serde_json::Error),
}
