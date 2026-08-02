//! Śledzenie czasu pracy nad notatką. Każdy wpis to jedna sesja (start–stop).
//! Dane leżą w zaszyfrowanym `time_tracking.rune` (duress: `time_tracking_duress.rune`),
//! w tym samym formacie co `kanban.rune`:
//! ```text
//! [12 bajtów nonce][szyfrogram + tag GCM]
//! ```
//! W danym momencie biegnie najwyżej jeden licznik (start zatrzymuje poprzedni).

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

/// Jedna sesja pracy nad notatką. camelCase — wymiana z JS.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeEntry {
    pub id: String,
    pub note_id: String,
    pub started_at: u64,       // unix ms
    pub ended_at: Option<u64>, // None = wciąż biegnie
    pub duration_ms: u64,      // wyliczane przy stop: ended_at - started_at
}

/// Pełny stan śledzenia czasu danego vaultu.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TimeTrackingData {
    pub entries: Vec<TimeEntry>,
}

/// Suma czasu dla jednej notatki.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSummary {
    pub note_id: String,
    pub total_ms: u64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Plik śledzenia czasu aktywnej sesji (`*_duress` w trybie wabika).
fn tracking_path(state: &State<'_, VaultState>) -> Result<PathBuf, RuneError> {
    let name = if *state.is_duress.lock() {
        "time_tracking_duress.rune"
    } else {
        "time_tracking.rune"
    };
    Ok(active_vault_dir(state)?.join(name))
}

fn require_key(state: &State<'_, VaultState>) -> Result<[u8; 32], String> {
    state
        .key
        .lock()
        .ok_or_else(|| "Vault is locked".to_string())
}

/// Odczyt + deszyfracja. Brak pliku → pusta lista (nie błąd).
fn read_tracking(
    state: &State<'_, VaultState>,
    key: &[u8; 32],
) -> Result<TimeTrackingData, String> {
    let path = tracking_path(state).map_err(|_| "Data directory not found".to_string())?;

    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => return Ok(TimeTrackingData::default()),
    };
    if data.len() < 28 {
        return Err("Corrupted time tracking file".to_string());
    }

    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&data[..12]);
    let enc = EncryptedFile {
        salt: [0u8; 32],
        nonce,
        ciphertext: data[12..].to_vec(),
    };

    let mut plaintext =
        crypto::decrypt(key, &enc).map_err(|_| "Could not read time tracking".to_string())?;
    let td: TimeTrackingData = serde_json::from_slice(&plaintext)
        .map_err(|_| "Could not read time tracking".to_string())?;
    plaintext.zeroize();
    Ok(td)
}

/// Szyfruje i zapisuje stan na dysk.
fn write_tracking(
    state: &State<'_, VaultState>,
    key: &[u8; 32],
    data: &TimeTrackingData,
) -> Result<(), String> {
    let mut plaintext =
        serde_json::to_vec(data).map_err(|_| "Time tracking serialization error".to_string())?;
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

    let path = tracking_path(state).map_err(|_| "Data directory not found".to_string())?;
    fsutil::write_atomic(&path, &bytes).map_err(|_| "Could not save time tracking".to_string())
}

/// Zatrzymuje każdy wciąż biegnący wpis (dla dowolnej notatki). Zwraca `true`,
/// jeśli coś zatrzymano. Wspólne dla `start_timer` (zamyka poprzedni licznik).
fn stop_running(data: &mut TimeTrackingData, now: u64) -> bool {
    let mut changed = false;
    for e in data.entries.iter_mut() {
        if e.ended_at.is_none() {
            e.ended_at = Some(now);
            e.duration_ms = now.saturating_sub(e.started_at);
            changed = true;
        }
    }
    changed
}

// ── Komendy ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn load_time_entries(state: State<'_, VaultState>) -> Result<Vec<TimeEntry>, String> {
    let mut key = require_key(&state)?;
    let res = read_tracking(&state, &key);
    key.zeroize();
    res.map(|d| d.entries)
}

/// Startuje nowy licznik dla notatki, zatrzymując najpierw każdy inny biegnący
/// (gwarancja: maks. jeden aktywny licznik naraz).
#[tauri::command]
pub async fn start_timer(
    note_id: String,
    state: State<'_, VaultState>,
) -> Result<TimeEntry, String> {
    let mut key = require_key(&state)?;
    let mut data = match read_tracking(&state, &key) {
        Ok(d) => d,
        Err(e) => {
            key.zeroize();
            return Err(e);
        }
    };

    let now = now_ms();
    stop_running(&mut data, now);

    let entry = TimeEntry {
        id: Uuid::new_v4().to_string(),
        note_id,
        started_at: now,
        ended_at: None,
        duration_ms: 0,
    };
    data.entries.push(entry.clone());

    let res = write_tracking(&state, &key, &data);
    key.zeroize();
    res?;
    Ok(entry)
}

/// Zatrzymuje biegnący licznik danej notatki i zwraca zaktualizowany wpis.
#[tauri::command]
pub async fn stop_timer(
    note_id: String,
    state: State<'_, VaultState>,
) -> Result<TimeEntry, String> {
    let mut key = require_key(&state)?;
    let mut data = match read_tracking(&state, &key) {
        Ok(d) => d,
        Err(e) => {
            key.zeroize();
            return Err(e);
        }
    };

    let now = now_ms();
    let mut updated: Option<TimeEntry> = None;
    for e in data.entries.iter_mut() {
        if e.note_id == note_id && e.ended_at.is_none() {
            e.ended_at = Some(now);
            e.duration_ms = now.saturating_sub(e.started_at);
            updated = Some(e.clone());
            break;
        }
    }

    let Some(entry) = updated else {
        key.zeroize();
        return Err("No running timer for this note".to_string());
    };

    let res = write_tracking(&state, &key, &data);
    key.zeroize();
    res?;
    Ok(entry)
}

/// Usuwa konkretny wpis czasu po id.
#[tauri::command]
pub async fn delete_time_entry(
    entry_id: String,
    state: State<'_, VaultState>,
) -> Result<(), String> {
    let mut key = require_key(&state)?;
    let mut data = match read_tracking(&state, &key) {
        Ok(d) => d,
        Err(e) => {
            key.zeroize();
            return Err(e);
        }
    };

    data.entries.retain(|e| e.id != entry_id);

    let res = write_tracking(&state, &key, &data);
    key.zeroize();
    res
}

/// Sumuje czas (`duration_ms`) per notatka. Biegnące wpisy (jeszcze bez końca)
/// nie wliczają się — liczymy tylko zamknięte sesje.
#[tauri::command]
pub async fn get_time_summary(state: State<'_, VaultState>) -> Result<Vec<NoteSummary>, String> {
    let mut key = require_key(&state)?;
    let data = match read_tracking(&state, &key) {
        Ok(d) => d,
        Err(e) => {
            key.zeroize();
            return Err(e);
        }
    };
    key.zeroize();

    let mut totals: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for e in &data.entries {
        if e.ended_at.is_some() {
            *totals.entry(e.note_id.clone()).or_insert(0) += e.duration_ms;
        }
    }

    Ok(totals
        .into_iter()
        .map(|(note_id, total_ms)| NoteSummary { note_id, total_ms })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(note: &str, start: u64, end: Option<u64>, dur: u64) -> TimeEntry {
        TimeEntry {
            id: Uuid::new_v4().to_string(),
            note_id: note.into(),
            started_at: start,
            ended_at: end,
            duration_ms: dur,
        }
    }

    #[test]
    fn stop_running_closes_all_open_entries() {
        let mut data = TimeTrackingData {
            entries: vec![entry("a", 1000, None, 0), entry("b", 2000, Some(2500), 500)],
        };
        let changed = stop_running(&mut data, 3000);
        assert!(changed);
        // Otwarty wpis „a" zamknięty z duration = 3000-1000.
        let a = data.entries.iter().find(|e| e.note_id == "a").unwrap();
        assert_eq!(a.ended_at, Some(3000));
        assert_eq!(a.duration_ms, 2000);
        // „b" pozostaje bez zmian.
        let b = data.entries.iter().find(|e| e.note_id == "b").unwrap();
        assert_eq!(b.duration_ms, 500);
    }

    #[test]
    fn stop_running_noop_when_nothing_open() {
        let mut data = TimeTrackingData {
            entries: vec![entry("a", 1000, Some(1500), 500)],
        };
        assert!(!stop_running(&mut data, 3000));
    }
}
