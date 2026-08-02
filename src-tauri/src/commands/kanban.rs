//! Tablica Kanban: alternatywny widok organizacji notatek w kolumny. Stan
//! (kolumny + przypisania notatek) leży w zaszyfrowanym pliku `kanban.rune`
//! (w trybie duress: `kanban_duress.rune`), w formacie jak foldery:
//! ```text
//! [12 bajtów nonce][szyfrogram + tag GCM]
//! ```
//! Kanban przechowuje wyłącznie przypisania po `note_id` — treść notatek nadal
//! żyje w ich własnych, osobno szyfrowanych plikach.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;
use zeroize::Zeroize;

use crate::commands::crypto;
use crate::commands::fsutil;
use crate::commands::vault::{active_vault_dir, VaultState};
use crate::error::RuneError;
use crate::models::EncryptedFile;

/// Kolumna tablicy. camelCase — wymiana z JS.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KanbanColumn {
    pub id: String,
    pub name: String,
    pub order: i32,
    pub color: Option<String>,
}

/// Przypisanie notatki do kolumny (z pozycją w kolumnie).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KanbanCardAssignment {
    pub note_id: String,
    pub column_id: String,
    pub order: i32,
}

/// Pełny stan tablicy Kanban danego vaultu.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KanbanData {
    pub columns: Vec<KanbanColumn>,
    pub assignments: Vec<KanbanCardAssignment>,
}

/// Domyślne kolumny dla świeżej tablicy.
fn default_kanban() -> KanbanData {
    KanbanData {
        columns: vec![
            KanbanColumn {
                id: Uuid::new_v4().to_string(),
                name: "To Do".into(),
                order: 0,
                color: None,
            },
            KanbanColumn {
                id: Uuid::new_v4().to_string(),
                name: "In Progress".into(),
                order: 1,
                color: None,
            },
            KanbanColumn {
                id: Uuid::new_v4().to_string(),
                name: "Done".into(),
                order: 2,
                color: None,
            },
        ],
        assignments: Vec::new(),
    }
}

/// Plik tablicy aktywnej sesji (`kanban_duress.rune` w trybie wabika).
fn kanban_path(state: &State<'_, VaultState>) -> Result<PathBuf, RuneError> {
    let name = if *state.is_duress.lock() {
        "kanban_duress.rune"
    } else {
        "kanban.rune"
    };
    Ok(active_vault_dir(state)?.join(name))
}

fn require_key(state: &State<'_, VaultState>) -> Result<[u8; 32], String> {
    state
        .key
        .lock()
        .ok_or_else(|| "Vault is locked".to_string())
}

/// Odczytuje i deszyfruje tablicę. Brak pliku → domyślne kolumny (bez zapisu).
fn read_kanban(state: &State<'_, VaultState>, key: &[u8; 32]) -> Result<KanbanData, String> {
    let path = kanban_path(state).map_err(|_| "Data directory not found".to_string())?;

    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => return Ok(default_kanban()),
    };
    if data.len() < 28 {
        return Err("Corrupted kanban file".to_string());
    }

    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&data[..12]);
    let enc = EncryptedFile {
        salt: [0u8; 32],
        nonce,
        ciphertext: data[12..].to_vec(),
    };

    let mut plaintext =
        crypto::decrypt(key, &enc).map_err(|_| "Could not read kanban".to_string())?;
    let kd: KanbanData =
        serde_json::from_slice(&plaintext).map_err(|_| "Could not read kanban".to_string())?;
    plaintext.zeroize();
    Ok(kd)
}

/// Szyfruje i zapisuje tablicę na dysk.
fn write_kanban(
    state: &State<'_, VaultState>,
    key: &[u8; 32],
    data: &KanbanData,
) -> Result<(), String> {
    let mut plaintext =
        serde_json::to_vec(data).map_err(|_| "Kanban serialization error".to_string())?;
    let enc = crypto::encrypt(key, &plaintext).map_err(|_| {
        plaintext.zeroize();
        "Encryption error".to_string()
    })?;
    plaintext.zeroize();

    let dir = active_vault_dir(state).map_err(|_| "Data directory not found".to_string())?;
    std::fs::create_dir_all(&dir).map_err(|_| "Could not create data directory".to_string())?;

    let mut bytes = Vec::with_capacity(12 + enc.ciphertext.len());
    bytes.extend_from_slice(&enc.nonce);
    bytes.extend_from_slice(&enc.ciphertext);

    let path = kanban_path(state).map_err(|_| "Data directory not found".to_string())?;
    fsutil::write_atomic(&path, &bytes).map_err(|_| "Could not save kanban".to_string())
}

// ── Komendy ─────────────────────────────────────────────────────────────────

/// Wczytuje tablicę. Przy pierwszym uruchomieniu tworzy i utrwala domyślne
/// kolumny (żeby ich id były stabilne między sesjami).
#[tauri::command]
pub async fn load_kanban(state: State<'_, VaultState>) -> Result<KanbanData, String> {
    let mut key = require_key(&state)?;
    let path = match kanban_path(&state) {
        Ok(p) => p,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };

    if path.exists() {
        let res = read_kanban(&state, &key);
        key.zeroize();
        return res;
    }

    // Pierwsze otwarcie: utwórz domyślne kolumny i zapisz je od razu.
    let def = default_kanban();
    let res = write_kanban(&state, &key, &def);
    key.zeroize();
    res?;
    Ok(def)
}

/// Zapisuje całą tablicę (frontend wysyła pełny, zmodyfikowany stan).
#[tauri::command]
pub async fn save_kanban(data: KanbanData, state: State<'_, VaultState>) -> Result<(), String> {
    let mut key = require_key(&state)?;
    let res = write_kanban(&state, &key, &data);
    key.zeroize();
    res
}

/// Dodaje notatkę na koniec wskazanej kolumny (idempotentnie po `note_id`).
#[tauri::command]
pub async fn add_note_to_kanban(
    note_id: String,
    column_id: String,
    state: State<'_, VaultState>,
) -> Result<KanbanData, String> {
    let mut key = require_key(&state)?;
    let mut kd = match read_kanban(&state, &key) {
        Ok(k) => k,
        Err(e) => {
            key.zeroize();
            return Err(e);
        }
    };

    if !kd.assignments.iter().any(|a| a.note_id == note_id) {
        let order = kd
            .assignments
            .iter()
            .filter(|a| a.column_id == column_id)
            .map(|a| a.order)
            .max()
            .map(|m| m + 1)
            .unwrap_or(0);
        kd.assignments.push(KanbanCardAssignment {
            note_id,
            column_id,
            order,
        });
    }

    let res = write_kanban(&state, &key, &kd);
    key.zeroize();
    res?;
    Ok(kd)
}

/// Przenosi kartę do kolumny `target_column_id` na pozycję `target_order`
/// (przeliczając porządek całej kolumny docelowej).
#[tauri::command]
pub async fn move_kanban_card(
    note_id: String,
    target_column_id: String,
    target_order: i32,
    state: State<'_, VaultState>,
) -> Result<KanbanData, String> {
    let mut key = require_key(&state)?;
    let mut kd = match read_kanban(&state, &key) {
        Ok(k) => k,
        Err(e) => {
            key.zeroize();
            return Err(e);
        }
    };

    // Odłącz przenoszoną kartę.
    let mut moving: Option<KanbanCardAssignment> = None;
    kd.assignments.retain(|a| {
        if a.note_id == note_id {
            moving = Some(a.clone());
            false
        } else {
            true
        }
    });

    if let Some(mut m) = moving {
        m.column_id = target_column_id.clone();

        // Karty kolumny docelowej w aktualnym porządku.
        let mut col: Vec<KanbanCardAssignment> = kd
            .assignments
            .iter()
            .filter(|a| a.column_id == target_column_id)
            .cloned()
            .collect();
        col.sort_by_key(|a| a.order);

        let idx = (target_order.max(0) as usize).min(col.len());
        col.insert(idx, m);
        for (i, a) in col.iter_mut().enumerate() {
            a.order = i as i32;
        }

        let mut rest: Vec<KanbanCardAssignment> = kd
            .assignments
            .into_iter()
            .filter(|a| a.column_id != target_column_id)
            .collect();
        rest.extend(col);
        kd.assignments = rest;
    }

    let res = write_kanban(&state, &key, &kd);
    key.zeroize();
    res?;
    Ok(kd)
}

/// Usuwa notatkę z tablicy (sama notatka pozostaje nienaruszona).
#[tauri::command]
pub async fn remove_note_from_kanban(
    note_id: String,
    state: State<'_, VaultState>,
) -> Result<KanbanData, String> {
    let mut key = require_key(&state)?;
    let mut kd = match read_kanban(&state, &key) {
        Ok(k) => k,
        Err(e) => {
            key.zeroize();
            return Err(e);
        }
    };

    kd.assignments.retain(|a| a.note_id != note_id);

    let res = write_kanban(&state, &key, &kd);
    key.zeroize();
    res?;
    Ok(kd)
}
