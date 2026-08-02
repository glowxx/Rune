//! Eksport/import całego vaultu do przenośnego pliku `.vault` oraz eksport
//! pojedynczej notatki do DOCX / HTML (do druku PDF).
//!
//! Format `.vault`:
//! ```text
//! [8 magic "RUNEVLT\0"][4 version LE][4 header_len LE][ExportHeader JSON]
//! [32 salt][12 nonce][ciphertext+tag]   <- payload (AES-256-GCM, hasło eksportu)
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use docx_rs::*;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;
use zeroize::Zeroize;

use crate::commands::attachments::AttachmentMeta;
use crate::commands::crypto;
use crate::commands::folders::FolderData;
use crate::commands::fsutil;
use crate::commands::notes::NoteData;
use crate::commands::timeutil;
use crate::commands::vault::{active_vault_dir, VaultState};
use crate::commands::whiteboard::WhiteboardMeta;
use crate::error::RuneError;
use crate::models::{ArgonParams, EncryptedFile};

const MAGIC: &[u8; 8] = b"RUNEVLT\0";
const EXPORT_VERSION: u32 = 1;
const RUNE_VERSION: &str = "0.1.0";

fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

fn validate_export_destination(path: &Path, expected_extension: &str) -> Result<(), String> {
    if !path.is_absolute() || !has_extension(path, expected_extension) {
        return Err(format!(
            "Destination must be an absolute .{expected_extension} file path"
        ));
    }
    let Some(parent) = path.parent() else {
        return Err("Destination directory not found".to_string());
    };
    if !parent.is_dir() || (path.exists() && !path.is_file()) {
        return Err("Destination directory not found".to_string());
    }
    Ok(())
}

fn validate_import_source(path: &Path) -> Result<(), String> {
    if !path.is_absolute() || !has_extension(path, "vault") || !path.is_file() {
        return Err("Source must be an existing .vault file".to_string());
    }
    Ok(())
}

// ── Struktury pliku eksportu ────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct ExportHeader {
    version: u32,
    created_at: String,
    rune_version: String,
    note_count: u32,
    attachment_count: u32,
    argon_params: ArgonParams,
}

#[derive(Serialize, Deserialize)]
struct NoteExportAttachment {
    id: String,
    original_filename: String,
    mime_type: String,
    size_bytes: u64,
}

#[derive(Serialize, Deserialize)]
struct NoteExport {
    id: String,
    title: String,
    content: String,
    tags: Vec<String>,
    folder_id: Option<String>,
    pinned: bool,
    created_at: String,
    updated_at: String,
    attachments: Vec<NoteExportAttachment>,
}

/// Tablica Excalidraw w eksporcie (scena jako tekst JSON — bez base64).
#[derive(Serialize, Deserialize)]
struct WhiteboardExport {
    id: String,
    note_id: String,
    title: String,
    created_at: u64,
    updated_at: u64,
    scene: String,
}

#[derive(Serialize, Deserialize)]
struct ExportPayload {
    folders: Vec<FolderData>,
    notes: Vec<NoteExport>,
    attachments_data: HashMap<String, String>, // id -> base64
    // `default` — starsze pliki .vault (bez tablic) wciąż się importują.
    #[serde(default)]
    whiteboards: Vec<WhiteboardExport>,
}

/// Wynik importu (camelCase — do UI).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub note_count: u32,
    pub folder_count: u32,
    pub attachment_count: u32,
}

// ── Pomocnicze: ścieżki aktywnej sesji (świadome trybu duress) ──────────────

fn is_duress(state: &State<'_, VaultState>) -> bool {
    *state.is_duress.lock()
}

fn notes_dir(state: &State<'_, VaultState>) -> Result<PathBuf, RuneError> {
    let sub = if is_duress(state) {
        "notes_duress"
    } else {
        "notes"
    };
    Ok(active_vault_dir(state)?.join(sub))
}

fn attachments_dir(state: &State<'_, VaultState>) -> Result<PathBuf, RuneError> {
    let sub = if is_duress(state) {
        "attachments_duress"
    } else {
        "attachments"
    };
    Ok(active_vault_dir(state)?.join(sub))
}

fn folders_file(state: &State<'_, VaultState>) -> Result<PathBuf, RuneError> {
    let name = if is_duress(state) {
        "folders_duress.rune"
    } else {
        "folders.rune"
    };
    Ok(active_vault_dir(state)?.join(name))
}

fn attach_meta_file(state: &State<'_, VaultState>) -> Result<PathBuf, RuneError> {
    let name = if is_duress(state) {
        "attachments_meta_duress.rune"
    } else {
        "attachments_meta.rune"
    };
    Ok(active_vault_dir(state)?.join(name))
}

fn whiteboards_dir(state: &State<'_, VaultState>) -> Result<PathBuf, RuneError> {
    let sub = if is_duress(state) {
        "whiteboards_duress"
    } else {
        "whiteboards"
    };
    Ok(active_vault_dir(state)?.join(sub))
}

fn wb_meta_file(state: &State<'_, VaultState>) -> Result<PathBuf, RuneError> {
    let name = if is_duress(state) {
        "whiteboards_meta_duress.rune"
    } else {
        "whiteboards_meta.rune"
    };
    Ok(active_vault_dir(state)?.join(name))
}

fn require_key(state: &State<'_, VaultState>) -> Result<[u8; 32], String> {
    state
        .key
        .lock()
        .ok_or_else(|| "Vault is locked".to_string())
}

fn clean_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

// ── Pomocnicze: szyfrowanie blobów `[12 nonce][ct]` ─────────────────────────

fn decrypt_blob(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, RuneError> {
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

fn encrypt_blob(key: &[u8; 32], plain: &[u8]) -> Result<Vec<u8>, RuneError> {
    let enc = crypto::encrypt(key, plain)?;
    let mut out = Vec::with_capacity(12 + enc.ciphertext.len());
    out.extend_from_slice(&enc.nonce);
    out.extend_from_slice(&enc.ciphertext);
    Ok(out)
}

// ── Odczyt aktywnej sesji ───────────────────────────────────────────────────

fn read_folders(key: &[u8; 32], state: &State<'_, VaultState>) -> Vec<FolderData> {
    let Ok(path) = folders_file(state) else {
        return Vec::new();
    };
    let Ok(data) = std::fs::read(&path) else {
        return Vec::new();
    };
    decrypt_blob(key, &data)
        .ok()
        .and_then(|p| serde_json::from_slice(&p).ok())
        .unwrap_or_default()
}

fn read_all_notes(key: &[u8; 32], state: &State<'_, VaultState>) -> Vec<NoteData> {
    let mut out = Vec::new();
    let Ok(dir) = notes_dir(state) else {
        return out;
    };
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("rune") {
                continue;
            }
            if let Ok(data) = std::fs::read(&path) {
                if let Ok(plain) = decrypt_blob(key, &data) {
                    if let Ok(note) = serde_json::from_slice::<NoteData>(&plain) {
                        out.push(note);
                    }
                }
            }
        }
    }
    out
}

fn read_note(key: &[u8; 32], state: &State<'_, VaultState>, id: &str) -> Option<NoteData> {
    if !clean_id(id) {
        return None;
    }
    let dir = notes_dir(state).ok()?;
    let data = std::fs::read(dir.join(format!("{id}.rune"))).ok()?;
    let plain = decrypt_blob(key, &data).ok()?;
    serde_json::from_slice(&plain).ok()
}

fn read_attach_meta(key: &[u8; 32], state: &State<'_, VaultState>) -> Vec<AttachmentMeta> {
    let Ok(path) = attach_meta_file(state) else {
        return Vec::new();
    };
    let Ok(data) = std::fs::read(&path) else {
        return Vec::new();
    };
    decrypt_blob(key, &data)
        .ok()
        .and_then(|p| serde_json::from_slice(&p).ok())
        .unwrap_or_default()
}

fn read_attach_bytes(key: &[u8; 32], dir: &Path, id: &str) -> Option<Vec<u8>> {
    if !clean_id(id) {
        return None;
    }
    let data = std::fs::read(dir.join(format!("{id}.rune"))).ok()?;
    decrypt_blob(key, &data).ok()
}

fn read_wb_meta(key: &[u8; 32], state: &State<'_, VaultState>) -> Vec<WhiteboardMeta> {
    let Ok(path) = wb_meta_file(state) else {
        return Vec::new();
    };
    let Ok(data) = std::fs::read(&path) else {
        return Vec::new();
    };
    decrypt_blob(key, &data)
        .ok()
        .and_then(|p| serde_json::from_slice(&p).ok())
        .unwrap_or_default()
}

fn read_wb_scene(key: &[u8; 32], dir: &Path, id: &str) -> Option<String> {
    if !clean_id(id) {
        return None;
    }
    let data = std::fs::read(dir.join(format!("{id}.rune"))).ok()?;
    String::from_utf8(decrypt_blob(key, &data).ok()?).ok()
}

// ── Zapis (import) ──────────────────────────────────────────────────────────

fn write_folders(
    key: &[u8; 32],
    state: &State<'_, VaultState>,
    list: &[FolderData],
) -> Result<(), String> {
    let path = folders_file(state).map_err(|_| "Data directory not found".to_string())?;
    let plain = serde_json::to_vec(list).map_err(|_| "Serialization error".to_string())?;
    let bytes = encrypt_blob(key, &plain).map_err(|_| "Encryption error".to_string())?;
    fsutil::write_atomic(&path, &bytes).map_err(|_| "Could not write folders".to_string())
}

fn write_attach_meta(
    key: &[u8; 32],
    state: &State<'_, VaultState>,
    list: &[AttachmentMeta],
) -> Result<(), String> {
    let path = attach_meta_file(state).map_err(|_| "Data directory not found".to_string())?;
    let plain = serde_json::to_vec(list).map_err(|_| "Serialization error".to_string())?;
    let bytes = encrypt_blob(key, &plain).map_err(|_| "Encryption error".to_string())?;
    fsutil::write_atomic(&path, &bytes).map_err(|_| "Could not write attachments index".to_string())
}

fn write_note(key: &[u8; 32], dir: &Path, note: &NoteData) -> Result<(), String> {
    let plain = serde_json::to_vec(note).map_err(|_| "Serialization error".to_string())?;
    let bytes = encrypt_blob(key, &plain).map_err(|_| "Encryption error".to_string())?;
    fsutil::write_atomic(&dir.join(format!("{}.rune", note.id)), &bytes)
        .map_err(|_| "Could not write note".to_string())
}

fn write_attachment(key: &[u8; 32], dir: &Path, id: &str, bytes: &[u8]) -> Result<(), String> {
    let enc = encrypt_blob(key, bytes).map_err(|_| "Encryption error".to_string())?;
    fsutil::write_atomic(&dir.join(format!("{id}.rune")), &enc)
        .map_err(|_| "Could not write attachment".to_string())
}

fn write_wb_meta(
    key: &[u8; 32],
    state: &State<'_, VaultState>,
    list: &[WhiteboardMeta],
) -> Result<(), String> {
    let path = wb_meta_file(state).map_err(|_| "Data directory not found".to_string())?;
    let plain = serde_json::to_vec(list).map_err(|_| "Serialization error".to_string())?;
    let bytes = encrypt_blob(key, &plain).map_err(|_| "Encryption error".to_string())?;
    fsutil::write_atomic(&path, &bytes).map_err(|_| "Could not write whiteboards index".to_string())
}

fn write_wb_scene(key: &[u8; 32], dir: &Path, id: &str, scene: &str) -> Result<(), String> {
    let enc = encrypt_blob(key, scene.as_bytes()).map_err(|_| "Encryption error".to_string())?;
    fsutil::write_atomic(&dir.join(format!("{id}.rune")), &enc)
        .map_err(|_| "Could not write whiteboard".to_string())
}

// ── Komendy: export / import vaultu ─────────────────────────────────────────

#[tauri::command]
pub async fn export_vault(
    mut export_password: String,
    dest_path: String,
    state: State<'_, VaultState>,
) -> Result<u32, String> {
    let dest_path = PathBuf::from(dest_path);
    if let Err(error) = validate_export_destination(&dest_path, "vault") {
        export_password.zeroize();
        return Err(error);
    }

    let mut key = match require_key(&state) {
        Ok(k) => k,
        Err(e) => {
            export_password.zeroize();
            return Err(e);
        }
    };

    let folders = read_folders(&key, &state);
    let notes_full = read_all_notes(&key, &state);
    let metas = read_attach_meta(&key, &state);

    let attach_dir = attachments_dir(&state).map_err(|_| {
        key.zeroize();
        "Data directory not found".to_string()
    })?;

    let mut attachments_data: HashMap<String, String> = HashMap::new();
    for m in &metas {
        if let Some(bytes) = read_attach_bytes(&key, &attach_dir, &m.id) {
            attachments_data.insert(
                m.id.clone(),
                base64::engine::general_purpose::STANDARD.encode(&bytes),
            );
        }
    }

    let notes: Vec<NoteExport> = notes_full
        .iter()
        .map(|n| {
            let attachments = metas
                .iter()
                .filter(|m| m.note_id == n.id)
                .map(|m| NoteExportAttachment {
                    id: m.id.clone(),
                    original_filename: m.original_filename.clone(),
                    mime_type: m.mime_type.clone(),
                    size_bytes: m.size_bytes,
                })
                .collect();
            NoteExport {
                id: n.id.clone(),
                title: n.title.clone(),
                content: n.content.clone(),
                tags: n.tags.clone(),
                folder_id: n.folder_id.clone(),
                pinned: n.pinned,
                created_at: n.created_at.clone(),
                updated_at: n.updated_at.clone(),
                attachments,
            }
        })
        .collect();

    let note_count = notes.len() as u32;
    let attachment_count = attachments_data.len() as u32;

    // Tablice Excalidraw (metadane + scena każdej).
    let wb_metas = read_wb_meta(&key, &state);
    let whiteboards: Vec<WhiteboardExport> = match whiteboards_dir(&state) {
        Ok(wb_dir) => wb_metas
            .iter()
            .filter_map(|m| {
                let scene = read_wb_scene(&key, &wb_dir, &m.id)?;
                Some(WhiteboardExport {
                    id: m.id.clone(),
                    note_id: m.note_id.clone(),
                    title: m.title.clone(),
                    created_at: m.created_at,
                    updated_at: m.updated_at,
                    scene,
                })
            })
            .collect(),
        Err(_) => Vec::new(),
    };

    let payload = ExportPayload {
        folders,
        notes,
        attachments_data,
        whiteboards,
    };
    let mut payload_json = serde_json::to_vec(&payload).map_err(|_| {
        key.zeroize();
        "Serialization error".to_string()
    })?;

    let argon = state
        .argon_params
        .lock()
        .clone()
        .unwrap_or_else(ArgonParams::balanced);

    let mut salt = [0u8; 32];
    OsRng.fill_bytes(&mut salt);
    let mut ekey = match crypto::derive_key(&export_password, &salt, &argon) {
        Ok(k) => k,
        Err(_) => {
            export_password.zeroize();
            payload_json.zeroize();
            key.zeroize();
            return Err("Key derivation error".to_string());
        }
    };
    export_password.zeroize();
    key.zeroize();

    let mut enc = match crypto::encrypt(&ekey, &payload_json) {
        Ok(e) => e,
        Err(_) => {
            ekey.zeroize();
            payload_json.zeroize();
            return Err("Encryption error".to_string());
        }
    };
    payload_json.zeroize();
    ekey.zeroize();
    enc.salt = salt;

    let header = ExportHeader {
        version: EXPORT_VERSION,
        created_at: timeutil::now_iso(),
        rune_version: RUNE_VERSION.to_string(),
        note_count,
        attachment_count,
        argon_params: argon,
    };
    let header_json = serde_json::to_vec(&header).map_err(|_| "Serialization error".to_string())?;

    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&EXPORT_VERSION.to_le_bytes());
    out.extend_from_slice(&(header_json.len() as u32).to_le_bytes());
    out.extend_from_slice(&header_json);
    out.extend_from_slice(&enc.to_bytes());

    fsutil::write_atomic(&dest_path, &out)
        .map_err(|_| "Could not write export file".to_string())?;
    Ok(note_count)
}

#[tauri::command]
pub async fn import_vault(
    mut import_password: String,
    src_path: String,
    state: State<'_, VaultState>,
) -> Result<ImportResult, String> {
    let src_path = PathBuf::from(src_path);
    if let Err(error) = validate_import_source(&src_path) {
        import_password.zeroize();
        return Err(error);
    }

    let mut key = match require_key(&state) {
        Ok(k) => k,
        Err(e) => {
            import_password.zeroize();
            return Err(e);
        }
    };

    let data = match std::fs::read(&src_path) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            import_password.zeroize();
            return Err("Could not read export file".to_string());
        }
    };
    if data.len() < 16 || &data[..8] != MAGIC {
        key.zeroize();
        import_password.zeroize();
        return Err("Not a valid .vault file".to_string());
    }

    let header_len = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;
    let hstart = 16usize;
    let hend = match hstart.checked_add(header_len) {
        Some(e) if e <= data.len() => e,
        _ => {
            key.zeroize();
            import_password.zeroize();
            return Err("Corrupted export file".to_string());
        }
    };
    let header: ExportHeader = match serde_json::from_slice(&data[hstart..hend]) {
        Ok(h) => h,
        Err(_) => {
            key.zeroize();
            import_password.zeroize();
            return Err("Corrupted export file".to_string());
        }
    };

    let enc = match EncryptedFile::from_bytes(&data[hend..]) {
        Ok(e) => e,
        Err(_) => {
            key.zeroize();
            import_password.zeroize();
            return Err("Corrupted export file".to_string());
        }
    };

    let mut ekey = match crypto::derive_key(&import_password, &enc.salt, &header.argon_params) {
        Ok(k) => k,
        Err(_) => {
            key.zeroize();
            import_password.zeroize();
            return Err("Key derivation error".to_string());
        }
    };
    import_password.zeroize();

    let mut payload_json = match crypto::decrypt(&ekey, &enc) {
        Ok(p) => p,
        Err(_) => {
            ekey.zeroize();
            key.zeroize();
            return Err("Invalid password or corrupted export file".to_string());
        }
    };
    ekey.zeroize();

    let payload: ExportPayload = match serde_json::from_slice(&payload_json) {
        Ok(p) => p,
        Err(_) => {
            payload_json.zeroize();
            key.zeroize();
            return Err("Invalid password or corrupted export file".to_string());
        }
    };
    payload_json.zeroize();

    // ── Merge: nowe UUID dla wszystkiego, remap referencji ──────────────────
    let mut existing_folders = read_folders(&key, &state);
    let max_order = existing_folders.iter().map(|f| f.order).max().unwrap_or(-1);
    let mut folder_map: HashMap<String, String> = HashMap::new();
    for (i, f) in payload.folders.iter().enumerate() {
        let nid = Uuid::new_v4().to_string();
        folder_map.insert(f.id.clone(), nid.clone());
        existing_folders.push(FolderData {
            id: nid,
            name: f.name.clone(),
            created_at: f.created_at.clone(),
            order: max_order + 1 + i as i32,
        });
    }
    let folder_count = payload.folders.len() as u32;
    write_folders(&key, &state, &existing_folders).inspect_err(|_| {
        key.zeroize();
    })?;

    let ndir = notes_dir(&state).map_err(|_| {
        key.zeroize();
        "Data directory not found".to_string()
    })?;
    std::fs::create_dir_all(&ndir).ok();
    let adir = attachments_dir(&state).map_err(|_| {
        key.zeroize();
        "Data directory not found".to_string()
    })?;
    std::fs::create_dir_all(&adir).ok();

    let mut existing_meta = read_attach_meta(&key, &state);
    let mut note_count = 0u32;
    let mut attachment_count = 0u32;
    // Mapa stare id notatki → nowe (do remapowania referencji tablic).
    let mut note_map: HashMap<String, String> = HashMap::new();

    for n in payload.notes {
        let new_note_id = Uuid::new_v4().to_string();
        note_map.insert(n.id.clone(), new_note_id.clone());
        let mut content = n.content;

        for a in &n.attachments {
            if let Some(b64) = payload.attachments_data.get(&a.id) {
                if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64) {
                    let new_aid = Uuid::new_v4().to_string();
                    if write_attachment(&key, &adir, &new_aid, &bytes).is_ok() {
                        content = content.replace(
                            &format!("attachment://{}", a.id),
                            &format!("attachment://{new_aid}"),
                        );
                        existing_meta.push(AttachmentMeta {
                            id: new_aid,
                            original_filename: a.original_filename.clone(),
                            mime_type: a.mime_type.clone(),
                            size_bytes: a.size_bytes,
                            note_id: new_note_id.clone(),
                            created_at: timeutil::now_iso(),
                        });
                        attachment_count += 1;
                    }
                }
            }
        }

        let new_folder = n
            .folder_id
            .as_ref()
            .and_then(|fid| folder_map.get(fid).cloned());
        let note = NoteData {
            id: new_note_id,
            title: n.title,
            content,
            tags: n.tags,
            folder_id: new_folder,
            pinned: n.pinned,
            created_at: n.created_at,
            updated_at: n.updated_at,
        };
        if write_note(&key, &ndir, &note).is_ok() {
            note_count += 1;
        }
    }

    let _ = write_attach_meta(&key, &state, &existing_meta);

    // ── Tablice Excalidraw: nowe id, remap note_id przez note_map ────────────
    if !payload.whiteboards.is_empty() {
        if let Ok(wb_dir) = whiteboards_dir(&state) {
            std::fs::create_dir_all(&wb_dir).ok();
            let mut existing_wb = read_wb_meta(&key, &state);
            for wb in payload.whiteboards {
                let Some(new_note) = note_map.get(&wb.note_id) else {
                    continue; // tablica osierocona (brak notatki w eksporcie)
                };
                let new_id = Uuid::new_v4().to_string();
                if write_wb_scene(&key, &wb_dir, &new_id, &wb.scene).is_ok() {
                    existing_wb.push(WhiteboardMeta {
                        id: new_id,
                        note_id: new_note.clone(),
                        title: wb.title,
                        created_at: wb.created_at,
                        updated_at: wb.updated_at,
                    });
                }
            }
            let _ = write_wb_meta(&key, &state, &existing_wb);
        }
    }

    key.zeroize();

    Ok(ImportResult {
        note_count,
        folder_count,
        attachment_count,
    })
}

// ── Eksport pojedynczej notatki: DOCX ───────────────────────────────────────

fn flush_buf(out: &mut Vec<(String, u8)>, buf: &mut String) {
    if !buf.is_empty() {
        out.push((std::mem::take(buf), 0));
    }
}

fn find_marker(chars: &[char], start: usize, marker: &[char]) -> Option<usize> {
    let mut i = start;
    while i + marker.len() <= chars.len() {
        if chars[i..i + marker.len()] == *marker {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Dzieli linię na segmenty: 0 plain, 1 bold, 2 italic, 3 code.
fn parse_inline(s: &str) -> Vec<(String, u8)> {
    let chars: Vec<char> = s.chars().collect();
    let mut out: Vec<(String, u8)> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
            if let Some(end) = find_marker(&chars, i + 2, &['*', '*']) {
                flush_buf(&mut out, &mut buf);
                out.push((chars[i + 2..end].iter().collect(), 1));
                i = end + 2;
                continue;
            }
        }
        if chars[i] == '*' {
            if let Some(end) = find_marker(&chars, i + 1, &['*']) {
                flush_buf(&mut out, &mut buf);
                out.push((chars[i + 1..end].iter().collect(), 2));
                i = end + 1;
                continue;
            }
        }
        if chars[i] == '`' {
            if let Some(end) = find_marker(&chars, i + 1, &['`']) {
                flush_buf(&mut out, &mut buf);
                out.push((chars[i + 1..end].iter().collect(), 3));
                i = end + 1;
                continue;
            }
        }
        buf.push(chars[i]);
        i += 1;
    }
    flush_buf(&mut out, &mut buf);
    if out.is_empty() {
        out.push((String::new(), 0));
    }
    out
}

fn heading_para(text: &str, size: usize) -> Paragraph {
    Paragraph::new().add_run(Run::new().add_text(text).bold().size(size))
}

fn inline_para(text: &str) -> Paragraph {
    let mut p = Paragraph::new();
    for (seg, style) in parse_inline(text) {
        let mut r = Run::new().add_text(seg);
        match style {
            1 => r = r.bold(),
            2 => r = r.italic(),
            3 => r = r.fonts(RunFonts::new().ascii("Consolas")),
            _ => {}
        }
        p = p.add_run(r);
    }
    p
}

fn build_docx(title: &str, content: &str) -> Docx {
    let mut docx = Docx::new().add_paragraph(heading_para(title, 40));
    let mut in_code = false;

    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            docx = docx.add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text(line)
                        .fonts(RunFonts::new().ascii("Consolas"))
                        .size(20),
                ),
            );
            continue;
        }
        if let Some(rest) = line.strip_prefix("### ") {
            docx = docx.add_paragraph(heading_para(rest, 28));
        } else if let Some(rest) = line.strip_prefix("## ") {
            docx = docx.add_paragraph(heading_para(rest, 32));
        } else if let Some(rest) = line.strip_prefix("# ") {
            docx = docx.add_paragraph(heading_para(rest, 40));
        } else {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
            {
                docx = docx.add_paragraph(inline_para(&format!("•  {rest}")));
            } else if line.trim().is_empty() {
                docx = docx.add_paragraph(Paragraph::new());
            } else {
                docx = docx.add_paragraph(inline_para(line));
            }
        }
    }
    docx
}

#[tauri::command]
pub async fn export_note_to_docx(
    note_id: String,
    dest_path: String,
    state: State<'_, VaultState>,
) -> Result<(), String> {
    let dest_path = PathBuf::from(dest_path);
    validate_export_destination(&dest_path, "docx")?;

    let mut key = require_key(&state)?;
    let note = read_note(&key, &state, &note_id);
    key.zeroize();
    let note = note.ok_or_else(|| "Note not found".to_string())?;

    let file =
        std::fs::File::create(&dest_path).map_err(|_| "Could not create file".to_string())?;
    build_docx(&note.title, &note.content)
        .build()
        .pack(file)
        .map_err(|_| "Could not write DOCX".to_string())?;
    Ok(())
}

/// Zapisuje gotowy HTML notatki do pliku tymczasowego i otwiera go w domyślnej
/// przeglądarce — użytkownik drukuje do PDF (Ctrl+P → Save as PDF). HTML jest
/// renderowany po stronie frontendu (tam żyje `marked`).
#[tauri::command]
pub async fn export_note_html(html: String, app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let id = Uuid::new_v4().to_string();
    let tmp = std::env::temp_dir().join(format!("rune-export-{id}.html"));
    std::fs::write(&tmp, html.as_bytes()).map_err(|_| "Could not write temp file".to_string())?;
    app.opener()
        .open_path(tmp.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_test_dir() -> PathBuf {
        std::env::temp_dir().join(format!("rune-export-test-{}", Uuid::new_v4()))
    }

    #[test]
    fn export_destination_requires_absolute_expected_extension() {
        assert!(validate_export_destination(Path::new("backup.vault"), "vault").is_err());

        let dir = temp_test_dir();
        std::fs::create_dir_all(&dir).unwrap();
        assert!(validate_export_destination(&dir.join("backup.txt"), "vault").is_err());
        assert!(validate_export_destination(&dir.join("backup.VAULT"), "vault").is_ok());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn import_source_must_be_an_existing_vault_file() {
        let dir = temp_test_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("backup.vault");

        assert!(validate_import_source(&source).is_err());
        std::fs::write(&source, b"test").unwrap();
        assert!(validate_import_source(&source).is_ok());
        assert!(validate_import_source(&dir.join("backup.txt")).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
