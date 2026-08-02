//! Osadzone tablice Excalidraw. Każda tablica to scena Excalidraw (JSON) zapisana
//! zaszyfrowana w `whiteboards/{id}.rune` (`[12 nonce][ciphertext+tag]`), z
//! indeksem metadanych w `whiteboards_meta.rune` (zaszyfrowany JSON listy
//! `WhiteboardMeta`) — analogicznie do załączników.
//!
//! Excalidraw wymaga pełnego środowiska React, więc działa w osobnym oknie Tauri
//! (`static/whiteboard.html`). Okno pobiera scenę przez `load_whiteboard`, a
//! zmiany zapisuje (debounce po stronie okna) przez `save_whiteboard_from_window`.
//!
//! Tryb duress używa `whiteboards_duress/` i `whiteboards_meta_duress.rune`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use zeroize::Zeroize;

use crate::commands::crypto;
use crate::commands::fsutil;
use crate::commands::vault::{active_vault_dir, VaultState};
use crate::error::RuneError;
use crate::models::EncryptedFile;

/// Metadane tablicy (camelCase — wymiana z JS).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhiteboardMeta {
    pub id: String,
    pub note_id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Pełna tablica: metadane + surowa scena Excalidraw (JSON jako string).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhiteboardData {
    pub meta: WhiteboardMeta,
    pub excalidraw_json: String,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn is_duress(state: &State<'_, VaultState>) -> bool {
    *state.is_duress.lock()
}

fn whiteboards_dir(state: &State<'_, VaultState>) -> Result<PathBuf, RuneError> {
    let sub = if is_duress(state) {
        "whiteboards_duress"
    } else {
        "whiteboards"
    };
    Ok(active_vault_dir(state)?.join(sub))
}

fn meta_path(state: &State<'_, VaultState>) -> Result<PathBuf, RuneError> {
    let name = if is_duress(state) {
        "whiteboards_meta_duress.rune"
    } else {
        "whiteboards_meta.rune"
    };
    Ok(active_vault_dir(state)?.join(name))
}

/// Czy id jest „czyste" (ochrona przed path traversal — id z frontendu).
fn clean_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn scene_file(dir: &Path, id: &str) -> Result<PathBuf, RuneError> {
    if !clean_id(id) {
        return Err(RuneError::Format);
    }
    Ok(dir.join(format!("{id}.rune")))
}

fn require_key(state: &State<'_, VaultState>) -> Result<[u8; 32], String> {
    state
        .key
        .lock()
        .ok_or_else(|| "Vault is locked".to_string())
}

fn build_file(enc: &EncryptedFile) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + enc.ciphertext.len());
    out.extend_from_slice(&enc.nonce);
    out.extend_from_slice(&enc.ciphertext);
    out
}

fn decrypt_file(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, RuneError> {
    if data.len() < 28 {
        return Err(RuneError::Format);
    }
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&data[..12]);
    let enc = EncryptedFile {
        salt: [0u8; 32],
        nonce,
        ciphertext: data[12..].to_vec(),
    };
    crypto::decrypt(key, &enc)
}

/// Odczyt indeksu metadanych. Brak pliku = pusta lista.
fn read_meta(key: &[u8; 32], path: &Path) -> Result<Vec<WhiteboardMeta>, RuneError> {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };
    let mut plain = decrypt_file(key, &data)?;
    let list: Vec<WhiteboardMeta> = serde_json::from_slice(&plain).unwrap_or_default();
    plain.zeroize();
    Ok(list)
}

fn write_meta(key: &[u8; 32], path: &Path, list: &[WhiteboardMeta]) -> Result<(), RuneError> {
    let mut plain = serde_json::to_vec(list)?;
    let enc = crypto::encrypt(key, &plain)?;
    plain.zeroize();
    fsutil::write_atomic(path, &build_file(&enc))?;
    Ok(())
}

fn write_scene(key: &[u8; 32], dir: &Path, id: &str, json: &str) -> Result<(), String> {
    let path = scene_file(dir, id).map_err(|_| "Invalid whiteboard id".to_string())?;
    let enc = crypto::encrypt(key, json.as_bytes()).map_err(|_| "Encryption error".to_string())?;
    fsutil::write_atomic(&path, &build_file(&enc))
        .map_err(|_| "Could not save whiteboard".to_string())
}

fn read_scene(key: &[u8; 32], dir: &Path, id: &str) -> Option<String> {
    let path = scene_file(dir, id).ok()?;
    let data = std::fs::read(&path).ok()?;
    let bytes = decrypt_file(key, &data).ok()?;
    String::from_utf8(bytes).ok()
}

/// Wspólny zapis: scena + upsert metadanych. `title == None` zachowuje istniejący
/// tytuł (autozapis z okna nie może wymazać zmiany nazwy zrobionej w panelu).
fn save_inner(
    state: &State<'_, VaultState>,
    key: &[u8; 32],
    id: &str,
    note_id: &str,
    title: Option<&str>,
    excalidraw_json: &str,
) -> Result<WhiteboardMeta, String> {
    let dir = whiteboards_dir(state).map_err(|_| "Data directory not found".to_string())?;
    std::fs::create_dir_all(&dir)
        .map_err(|_| "Could not create whiteboards directory".to_string())?;
    write_scene(key, &dir, id, excalidraw_json)?;

    let mpath = meta_path(state).map_err(|_| "Data directory not found".to_string())?;
    let mut list =
        read_meta(key, &mpath).map_err(|_| "Could not read whiteboards index".to_string())?;

    let now = now_ms();
    let result = if let Some(m) = list.iter_mut().find(|m| m.id == id) {
        if let Some(t) = title {
            m.title = t.to_string();
        }
        m.note_id = note_id.to_string();
        m.updated_at = now;
        m.clone()
    } else {
        let m = WhiteboardMeta {
            id: id.to_string(),
            note_id: note_id.to_string(),
            title: title.unwrap_or("Untitled board").to_string(),
            created_at: now,
            updated_at: now,
        };
        list.push(m.clone());
        m
    };
    write_meta(key, &mpath, &list).map_err(|_| "Could not update whiteboards index".to_string())?;
    Ok(result)
}

/// Zamyka wszystkie okna tablic (etykieta `whiteboard-*`). Wołane przy locku.
pub fn close_all_whiteboard_windows(app: &AppHandle) {
    for (label, win) in app.webview_windows() {
        if label.starts_with("whiteboard-") {
            let _ = win.close();
        }
    }
}

// ── Komendy: CRUD ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_whiteboards(
    note_id: String,
    state: State<'_, VaultState>,
) -> Result<Vec<WhiteboardMeta>, String> {
    let mut key = require_key(&state)?;
    let mpath = match meta_path(&state) {
        Ok(p) => p,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };
    let list = read_meta(&key, &mpath).unwrap_or_default();
    key.zeroize();
    Ok(list.into_iter().filter(|m| m.note_id == note_id).collect())
}

#[tauri::command]
pub async fn load_whiteboard(
    whiteboard_id: String,
    state: State<'_, VaultState>,
) -> Result<WhiteboardData, String> {
    let mut key = require_key(&state)?;

    let mpath = match meta_path(&state) {
        Ok(p) => p,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };
    let meta = read_meta(&key, &mpath)
        .unwrap_or_default()
        .into_iter()
        .find(|m| m.id == whiteboard_id);

    let dir = match whiteboards_dir(&state) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };
    let scene = read_scene(&key, &dir, &whiteboard_id);
    key.zeroize();

    let Some(meta) = meta else {
        return Err("Whiteboard not found".to_string());
    };
    Ok(WhiteboardData {
        meta,
        excalidraw_json: scene.unwrap_or_else(|| "{}".to_string()),
    })
}

/// Tworzy lub aktualizuje tablicę. Jeśli okno tej tablicy jest otwarte i zmienił
/// się tytuł — aktualizuje też tytuł okna (rename odbija się natychmiast).
#[tauri::command]
pub async fn save_whiteboard(
    app: AppHandle,
    whiteboard_id: String,
    note_id: String,
    title: String,
    excalidraw_json: String,
    state: State<'_, VaultState>,
) -> Result<WhiteboardMeta, String> {
    let mut key = require_key(&state)?;
    let res = save_inner(
        &state,
        &key,
        &whiteboard_id,
        &note_id,
        Some(&title),
        &excalidraw_json,
    );
    key.zeroize();
    let meta = res?;

    if let Some(win) = app.get_webview_window(&format!("whiteboard-{whiteboard_id}")) {
        let _ = win.set_title(&title);
    }
    Ok(meta)
}

#[tauri::command]
pub async fn delete_whiteboard(
    app: AppHandle,
    whiteboard_id: String,
    state: State<'_, VaultState>,
) -> Result<(), String> {
    // Zamknij ewentualne otwarte okno tej tablicy.
    if let Some(win) = app.get_webview_window(&format!("whiteboard-{whiteboard_id}")) {
        let _ = win.close();
    }

    let mut key = require_key(&state)?;

    if let Ok(dir) = whiteboards_dir(&state) {
        if let Ok(path) = scene_file(&dir, &whiteboard_id) {
            let _ = std::fs::remove_file(path);
        }
    }
    if let Ok(mpath) = meta_path(&state) {
        if let Ok(mut list) = read_meta(&key, &mpath) {
            let before = list.len();
            list.retain(|m| m.id != whiteboard_id);
            if list.len() != before {
                let _ = write_meta(&key, &mpath, &list);
            }
        }
    }
    key.zeroize();
    Ok(())
}

// ── Komendy: okno Excalidraw ────────────────────────────────────────────────

/// Otwiera (lub fokusuje) okno Excalidraw dla danej tablicy. Dane inicjalne
/// (id/note/title) wstrzykujemy skryptem startowym; scenę okno pobiera samo
/// przez `load_whiteboard` — to unika wyścigu z eventem inicjalizacyjnym.
#[tauri::command]
pub async fn open_whiteboard_window(
    app: AppHandle,
    whiteboard_id: String,
    note_id: String,
    title: String,
) -> Result<(), String> {
    if !clean_id(&whiteboard_id) {
        return Err("Invalid whiteboard id".to_string());
    }
    let label = format!("whiteboard-{whiteboard_id}");

    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.set_focus();
        return Ok(());
    }

    let init = serde_json::json!({
        "id": whiteboard_id,
        "noteId": note_id,
        "title": title,
    })
    .to_string();
    let script = format!("window.__WB_INIT__ = {init};");

    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("whiteboard.html".into()))
        .title(&title)
        .inner_size(1200.0, 800.0)
        .initialization_script(&script)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Zapis wywoływany z okna Excalidraw (po debounce). Zapisuje scenę i powiadamia
/// główne okno eventem `rune://whiteboard-saved`, by panel odświeżył kartę.
#[tauri::command]
pub async fn save_whiteboard_from_window(
    app: AppHandle,
    whiteboard_id: String,
    note_id: String,
    scene_json: String,
    state: State<'_, VaultState>,
) -> Result<(), String> {
    let mut key = require_key(&state)?;
    // `None` — autozapis z okna nie zmienia tytułu (zachowuje rename z panelu).
    let res = save_inner(&state, &key, &whiteboard_id, &note_id, None, &scene_json);
    key.zeroize();
    let meta = res?;

    let _ = app.emit(
        "rune://whiteboard-saved",
        serde_json::json!({ "whiteboardId": meta.id, "updatedAt": meta.updated_at }),
    );
    Ok(())
}
