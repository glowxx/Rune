//! Widok kalendarza: zbiera notatki z datami do rozmieszczenia na siatce dni.
//!
//! Każda notatka trafia na kalendarz po swojej dacie utworzenia (`created_at`)
//! oraz po wszystkich datach `YYYY-MM-DD` znalezionych w treści (także w formie
//! `[[YYYY-MM-DD]]`). Skanowanie dat i liczenie zadań robimy po stronie Rusta,
//! żeby frontend nie musiał deszyfrować ani ładować treści wszystkich notatek.

use serde::Serialize;
use tauri::State;
use zeroize::Zeroize;

use crate::commands::notes::{active_notes_dir, load_all_notes};
use crate::commands::vault::VaultState;

/// Notatka w ujęciu kalendarza (camelCase — wymiana z JS).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarNote {
    pub id: String,
    pub title: String,
    pub created_at: String, // ISO 8601 (jak w NoteData)
    pub dates: Vec<String>, // YYYY-MM-DD wyłuskane z treści
    pub tags: Vec<String>,
    pub has_tasks: bool,
    pub tasks_done: u32,
    pub tasks_total: u32,
}

fn require_key(state: &State<'_, VaultState>) -> Result<[u8; 32], String> {
    state
        .key
        .lock()
        .ok_or_else(|| "Vault is locked".to_string())
}

/// Czy `s` (długości 10) ma kształt `YYYY-MM-DD` z sensownym miesiącem/dniem.
fn is_date_shaped(s: &[u8]) -> bool {
    if s.len() != 10 {
        return false;
    }
    let digit = |b: u8| b.is_ascii_digit();
    if !(digit(s[0]) && digit(s[1]) && digit(s[2]) && digit(s[3])) {
        return false;
    }
    if s[4] != b'-' || s[7] != b'-' {
        return false;
    }
    if !(digit(s[5]) && digit(s[6]) && digit(s[8]) && digit(s[9])) {
        return false;
    }
    let month = (s[5] - b'0') * 10 + (s[6] - b'0');
    let day = (s[8] - b'0') * 10 + (s[9] - b'0');
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

/// Wyłuskuje unikalne daty `YYYY-MM-DD` z treści (bez regexu, jak reszta kodu).
/// Granice „słowa" sprawdzamy ręcznie: sąsiadująca cyfra unieważnia dopasowanie
/// (żeby nie łapać fragmentów dłuższych liczb), ale `[[`/`]]` są dozwolone.
fn scan_dates(content: &str) -> Vec<String> {
    let bytes = content.as_bytes();
    let n = bytes.len();
    let mut out: Vec<String> = Vec::new();

    let mut i = 0usize;
    while i + 10 <= n {
        let window = &bytes[i..i + 10];
        if is_date_shaped(window) {
            let prev_is_digit = i > 0 && bytes[i - 1].is_ascii_digit();
            let next_is_digit = i + 10 < n && bytes[i + 10].is_ascii_digit();
            if !prev_is_digit && !next_is_digit {
                // ASCII-only, więc konwersja jest bezpieczna.
                let date = std::str::from_utf8(window).unwrap_or("").to_string();
                if !date.is_empty() && !out.contains(&date) {
                    out.push(date);
                }
                i += 10;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Liczy zadania (checkboxy) w treści: `(done, total)`. Wzorzec jak w edytorze:
/// linia zaczynająca się od `-`/`*`/`+`, spacji i `[ ]`/`[x]`/`[X]`.
fn count_tasks(content: &str) -> (u32, u32) {
    let mut total = 0u32;
    let mut done = 0u32;
    for line in content.lines() {
        let trimmed = line.trim_start();
        let b = trimmed.as_bytes();
        // [-*+] ' ' '[' (x| |X) ']'
        if b.len() >= 5
            && matches!(b[0], b'-' | b'*' | b'+')
            && b[1] == b' '
            && b[2] == b'['
            && b[4] == b']'
            && matches!(b[3], b' ' | b'x' | b'X')
        {
            total += 1;
            if matches!(b[3], b'x' | b'X') {
                done += 1;
            }
        }
    }
    (done, total)
}

/// Zwraca wszystkie notatki z danymi potrzebnymi kalendarzowi. Treść jest
/// deszyfrowana tylko na czas skanu i nie opuszcza Rusta; klucz jest zerowany.
#[tauri::command]
pub async fn get_calendar_notes(state: State<'_, VaultState>) -> Result<Vec<CalendarNote>, String> {
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

    let out = notes
        .into_iter()
        .map(|n| {
            let dates = scan_dates(&n.content);
            let (tasks_done, tasks_total) = count_tasks(&n.content);
            CalendarNote {
                id: n.id,
                title: n.title,
                created_at: n.created_at,
                dates,
                tags: n.tags,
                has_tasks: tasks_total > 0,
                tasks_done,
                tasks_total,
            }
        })
        .collect();

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_plain_and_bracketed_dates() {
        let content = "Spotkanie 2025-06-10 oraz [[2025-12-01]] i znowu 2025-06-10.";
        let dates = scan_dates(content);
        assert_eq!(dates, vec!["2025-06-10", "2025-12-01"]); // dedup, kolejność wystąpień
    }

    #[test]
    fn rejects_non_dates_and_bad_months() {
        // 13 miesiąc i 00 dzień odrzucone; cyfry dookoła unieważniają dopasowanie.
        let content = "2025-13-01 2025-00-10 12025-06-10x 9999-09-09";
        let dates = scan_dates(content);
        assert_eq!(dates, vec!["9999-09-09"]);
    }

    #[test]
    fn counts_tasks() {
        let content = "- [ ] a\n  - [x] b\n* [X] c\n+ [ ] d\nplain line\n- not a task";
        let (done, total) = count_tasks(content);
        assert_eq!(total, 4);
        assert_eq!(done, 2);
    }
}
