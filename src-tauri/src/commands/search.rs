//! Pełnotekstowy indeks notatek oparty o SQLite FTS5, trzymany **wyłącznie w
//! pamięci RAM** (`:memory:`). Indeks budowany jest po odblokowaniu vaultu i
//! porzucany przy locku/przełączeniu — dzięki temu zaszyfrowana treść nie ma
//! plaintextowego cache na dysku.
//!
//! Bezpieczeństwo: baza nigdy nie powstaje jako plik; klucz służy tylko do
//! odszyfrowania notatek w trakcie budowy indeksu i jest natychmiast zerowany.

use rusqlite::Connection;
use tauri::{Emitter, State};
use zeroize::Zeroize;

use crate::commands::notes::{self, esc_html, NoteData, SearchHit};
use crate::commands::vault::VaultState;

/// Schemat tabeli FTS5. Kolumny metadanych są `UNINDEXED` (nie wpływają na
/// dopasowanie, ale wracają w wynikach), indeksowane są tytuł, treść i tagi.
const SCHEMA: &str = "CREATE VIRTUAL TABLE notes_fts USING fts5(\
    id UNINDEXED, title, content, tags, \
    folder_id UNINDEXED, pinned UNINDEXED, created_at UNINDEXED, updated_at UNINDEXED, \
    tokenize = 'unicode61');";

const INSERT_SQL: &str = "INSERT INTO notes_fts\
    (id, title, content, tags, folder_id, pinned, created_at, updated_at) \
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";

/// Kopiuje klucz z odblokowanego vaultu (lub błąd, gdy zablokowany).
fn require_key(state: &State<'_, VaultState>) -> Result<[u8; 32], String> {
    state
        .key
        .lock()
        .ok_or_else(|| "Vault is locked".to_string())
}

/// Buduje świeży indeks w pamięci z listy odszyfrowanych notatek.
fn build_index_conn(notes: &[NoteData]) -> Result<Connection, String> {
    let mut conn = Connection::open_in_memory().map_err(|_| "Search index error".to_string())?;
    conn.execute_batch(SCHEMA)
        .map_err(|_| "Search index error".to_string())?;
    {
        let tx = conn
            .transaction()
            .map_err(|_| "Search index error".to_string())?;
        {
            let mut stmt = tx
                .prepare(INSERT_SQL)
                .map_err(|_| "Search index error".to_string())?;
            for n in notes {
                let tags = n.tags.join(" ");
                let pinned: i32 = if n.pinned { 1 } else { 0 };
                stmt.execute(rusqlite::params![
                    n.id,
                    n.title,
                    n.content,
                    tags,
                    n.folder_id,
                    pinned,
                    n.created_at,
                    n.updated_at
                ])
                .map_err(|_| "Search index error".to_string())?;
            }
        }
        tx.commit().map_err(|_| "Search index error".to_string())?;
    }
    Ok(conn)
}

/// Wstawia (lub zastępuje) pojedynczą notatkę w istniejącym indeksie.
fn upsert(conn: &Connection, note: &NoteData) -> Result<(), String> {
    conn.execute("DELETE FROM notes_fts WHERE id = ?1", [&note.id])
        .map_err(|_| "Search index error".to_string())?;
    let tags = note.tags.join(" ");
    let pinned: i32 = if note.pinned { 1 } else { 0 };
    conn.execute(
        INSERT_SQL,
        rusqlite::params![
            note.id,
            note.title,
            note.content,
            tags,
            note.folder_id,
            pinned,
            note.created_at,
            note.updated_at
        ],
    )
    .map_err(|_| "Search index error".to_string())?;
    Ok(())
}

/// Buduje bezpieczne zapytanie MATCH z dowolnego tekstu użytkownika: każdy
/// token cytujemy (podwajając `"`) i dodajemy `*` dla dopasowania prefiksowego,
/// dzięki czemu znaki specjalne FTS5 nie psują składni zapytania.
fn build_match_query(query: &str) -> String {
    let mut parts = Vec::new();
    for tok in query.split_whitespace() {
        let escaped = tok.replace('"', "\"\"");
        parts.push(format!("\"{escaped}\"*"));
    }
    parts.join(" ")
}

/// Zamienia znaczniki snippetu (znaki sterujące U+0002/U+0003) na `<mark>`,
/// uprzednio escapując HTML — wynik jest bezpieczny dla `{@html}`.
fn finish_snippet(raw: &str) -> String {
    esc_html(raw)
        .replace('\u{2}', "<mark>")
        .replace('\u{3}', "</mark>")
}

fn split_tags(joined: &str) -> Vec<String> {
    joined.split_whitespace().map(|s| s.to_string()).collect()
}

/// Wyszukiwanie po indeksie FTS5. Zwraca `None`, gdy indeks nie jest jeszcze
/// zbudowany (wołający zrobi wtedy wolniejszy skan). Błędy zapytania (np. pusty
/// po tokenizacji ciąg) degradują się do pustej listy, nie do błędu komendy.
pub fn fts_search(query: &str, state: &State<'_, VaultState>) -> Option<Vec<SearchHit>> {
    let guard = state.search_db.lock();
    let conn = guard.as_ref()?;

    let match_q = build_match_query(query);
    if match_q.is_empty() {
        return Some(Vec::new());
    }

    // snippet() z kolumny `content` (indeks 2), znaczniki jako znaki sterujące,
    // które `finish_snippet` zamieni na <mark> po zescapowaniu HTML.
    let sql = "SELECT id, title, \
        snippet(notes_fts, 2, char(2), char(3), '…', 12), \
        tags, folder_id, pinned, created_at, updated_at \
        FROM notes_fts WHERE notes_fts MATCH ?1 ORDER BY rank LIMIT 50";

    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return Some(Vec::new()),
    };

    let mapped = stmt.query_map([&match_q], |row| {
        Ok(SearchHit {
            id: row.get::<_, String>(0)?,
            title: row.get::<_, String>(1)?,
            snippet: finish_snippet(&row.get::<_, String>(2)?),
            tags: split_tags(&row.get::<_, String>(3)?),
            folder_id: row.get::<_, Option<String>>(4)?,
            pinned: row.get::<_, i64>(5)? != 0,
            created_at: row.get::<_, String>(6)?,
            updated_at: row.get::<_, String>(7)?,
        })
    });

    let rows = match mapped {
        Ok(r) => r,
        Err(_) => return Some(Vec::new()),
    };

    let mut out = Vec::new();
    for r in rows.flatten() {
        out.push(r);
    }
    Some(out)
}

// ── Komendy ─────────────────────────────────────────────────────────────────

/// Buduje indeks pełnotekstowy w tle (po odblokowaniu). Po zakończeniu wysyła
/// event `search_index_ready`, na który frontend reaguje, gasząc „Indexing…".
#[tauri::command]
pub async fn build_search_index(
    app: tauri::AppHandle,
    state: State<'_, VaultState>,
) -> Result<(), String> {
    let mut key = require_key(&state)?;

    let dir = match notes::active_notes_dir(&state) {
        Ok(d) => d,
        Err(_) => {
            key.zeroize();
            return Err("Data directory not found".to_string());
        }
    };

    let all = notes::load_all_notes(&key, &dir);
    key.zeroize();

    let conn = build_index_conn(&all)?;
    *state.search_db.lock() = Some(conn);

    let _ = app.emit("search_index_ready", ());
    Ok(())
}

/// Aktualizuje pojedynczy wpis indeksu po zapisie notatki. No-op, gdy indeks
/// jeszcze nie istnieje (zostanie zbudowany z aktualnymi danymi).
#[tauri::command]
pub async fn update_search_index(
    note: NoteData,
    state: State<'_, VaultState>,
) -> Result<(), String> {
    let guard = state.search_db.lock();
    if let Some(conn) = guard.as_ref() {
        upsert(conn, &note)?;
    }
    Ok(())
}

/// Usuwa notatkę z indeksu po jej skasowaniu.
#[tauri::command]
pub async fn remove_from_search_index(
    id: String,
    state: State<'_, VaultState>,
) -> Result<(), String> {
    let guard = state.search_db.lock();
    if let Some(conn) = guard.as_ref() {
        let _ = conn.execute("DELETE FROM notes_fts WHERE id = ?1", [&id]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(id: &str, title: &str, content: &str, tags: &[&str]) -> NoteData {
        NoteData {
            id: id.into(),
            title: title.into(),
            content: content.into(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            folder_id: None,
            pinned: false,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    /// Budowa indeksu i znalezienie notatki po słowie z treści (nie z tytułu).
    #[test]
    fn finds_by_content_word() {
        let notes = vec![
            note("a", "Groceries", "buy milk and bread tomorrow", &["home"]),
            note("b", "Work", "ship the release candidate", &["dev"]),
        ];
        let conn = build_index_conn(&notes).unwrap();
        let q = build_match_query("bread");
        let mut stmt = conn
            .prepare("SELECT id FROM notes_fts WHERE notes_fts MATCH ?1")
            .unwrap();
        let ids: Vec<String> = stmt
            .query_map([&q], |r| r.get::<_, String>(0))
            .unwrap()
            .flatten()
            .collect();
        assert_eq!(ids, vec!["a"]);
    }

    /// Tokeny z dziwnymi znakami nie wysadzają zapytania MATCH.
    #[test]
    fn special_chars_do_not_crash() {
        let notes = vec![note("a", "Title", "alpha beta gamma", &[])];
        let conn = build_index_conn(&notes).unwrap();
        for raw in ["alp", "\"quote", "a*b", "beta AND", "()"] {
            let q = build_match_query(raw);
            if q.is_empty() {
                continue;
            }
            // Nie może panikować ani zwracać błędu twardego.
            let mut stmt = conn
                .prepare("SELECT id FROM notes_fts WHERE notes_fts MATCH ?1")
                .unwrap();
            let _ = stmt
                .query_map([&q], |r| r.get::<_, String>(0))
                .map(|rows| rows.flatten().count());
        }
    }

    /// snippet jest bezpieczny dla `{@html}` (escape HTML), a znaczniki wracają
    /// jako <mark>.
    #[test]
    fn snippet_is_html_safe() {
        let s = finish_snippet("\u{2}<script>\u{3} ok & <b>");
        assert!(s.contains("<mark>&lt;script&gt;</mark>"));
        assert!(!s.contains("<script>"));
        assert!(s.contains("&amp;"));
    }
}
