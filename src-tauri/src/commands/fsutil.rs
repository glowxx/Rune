//! Odporność na uszkodzenia plików vaultu: atomowy zapis (temp+rename),
//! odzyskiwanie osieroconych plików `*.tmp` po awarii oraz plik-blokada
//! `vault.lock` chroniący przed jednoczesnym zapisem z dwóch instancji.
//!
//! Format każdego pliku danych (notatki, foldery, kanban, czas, tablice,
//! załączniki) to `[12 nonce][ciphertext+tag GCM]`, więc weryfikacja odzysku
//! sprowadza się do próbnego odszyfrowania tym samym schematem.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::commands::crypto;
use crate::commands::timeutil;
use crate::models::EncryptedFile;

/// Ścieżka pliku tymczasowego dla atomowego zapisu: dokleja `.tmp` do pełnej
/// nazwy (np. `notes/abc.rune` → `notes/abc.rune.tmp`). Dopisanie (nie zamiana
/// rozszerzenia) sprawia, że odzysk jednoznacznie odtwarza nazwę docelową.
fn tmp_for(path: &Path) -> PathBuf {
    let mut s: OsString = path.as_os_str().to_os_string();
    s.push(".tmp");
    PathBuf::from(s)
}

/// Zapis atomowy: zapisz do pliku `*.tmp`, potem `rename` na docelowy. `rename`
/// w obrębie tego samego katalogu jest atomowy i podmienia istniejący plik
/// (na Windows i Unix), więc przerwanie zapisu (crash/zanik zasilania) nigdy nie
/// zostawia obciętego pliku docelowego — zostaje albo stara, albo nowa całość.
pub fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = tmp_for(path);
    std::fs::write(&tmp, data)?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Sprzątnij plik tymczasowy, żeby nie został jako sierota.
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Odtwarza nazwę docelową z pliku `*.tmp` (usuwa końcówkę `.tmp`).
fn strip_tmp(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_str()?;
    let base = name.strip_suffix(".tmp")?;
    Some(path.with_file_name(base))
}

/// Czy bajty dają się odszyfrować jako `[12 nonce][ciphertext]` danym kluczem.
/// Sukces = kompletny, niezepsuty plik danych (tag GCM się zgadza).
fn decrypts_ok(key: &[u8; 32], data: &[u8]) -> bool {
    if data.len() < 28 {
        return false;
    }
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&data[..12]);
    let enc = EncryptedFile {
        salt: [0u8; 32],
        nonce,
        ciphertext: data[12..].to_vec(),
    };
    crypto::decrypt(key, &enc).is_ok()
}

/// Katalogi vaultu, w których mogą pojawić się pliki danych (i ich `*.tmp`).
fn candidate_dirs(base: &Path) -> Vec<PathBuf> {
    [
        "",
        "notes",
        "notes_duress",
        "attachments",
        "attachments_duress",
        "whiteboards",
        "whiteboards_duress",
    ]
    .iter()
    .map(|s| {
        if s.is_empty() {
            base.to_path_buf()
        } else {
            base.join(s)
        }
    })
    .collect()
}

/// Sprząta/odzyskuje osierocone pliki `*.tmp` po awarii w trakcie atomowego
/// zapisu. Dla każdego `X.tmp`:
/// - jeśli plik docelowy `X` istnieje → `*.tmp` to pozostałość, kasujemy;
/// - w przeciwnym razie próbujemy odszyfrować `*.tmp` bieżącym kluczem — gdy się
///   uda, promujemy go na `X` (rename); gdy nie, kasujemy (niekompletny zapis).
///
/// `vault.rune` (inny format) nigdy nie jest promowany — jeśli istnieje,
/// `*.tmp` znika; jeśli nie istnieje, i tak się nie odszyfruje tym schematem,
/// więc zostanie skasowany. To celowo zachowawcze: nie ryzykujemy podmiany
/// pliku vaultu danymi, których nie potrafimy zweryfikować.
pub fn recover_tmp_files(base: &Path, key: &[u8; 32]) {
    for dir in candidate_dirs(base) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("tmp") {
                continue;
            }
            let Some(final_path) = strip_tmp(&p) else {
                let _ = std::fs::remove_file(&p);
                continue;
            };
            if final_path.exists() {
                let _ = std::fs::remove_file(&p);
                continue;
            }
            match std::fs::read(&p) {
                Ok(data) if decrypts_ok(key, &data) => {
                    let _ = std::fs::rename(&p, &final_path);
                }
                _ => {
                    let _ = std::fs::remove_file(&p);
                }
            }
        }
    }
}

// ── Plik-blokada vaultu (jednoczesny dostęp z dwóch instancji) ───────────────

fn lock_path(base: &Path) -> PathBuf {
    base.join("vault.lock")
}

/// Czy proces o danym PID nadal żyje. Zapobiega blokadzie po awarii (stary PID
/// jest martwy → blokadę można przejąć). Na Windows sprawdza `GetExitCodeProcess`,
/// na Linux istnienie `/proc/{pid}`, na innych platformach zakłada, że nie żyje
/// (zachowawczo — brak fałszywych blokad kosztem słabszej ochrony).
#[cfg(target_os = "windows")]
fn process_alive(pid: u32) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    const STILL_ACTIVE: u32 = 259;
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };
        if handle.is_invalid() {
            return false;
        }
        let mut code: u32 = 0;
        let ok = GetExitCodeProcess(handle, &mut code).is_ok();
        let _ = CloseHandle(handle);
        ok && code == STILL_ACTIVE
    }
}

#[cfg(target_os = "linux")]
fn process_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn process_alive(_pid: u32) -> bool {
    false
}

/// Próbuje przejąć blokadę vaultu. Zwraca `Err`, gdy inna **żywa** instancja
/// Rune trzyma już ten vault otwarty (ochrona przed jednoczesnym zapisem).
/// Osierocona blokada po martwym procesie jest po cichu przejmowana.
pub fn acquire_lock(base: &Path) -> Result<(), String> {
    let lock = lock_path(base);
    if let Ok(content) = std::fs::read_to_string(&lock) {
        if let Some(pid) = content
            .lines()
            .next()
            .and_then(|l| l.trim().parse::<u32>().ok())
        {
            if pid != std::process::id() && process_alive(pid) {
                return Err("This vault is already open in another Rune window.".to_string());
            }
        }
    }
    let _ = std::fs::create_dir_all(base);
    let _ = std::fs::write(
        &lock,
        format!("{}\n{}", std::process::id(), timeutil::now_iso()),
    );
    Ok(())
}

/// Zwalnia blokadę vaultu (przy locku, przełączeniu vaultu lub wyjściu).
pub fn release_lock(base: &Path) {
    let _ = std::fs::remove_file(lock_path(base));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_roundtrips_and_replaces() {
        let dir = std::env::temp_dir().join(format!("rune_fsutil_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join("a.rune");
        write_atomic(&target, b"one").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"one");
        write_atomic(&target, b"two").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"two");
        // Po udanym zapisie nie zostaje żaden plik .tmp.
        assert!(!tmp_for(&target).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn strip_tmp_recovers_final_name() {
        let p = Path::new("x/abc.rune.tmp");
        assert_eq!(strip_tmp(p).unwrap(), Path::new("x/abc.rune"));
        assert!(strip_tmp(Path::new("x/abc.rune")).is_none());
    }

    #[test]
    fn recover_deletes_unverifiable_and_stale_tmp() {
        let key = [7u8; 32];
        let base = std::env::temp_dir().join(format!("rune_recover_{}", std::process::id()));
        let notes = base.join("notes");
        let _ = std::fs::create_dir_all(&notes);

        // Sierota .tmp z bezsensowną zawartością bez pliku docelowego → kasowana.
        let junk = notes.join("junk.rune.tmp");
        std::fs::write(&junk, b"not encrypted").unwrap();

        // .tmp obok istniejącego pliku docelowego → kasowana (stara pozostałość).
        let final_ok = notes.join("keep.rune");
        std::fs::write(&final_ok, b"final").unwrap();
        let stale = notes.join("keep.rune.tmp");
        std::fs::write(&stale, b"stale").unwrap();

        // Kompletny, szyfrowalny .tmp bez pliku docelowego → promowany.
        let enc = crypto::encrypt(&key, b"recovered").unwrap();
        let mut good_bytes = Vec::new();
        good_bytes.extend_from_slice(&enc.nonce);
        good_bytes.extend_from_slice(&enc.ciphertext);
        let good_tmp = notes.join("good.rune.tmp");
        std::fs::write(&good_tmp, &good_bytes).unwrap();

        recover_tmp_files(&base, &key);

        assert!(!junk.exists(), "unverifiable tmp should be deleted");
        assert!(!stale.exists(), "stale tmp beside final should be deleted");
        assert!(std::fs::read(&final_ok).unwrap() == b"final");
        assert!(!good_tmp.exists(), "recovered tmp should be renamed away");
        assert!(
            notes.join("good.rune").exists(),
            "good tmp should be promoted"
        );

        let _ = std::fs::remove_dir_all(&base);
    }
}
