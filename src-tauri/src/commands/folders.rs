//! Trwała, zaszyfrowana lista folderów. Folderów jest mało, więc trzymamy je
//! w jednym pliku `%APPDATA%/rune/folders.rune`.
//!
//! Format (jak notatki, klucz z odblokowanego vaultu):
//! ```text
//! [12 bajtów nonce][szyfrogram + tag GCM]
//! ```

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::State;
use zeroize::Zeroize;

use crate::commands::crypto;
use crate::commands::fsutil;
use crate::commands::vault::{active_vault_dir, VaultState};
use crate::error::RuneError;
use crate::models::EncryptedFile;

/// Folder (cała lista jest szyfrowana razem). camelCase — wymiana z JS.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderData {
    pub id: String,
    pub name: String,
    pub created_at: String, // ISO 8601
    pub order: i32,
}

/// Plik folderów aktywnej sesji: `folders.rune` normalnie, `folders_duress.rune`
/// w trybie duress — żeby wabik nie nadpisał prawdziwej listy folderów.
fn folders_path(state: &State<'_, VaultState>) -> Result<PathBuf, RuneError> {
    let name = if *state.is_duress.lock() {
        "folders_duress.rune"
    } else {
        "folders.rune"
    };
    Ok(active_vault_dir(state)?.join(name))
}

fn require_key(state: &State<'_, VaultState>) -> Result<[u8; 32], String> {
    state
        .key
        .lock()
        .ok_or_else(|| "Vault is locked".to_string())
}

#[tauri::command]
pub async fn save_folders(
    folders: Vec<FolderData>,
    state: State<'_, VaultState>,
) -> Result<(), String> {
    let mut key = require_key(&state)?;

    let mut plaintext = serde_json::to_vec(&folders).map_err(|_| {
        key.zeroize();
        "Folder serialization error".to_string()
    })?;

    let enc = crypto::encrypt(&key, &plaintext).map_err(|_| {
        key.zeroize();
        plaintext.zeroize();
        "Encryption error".to_string()
    })?;
    key.zeroize();
    plaintext.zeroize();

    let dir = active_vault_dir(&state).map_err(|_| "Data directory not found".to_string())?;
    std::fs::create_dir_all(&dir).map_err(|_| "Could not create data directory".to_string())?;

    let mut bytes = Vec::with_capacity(12 + enc.ciphertext.len());
    bytes.extend_from_slice(&enc.nonce);
    bytes.extend_from_slice(&enc.ciphertext);

    let path = folders_path(&state).map_err(|_| "Data directory not found".to_string())?;
    fsutil::write_atomic(&path, &bytes).map_err(|_| "Could not save folders".to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn load_folders(state: State<'_, VaultState>) -> Result<Vec<FolderData>, String> {
    let mut key = require_key(&state)?;

    let path = match folders_path(&state) {
        Ok(p) => p,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };

    // Brak pliku = brak folderów (nie błąd).
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            return Ok(Vec::new());
        }
    };

    if data.len() < 28 {
        key.zeroize();
        return Err("Corrupted folders file".to_string());
    }

    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&data[..12]);
    let enc = EncryptedFile {
        salt: [0u8; 32],
        nonce,
        ciphertext: data[12..].to_vec(),
    };

    let mut plaintext = match crypto::decrypt(&key, &enc) {
        Ok(p) => p,
        Err(_) => {
            key.zeroize();
            return Err("Could not read folders".to_string());
        }
    };
    key.zeroize();

    let folders: Vec<FolderData> =
        serde_json::from_slice(&plaintext).map_err(|_| "Could not read folders".to_string())?;
    plaintext.zeroize();
    Ok(folders)
}
