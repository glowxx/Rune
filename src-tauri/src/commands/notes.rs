//! Trwałe, zaszyfrowane notatki. Każda notatka to osobny plik
//! `%APPDATA%/rune/notes/{uuid}.rune`.
//!
//! Format pliku notatki (bez headera ArgonParams — klucz pochodzi z odblokowanego
//! vaultu trzymanego w `VaultState`):
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

/// Pełna notatka (szyfrowana w pliku). camelCase, bo wymienia się z JS.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteData {
    pub id: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub folder_id: Option<String>,
    pub pinned: bool,
    pub created_at: String, // ISO 8601
    pub updated_at: String, // ISO 8601
}

/// Metadane notatki — bez `content` (do zbudowania listy w sidebarze).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteMetadata {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub folder_id: Option<String>,
    pub pinned: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Wynik wyszukiwania — metadane + fragment treści wokół dopasowania.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub folder_id: Option<String>,
    pub pinned: bool,
    pub created_at: String,
    pub updated_at: String,
    pub snippet: String,
}

/// Notatka linkująca do danej (backlink) — metadane + linia kontekstu.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BacklinkHit {
    pub id: String,
    pub title: String,
    pub folder_id: Option<String>,
    pub pinned: bool,
    pub context: String,
}

/// Węzeł grafu notatek (jedna notatka).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub folder_id: Option<String>,
}

/// Krawędź grafu — `[[wikilink]]` ze źródła do celu (po id).
#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
}

/// Dane grafu notatek: węzły + krawędzie wyznaczone z `[[wikilinków]]`.
#[derive(Debug, Clone, Serialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// Katalog notatek aktywnej sesji: `notes/` normalnie, `notes_duress/` gdy
/// vault odblokowano hasłem duress. Dzięki temu wabik nie widzi (ani nie
/// nadpisuje) prawdziwych notatek.
pub(crate) fn active_notes_dir(state: &State<'_, VaultState>) -> Result<PathBuf, RuneError> {
    let sub = if *state.is_duress.lock() {
        "notes_duress"
    } else {
        "notes"
    };
    Ok(active_vault_dir(state)?.join(sub))
}

/// Ścieżka pliku notatki w danym katalogu, z zabezpieczeniem przed path
/// traversal — `id` pochodzi z frontendu, więc dopuszczamy tylko „czyste" id.
fn note_path_in(dir: &std::path::Path, id: &str) -> Result<PathBuf, RuneError> {
    let valid = !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid {
        return Err(RuneError::Format);
    }
    Ok(dir.join(format!("{id}.rune")))
}

/// Kopiuje klucz z odblokowanego vaultu lub zwraca błąd, gdy zablokowany.
fn require_key(state: &State<'_, VaultState>) -> Result<[u8; 32], String> {
    state
        .key
        .lock()
        .ok_or_else(|| "Vault is locked".to_string())
}

/// Składa bajty pliku notatki: `[12 nonce][ciphertext]`.
fn build_note_file(enc: &EncryptedFile) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + enc.ciphertext.len());
    out.extend_from_slice(&enc.nonce);
    out.extend_from_slice(&enc.ciphertext);
    out
}

/// Odszyfrowuje surowe bajty pliku notatki do `NoteData`.
fn decrypt_note_file(key: &[u8; 32], data: &[u8]) -> Result<NoteData, RuneError> {
    // 12 (nonce) + 16 (minimalny tag GCM) = 28 bajtów minimum.
    if data.len() < 28 {
        return Err(RuneError::Format);
    }
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&data[..12]);

    let enc = EncryptedFile {
        salt: [0u8; 32], // nieużywane dla notatek
        nonce,
        ciphertext: data[12..].to_vec(),
    };

    let mut plaintext = crypto::decrypt(key, &enc)?;
    let note: NoteData = serde_json::from_slice(&plaintext)?;
    plaintext.zeroize();
    Ok(note)
}

#[tauri::command]
pub async fn save_note(note: NoteData, state: State<'_, VaultState>) -> Result<(), String> {
    let mut key = require_key(&state)?;

    let mut plaintext = serde_json::to_vec(&note).map_err(|_| {
        key.zeroize();
        "Note serialization error".to_string()
    })?;

    let enc = crypto::encrypt(&key, &plaintext).map_err(|_| {
        key.zeroize();
        plaintext.zeroize();
        "Encryption error".to_string()
    })?;
    key.zeroize();
    plaintext.zeroize();

    let dir = active_notes_dir(&state).map_err(|_| "Data directory not found".to_string())?;
    std::fs::create_dir_all(&dir).map_err(|_| "Could not create notes directory".to_string())?;

    let path = note_path_in(&dir, &note.id).map_err(|_| "Invalid note id".to_string())?;
    fsutil::write_atomic(&path, &build_note_file(&enc))
        .map_err(|_| "Could not save note".to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn load_note(id: String, state: State<'_, VaultState>) -> Result<NoteData, String> {
    let mut key = require_key(&state)?;

    let dir = match active_notes_dir(&state) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };
    let path = note_path_in(&dir, &id).map_err(|_| {
        key.zeroize();
        "Invalid note id".to_string()
    })?;

    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            return Err("Note does not exist".to_string());
        }
    };

    let result = decrypt_note_file(&key, &data).map_err(|_| "Could not read note".to_string());
    key.zeroize();
    result
}

#[tauri::command]
pub async fn list_notes(
    app: tauri::AppHandle,
    state: State<'_, VaultState>,
) -> Result<Vec<NoteMetadata>, String> {
    let mut key = require_key(&state)?;

    let dir = match active_notes_dir(&state) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };

    let mut out: Vec<NoteMetadata> = Vec::new();
    let mut skipped: u32 = 0;

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("rune") {
                continue;
            }
            let data = match std::fs::read(&path) {
                Ok(d) => d,
                Err(_) => continue,
            };
            // Pojedynczy uszkodzony plik nie może zablokować całego vaultu —
            // pomijamy go (licząc), reszta ładuje się normalnie.
            if let Ok(note) = decrypt_note_file(&key, &data) {
                out.push(NoteMetadata {
                    id: note.id,
                    title: note.title,
                    tags: note.tags,
                    folder_id: note.folder_id,
                    pinned: note.pinned,
                    created_at: note.created_at,
                    updated_at: note.updated_at,
                });
            } else {
                skipped += 1;
            }
        }
    }
    key.zeroize();

    // Zgłoś frontendowi liczbę pominiętych (uszkodzonych) plików — pokaże toast.
    if skipped > 0 {
        use tauri::Emitter;
        let _ = app.emit("rune://notes-load-issues", skipped);
    }

    // Najnowsze (po updated_at) na górze.
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

/// Fragment treści wokół pierwszego dopasowania (z wielokropkami na brzegach).
/// Indeksowanie po znakach — nigdy nie panikuje na granicy znaku UTF-8.
fn snippet_around(content: &str, content_lower: &str, q: &str, width: usize) -> String {
    let Some(byte_pos) = content_lower.find(q) else {
        return leading_snippet(content, width);
    };
    // Pozycję liczymy w znakach; lowercase zwykle zachowuje ich liczbę.
    let char_pos = content_lower[..byte_pos].chars().count();
    let total = content.chars().count();
    let half = width / 2;
    let start = char_pos.saturating_sub(half);
    let end = (char_pos + q.chars().count() + half).min(total);

    let body: String = content.chars().skip(start).take(end - start).collect();
    let body = body.replace(['\n', '\r', '\t'], " ");
    let body = body.trim();
    let prefix = if start > 0 { "…" } else { "" };
    let suffix = if end < total { "…" } else { "" };
    format!("{prefix}{body}{suffix}")
}

/// Początek treści jako podgląd (gdy dopasowanie było w tytule lub tagach).
fn leading_snippet(content: &str, width: usize) -> String {
    let total = content.chars().count();
    let body: String = content.chars().take(width).collect();
    let body = body.replace(['\n', '\r', '\t'], " ");
    let body = body.trim();
    if total > width {
        format!("{body}…")
    } else {
        body.to_string()
    }
}

/// Escape minimalnego zestawu znaków HTML — snippet trafia do `{@html}` po
/// stronie frontendu, więc treść notatki nie może wstrzyknąć znaczników.
pub(crate) fn esc_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Pełnotekstowe wyszukiwanie. Najpierw próbuje indeksu FTS5 (zbudowanego po
/// odblokowaniu); zanim indeks będzie gotowy — albo gdy go brak — wraca do
/// wolniejszego skanu z deszyfracją każdego pliku (tytuł + tagi + treść).
#[tauri::command]
pub async fn search_notes(
    query: String,
    state: State<'_, VaultState>,
) -> Result<Vec<SearchHit>, String> {
    // Szybka ścieżka: indeks w pamięci RAM (jeśli zbudowany).
    if let Some(hits) = crate::commands::search::fts_search(&query, &state) {
        return Ok(hits);
    }

    let mut key = require_key(&state)?;

    let q = query.trim().to_lowercase();
    if q.is_empty() {
        key.zeroize();
        return Ok(Vec::new());
    }

    let dir = match active_notes_dir(&state) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };

    let mut out: Vec<SearchHit> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("rune") {
                continue;
            }
            let data = match std::fs::read(&path) {
                Ok(d) => d,
                Err(_) => continue,
            };
            // Pliki, których nie da się odszyfrować, pomijamy bez przerywania.
            let Ok(note) = decrypt_note_file(&key, &data) else {
                continue;
            };

            let title_match = note.title.to_lowercase().contains(&q);
            let tag_match = note.tags.iter().any(|t| t.to_lowercase().contains(&q));
            let content_lower = note.content.to_lowercase();
            let content_match = content_lower.contains(&q);

            if !(title_match || tag_match || content_match) {
                continue;
            }

            let snippet = if content_match {
                snippet_around(&note.content, &content_lower, &q, 80)
            } else {
                leading_snippet(&note.content, 80)
            };
            // Skan nie podświetla dopasowań — escapujemy całość dla `{@html}`.
            let snippet = esc_html(&snippet);

            out.push(SearchHit {
                id: note.id,
                title: note.title,
                tags: note.tags,
                folder_id: note.folder_id,
                pinned: note.pinned,
                created_at: note.created_at,
                updated_at: note.updated_at,
                snippet,
            });
        }
    }
    key.zeroize();

    // Najnowsze (po updated_at) na górze.
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

#[tauri::command]
pub async fn delete_note(id: String, state: State<'_, VaultState>) -> Result<(), String> {
    // Wymagamy odblokowanego vaultu, ale klucz nie jest potrzebny do usunięcia.
    if state.key.lock().is_none() {
        return Err("Vault is locked".to_string());
    }

    // Usuń powiązane załączniki razem z notatką (zanim zniknie plik notatki).
    crate::commands::attachments::purge_note_attachments(&state, &id);

    let dir = active_notes_dir(&state).map_err(|_| "Data directory not found".to_string())?;
    let path = note_path_in(&dir, &id).map_err(|_| "Invalid note id".to_string())?;

    match std::fs::remove_file(&path) {
        Ok(_) => Ok(()),
        // Brak pliku traktujemy jak sukces (idempotencja).
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("Could not delete note".to_string()),
    }
}

/// Wyciąga tytuły z `[[wikilinków]]` w tekście. Slicing po offsetach z `find`
/// (granice znaków UTF-8 są zachowane, bo „[[" i „]]" są ASCII).
fn extract_wikilink_titles(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find("]]") {
            let title = after[..end].trim();
            if !title.is_empty() {
                out.push(title.to_string());
            }
            rest = &after[end + 2..];
        } else {
            break;
        }
    }
    out
}

/// Odszyfrowuje wszystkie notatki z katalogu (pomijając nieczytelne pliki).
/// Wspólne dla `find_backlinks`, `get_graph_data` i budowy indeksu FTS.
pub(crate) fn load_all_notes(key: &[u8; 32], dir: &std::path::Path) -> Vec<NoteData> {
    let mut notes = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("rune") {
                continue;
            }
            let Ok(data) = std::fs::read(&path) else {
                continue;
            };
            if let Ok(note) = decrypt_note_file(key, &data) {
                notes.push(note);
            }
        }
    }
    notes
}

/// Linia kontekstu wokół linku, przycięta do rozsądnej długości.
fn trim_context(line: &str, width: usize) -> String {
    let line = line.trim();
    if line.chars().count() <= width {
        return line.to_string();
    }
    let body: String = line.chars().take(width).collect();
    format!("{}…", body.trim_end())
}

/// Notatki, które linkują `[[Tytuł]]` do podanego tytułu (backlinki).
#[tauri::command]
pub async fn find_backlinks(
    title: String,
    state: State<'_, VaultState>,
) -> Result<Vec<BacklinkHit>, String> {
    let mut key = require_key(&state)?;

    let target = title.trim().to_lowercase();
    if target.is_empty() {
        key.zeroize();
        return Ok(Vec::new());
    }

    let dir = match active_notes_dir(&state) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };

    let notes = load_all_notes(&key, &dir);
    key.zeroize();

    let mut out: Vec<BacklinkHit> = Vec::new();
    for note in notes {
        // Pomijamy odwołania notatki do samej siebie po tytule.
        if note.title.trim().to_lowercase() == target {
            continue;
        }
        let mut context: Option<String> = None;
        for line in note.content.lines() {
            let links = extract_wikilink_titles(line);
            if links.iter().any(|t| t.trim().to_lowercase() == target) {
                context = Some(trim_context(line, 120));
                break;
            }
        }
        if let Some(ctx) = context {
            out.push(BacklinkHit {
                id: note.id,
                title: note.title,
                folder_id: note.folder_id,
                pinned: note.pinned,
                context: ctx,
            });
        }
    }

    out.sort_by_key(|a| a.title.to_lowercase());
    Ok(out)
}

/// Dane grafu: każda notatka to węzeł, każdy `[[wikilink]]` to krawędź
/// (po zmapowaniu tytułu na id celu). Notatki bez połączeń pozostają węzłami.
#[tauri::command]
pub async fn get_graph_data(state: State<'_, VaultState>) -> Result<GraphData, String> {
    let mut key = require_key(&state)?;

    let dir = match active_notes_dir(&state) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };

    let notes = load_all_notes(&key, &dir);
    key.zeroize();

    // Mapa tytuł(lower) → id. Przy duplikatach tytułów wygrywa ostatni.
    let mut title_to_id: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for note in &notes {
        title_to_id.insert(note.title.trim().to_lowercase(), note.id.clone());
    }

    let nodes: Vec<GraphNode> = notes
        .iter()
        .map(|n| GraphNode {
            id: n.id.clone(),
            title: n.title.clone(),
            folder_id: n.folder_id.clone(),
        })
        .collect();

    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for note in &notes {
        for link_title in extract_wikilink_titles(&note.content) {
            let key_title = link_title.trim().to_lowercase();
            if let Some(target_id) = title_to_id.get(&key_title) {
                if *target_id == note.id {
                    continue; // pomijamy self-link
                }
                let pair = (note.id.clone(), target_id.clone());
                if seen.insert(pair) {
                    edges.push(GraphEdge {
                        source: note.id.clone(),
                        target: target_id.clone(),
                    });
                }
            }
        }
    }

    Ok(GraphData { nodes, edges })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> NoteData {
        NoteData {
            id: "note-abc_123".into(),
            title: "Note title".into(),
            content: "Body\nwith multiple\nlines and unicode ąćę".into(),
            tags: vec!["#crypto".into(), "#test".into()],
            folder_id: Some("dziennik".into()),
            pinned: true,
            created_at: "2026-06-19T10:00:00.000Z".into(),
            updated_at: "2026-06-19T11:30:00.000Z".into(),
        }
    }

    /// Pełny obieg: serializacja → szyfrowanie → format pliku → deszyfrowanie.
    #[test]
    fn note_file_round_trip() {
        let key = [7u8; 32];
        let note = sample();

        let plaintext = serde_json::to_vec(&note).unwrap();
        let enc = crypto::encrypt(&key, &plaintext).unwrap();
        let bytes = build_note_file(&enc);

        // Format: [12 nonce][ciphertext].
        assert_eq!(&bytes[..12], &enc.nonce);
        assert!(bytes.len() > 12 + 16); // nonce + co najmniej tag GCM

        let decoded = decrypt_note_file(&key, &bytes).unwrap();
        assert_eq!(decoded.id, note.id);
        assert_eq!(decoded.title, note.title);
        assert_eq!(decoded.content, note.content);
        assert_eq!(decoded.tags, note.tags);
        assert_eq!(decoded.folder_id, note.folder_id);
        assert_eq!(decoded.pinned, note.pinned);
        assert_eq!(decoded.updated_at, note.updated_at);
    }

    /// Zły klucz nie odszyfrowuje (uwierzytelnianie GCM).
    #[test]
    fn wrong_key_is_rejected() {
        let note = sample();
        let plaintext = serde_json::to_vec(&note).unwrap();
        let enc = crypto::encrypt(&[1u8; 32], &plaintext).unwrap();
        let bytes = build_note_file(&enc);
        assert!(decrypt_note_file(&[2u8; 32], &bytes).is_err());
    }

    /// Każde szyfrowanie używa świeżego nonce.
    #[test]
    fn nonce_is_fresh_each_time() {
        let key = [3u8; 32];
        let a = crypto::encrypt(&key, b"identyczny plaintext").unwrap();
        let b = crypto::encrypt(&key, b"identyczny plaintext").unwrap();
        assert_ne!(a.nonce, b.nonce);
    }

    /// `note_path_in` blokuje path traversal i akceptuje czyste id.
    #[test]
    fn note_path_guards_traversal() {
        let dir = std::path::Path::new("notes");
        assert!(note_path_in(dir, "../etc/passwd").is_err());
        assert!(note_path_in(dir, "a/b").is_err());
        assert!(note_path_in(dir, "a\\b").is_err());
        assert!(note_path_in(dir, "").is_err());
        assert!(note_path_in(dir, "ok-123_ID").is_ok());
    }

    /// Zbyt krótkie dane (poniżej nonce + tag) są odrzucane.
    #[test]
    fn truncated_file_is_rejected() {
        assert!(decrypt_note_file(&[0u8; 32], &[0u8; 10]).is_err());
    }

    /// Snippet centruje się na dopasowaniu i dokleja wielokropki na brzegach.
    #[test]
    fn snippet_centers_on_match() {
        let content =
            "The quick brown fox jumps over the lazy dog and keeps running far into the woods";
        let lower = content.to_lowercase();
        let s = snippet_around(content, &lower, "fox", 20);
        assert!(s.to_lowercase().contains("fox"));
        assert!(s.starts_with('…') && s.ends_with('…'));
        // Nowe linie zamienione na spacje, brak paniki na unicode.
        assert!(!s.contains('\n'));
    }

    /// Krótka treść w całości mieści się w snippetcie (bez wielokropka).
    #[test]
    fn snippet_short_content_no_ellipsis() {
        let content = "short body";
        let lower = content.to_lowercase();
        let s = snippet_around(content, &lower, "body", 80);
        assert_eq!(s, "short body");
    }

    /// Leading snippet przycina długą treść i dokleja wielokropek.
    #[test]
    fn leading_snippet_truncates() {
        let s = leading_snippet("hello world this is a fairly long note body indeed", 10);
        assert!(s.ends_with('…'));
        assert!(s.chars().count() <= 11); // 10 znaków + wielokropek
    }

    /// Ekstrakcja `[[wikilinków]]`: wiele linków, trim, pomijanie pustych.
    #[test]
    fn extracts_wikilink_titles() {
        let content =
            "See [[Project Atlas]] and [[ Meeting Notes ]].\nAlso [[]] empty and [[Daily]].";
        let titles = extract_wikilink_titles(content);
        assert_eq!(titles, vec!["Project Atlas", "Meeting Notes", "Daily"]);
    }

    /// Niedomknięty `[[` nie powoduje paniki i nie zwraca tytułu.
    #[test]
    fn unclosed_wikilink_is_ignored() {
        assert!(extract_wikilink_titles("broken [[start without end").is_empty());
        // Unicode wokół linku — brak paniki na granicy znaku.
        let titles = extract_wikilink_titles("ąćę [[Tytuł]] żółć");
        assert_eq!(titles, vec!["Tytuł"]);
    }
}
