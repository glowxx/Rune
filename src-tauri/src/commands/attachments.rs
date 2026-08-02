//! Zaszyfrowane załączniki (obrazy, PDF). Każdy plik osobno:
//! `%APPDATA%/rune/attachments/{id}.rune` = `[12 nonce][ciphertext+tag]`,
//! szyfrowany tym samym kluczem co notatki. Indeks metadanych:
//! `attachments_meta.rune` (zaszyfrowany JSON listy `AttachmentMeta`).
//!
//! W trybie duress używane są warianty `attachments_duress/` oraz
//! `attachments_meta_duress.rune`, żeby wabik nie widział (ani nie nadpisał)
//! prawdziwych załączników.

use std::path::{Path, PathBuf};

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;
use zeroize::Zeroize;

use crate::commands::crypto;
use crate::commands::fsutil;
use crate::commands::vault::{active_vault_dir, VaultState};
use crate::error::RuneError;
use crate::models::EncryptedFile;

/// Maksymalny rozmiar pojedynczego załącznika: 20 MB.
const MAX_SIZE: usize = 20 * 1024 * 1024;

/// Metadane załącznika (camelCase — wymiana z JS).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentMeta {
    pub id: String,
    pub original_filename: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub note_id: String,
    pub created_at: String,
}

fn is_duress(state: &State<'_, VaultState>) -> bool {
    *state.is_duress.lock()
}

fn attachments_dir(state: &State<'_, VaultState>) -> Result<PathBuf, RuneError> {
    let sub = if is_duress(state) {
        "attachments_duress"
    } else {
        "attachments"
    };
    Ok(active_vault_dir(state)?.join(sub))
}

fn meta_path(state: &State<'_, VaultState>) -> Result<PathBuf, RuneError> {
    let name = if is_duress(state) {
        "attachments_meta_duress.rune"
    } else {
        "attachments_meta.rune"
    };
    Ok(active_vault_dir(state)?.join(name))
}

fn notes_dir_for(state: &State<'_, VaultState>) -> Result<PathBuf, RuneError> {
    let sub = if is_duress(state) {
        "notes_duress"
    } else {
        "notes"
    };
    Ok(active_vault_dir(state)?.join(sub))
}

/// Czy id jest „czyste" (ochrona przed path traversal — id z frontendu).
fn clean_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn attachment_file(dir: &Path, id: &str) -> Result<PathBuf, RuneError> {
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

fn now_string() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_default()
}

/// Składa bajty pliku: `[12 nonce][ciphertext]`.
fn build_file(enc: &EncryptedFile) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + enc.ciphertext.len());
    out.extend_from_slice(&enc.nonce);
    out.extend_from_slice(&enc.ciphertext);
    out
}

/// Deszyfruje surowe bajty pliku załącznika do zwykłych bajtów.
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

/// Odczytuje indeks metadanych. Brak pliku = pusta lista (nie błąd).
fn read_meta(key: &[u8; 32], path: &Path) -> Result<Vec<AttachmentMeta>, RuneError> {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };
    let mut plain = decrypt_file(key, &data)?;
    let list: Vec<AttachmentMeta> = serde_json::from_slice(&plain).unwrap_or_default();
    plain.zeroize();
    Ok(list)
}

/// Zapisuje cały indeks metadanych (analogicznie do `save_folders`).
fn write_meta(key: &[u8; 32], path: &Path, list: &[AttachmentMeta]) -> Result<(), RuneError> {
    let mut plain = serde_json::to_vec(list)?;
    let enc = crypto::encrypt(key, &plain)?;
    plain.zeroize();
    fsutil::write_atomic(path, &build_file(&enc))?;
    Ok(())
}

/// Rozszerzenie pliku — z oryginalnej nazwy lub z typu MIME.
fn ext_for(filename: &str, mime: &str) -> String {
    if let Some(dot) = filename.rfind('.') {
        let ext = &filename[dot + 1..];
        if !ext.is_empty() && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
            return ext.to_lowercase();
        }
    }
    match mime {
        "application/pdf" => "pdf",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        _ => "bin",
    }
    .to_string()
}

#[tauri::command]
pub async fn save_attachment(
    mut file_bytes: Vec<u8>,
    original_filename: String,
    mime_type: String,
    note_id: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    if file_bytes.len() > MAX_SIZE {
        file_bytes.zeroize();
        return Err("File too large (max 20MB)".to_string());
    }
    let size_bytes = file_bytes.len() as u64;

    let mut key = require_key(&state)?;

    let dir = attachments_dir(&state).map_err(|_| {
        key.zeroize();
        "Data directory not found".to_string()
    })?;
    std::fs::create_dir_all(&dir).map_err(|_| {
        key.zeroize();
        "Could not create attachments directory".to_string()
    })?;

    let id = Uuid::new_v4().to_string();

    let enc = crypto::encrypt(&key, &file_bytes).map_err(|_| {
        key.zeroize();
        file_bytes.zeroize();
        "Encryption error".to_string()
    })?;
    file_bytes.zeroize();

    let path = attachment_file(&dir, &id).map_err(|_| {
        key.zeroize();
        "Invalid attachment id".to_string()
    })?;
    if fsutil::write_atomic(&path, &build_file(&enc)).is_err() {
        key.zeroize();
        return Err("Could not save attachment".to_string());
    }

    // Dopisz wpis do indeksu metadanych.
    let mpath = meta_path(&state).map_err(|_| {
        key.zeroize();
        "Data directory not found".to_string()
    })?;
    let mut list = match read_meta(&key, &mpath) {
        Ok(l) => l,
        Err(_) => {
            key.zeroize();
            return Err("Could not read attachments index".to_string());
        }
    };
    list.push(AttachmentMeta {
        id: id.clone(),
        original_filename,
        mime_type,
        size_bytes,
        note_id,
        created_at: now_string(),
    });
    let res = write_meta(&key, &mpath, &list);
    key.zeroize();
    res.map_err(|_| "Could not update attachments index".to_string())?;

    Ok(id)
}

#[tauri::command]
pub async fn load_attachment(id: String, state: State<'_, VaultState>) -> Result<Vec<u8>, String> {
    let mut key = require_key(&state)?;

    let dir = match attachments_dir(&state) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };
    let path = attachment_file(&dir, &id).map_err(|_| {
        key.zeroize();
        "Invalid attachment id".to_string()
    })?;

    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            return Err("Attachment does not exist".to_string());
        }
    };

    let result = decrypt_file(&key, &data).map_err(|_| "Could not read attachment".to_string());
    key.zeroize();
    result
}

/// Zwraca gotowy `data:` URL (mime z indeksu + base64) — wydajniej niż przesyłać
/// surowe bajty do JS i kodować je tam (omija też przepełnienie stosu przy
/// `String.fromCharCode(...bytes)` dla dużych obrazów).
#[tauri::command]
pub async fn load_attachment_data_url(
    id: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    let mut key = require_key(&state)?;

    let mpath = match meta_path(&state) {
        Ok(p) => p,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };
    let mime = read_meta(&key, &mpath)
        .ok()
        .and_then(|list| list.into_iter().find(|a| a.id == id))
        .map(|a| a.mime_type)
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let dir = match attachments_dir(&state) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };
    let path = attachment_file(&dir, &id).map_err(|_| {
        key.zeroize();
        "Invalid attachment id".to_string()
    })?;
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            return Err("Attachment does not exist".to_string());
        }
    };
    let bytes = match decrypt_file(&key, &data) {
        Ok(b) => b,
        Err(_) => {
            key.zeroize();
            return Err("Could not read attachment".to_string());
        }
    };
    key.zeroize();

    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{mime};base64,{b64}"))
}

#[tauri::command]
pub async fn delete_attachment(id: String, state: State<'_, VaultState>) -> Result<(), String> {
    let mut key = require_key(&state)?;

    let dir = attachments_dir(&state).map_err(|_| {
        key.zeroize();
        "Data directory not found".to_string()
    })?;
    if let Ok(path) = attachment_file(&dir, &id) {
        let _ = std::fs::remove_file(path);
    }

    if let Ok(mpath) = meta_path(&state) {
        if let Ok(mut list) = read_meta(&key, &mpath) {
            let before = list.len();
            list.retain(|a| a.id != id);
            if list.len() != before {
                let _ = write_meta(&key, &mpath, &list);
            }
        }
    }
    key.zeroize();
    Ok(())
}

#[tauri::command]
pub async fn list_attachments_for_note(
    note_id: String,
    state: State<'_, VaultState>,
) -> Result<Vec<AttachmentMeta>, String> {
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
    Ok(list.into_iter().filter(|a| a.note_id == note_id).collect())
}

/// Usuwa załączniki, których notatka (`note_id`) już nie istnieje na dysku.
/// Wołane przy starcie aplikacji, żeby posprzątać po skasowanych notatkach.
#[tauri::command]
pub async fn cleanup_orphaned_attachments(state: State<'_, VaultState>) -> Result<u32, String> {
    let mut key = require_key(&state)?;

    let mpath = match meta_path(&state) {
        Ok(p) => p,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };
    let dir = match attachments_dir(&state) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };
    let ndir = match notes_dir_for(&state) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };

    let mut list = read_meta(&key, &mpath).unwrap_or_default();
    let mut keep: Vec<AttachmentMeta> = Vec::new();
    let mut removed = 0u32;

    for a in list.drain(..) {
        let note_exists = clean_id(&a.note_id) && ndir.join(format!("{}.rune", a.note_id)).exists();
        if note_exists {
            keep.push(a);
        } else {
            if let Ok(p) = attachment_file(&dir, &a.id) {
                let _ = std::fs::remove_file(p);
            }
            removed += 1;
        }
    }

    if removed > 0 {
        let _ = write_meta(&key, &mpath, &keep);
    }
    key.zeroize();
    Ok(removed)
}

/// Deszyfruje załącznik do pliku tymczasowego i otwiera go domyślną aplikacją
/// systemową (przez plugin opener). Używane dla PDF („Open").
#[tauri::command]
pub async fn open_attachment(
    id: String,
    app: tauri::AppHandle,
    state: State<'_, VaultState>,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    let mut key = require_key(&state)?;

    let mpath = meta_path(&state).map_err(|_| {
        key.zeroize();
        "Data directory not found".to_string()
    })?;
    let meta = read_meta(&key, &mpath)
        .ok()
        .and_then(|list| list.into_iter().find(|a| a.id == id));

    let dir = attachments_dir(&state).map_err(|_| {
        key.zeroize();
        "Data directory not found".to_string()
    })?;
    let path = attachment_file(&dir, &id).map_err(|_| {
        key.zeroize();
        "Invalid attachment id".to_string()
    })?;
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            return Err("Attachment does not exist".to_string());
        }
    };
    let bytes = match decrypt_file(&key, &data) {
        Ok(b) => b,
        Err(_) => {
            key.zeroize();
            return Err("Could not read attachment".to_string());
        }
    };
    key.zeroize();

    let ext = meta
        .as_ref()
        .map(|m| ext_for(&m.original_filename, &m.mime_type))
        .unwrap_or_else(|| "bin".to_string());
    let tmp = std::env::temp_dir().join(format!("rune-preview-{id}.{ext}"));
    std::fs::write(&tmp, &bytes).map_err(|_| "Could not write temp file".to_string())?;

    app.opener()
        .open_path(tmp.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Sprząta pliki podglądu `rune-preview-*` z katalogu temp (wołane przy starcie).
#[tauri::command]
pub async fn cleanup_temp_previews() -> Result<(), String> {
    let tmp = std::env::temp_dir();
    if let Ok(entries) = std::fs::read_dir(&tmp) {
        for e in entries.flatten() {
            if let Some(n) = e.file_name().to_str() {
                if n.starts_with("rune-preview-") {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
    }
    Ok(())
}

/// Usuwa wszystkie załączniki należące do danej notatki (wołane z `delete_note`).
/// Nie jest komendą — wywoływane wewnętrznie przy odblokowanym vaulcie.
pub fn purge_note_attachments(state: &State<'_, VaultState>, note_id: &str) {
    let mut key = match *state.key.lock() {
        Some(k) => k,
        None => return,
    };

    let (Ok(dir), Ok(mpath)) = (attachments_dir(state), meta_path(state)) else {
        key.zeroize();
        return;
    };

    let mut list = read_meta(&key, &mpath).unwrap_or_default();
    let mut keep: Vec<AttachmentMeta> = Vec::new();
    for a in list.drain(..) {
        if a.note_id == note_id {
            if let Ok(p) = attachment_file(&dir, &a.id) {
                let _ = std::fs::remove_file(p);
            }
        } else {
            keep.push(a);
        }
    }
    let _ = write_meta(&key, &mpath, &keep);
    key.zeroize();
}
