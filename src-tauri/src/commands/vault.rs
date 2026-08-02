//! Zarządzanie vaultem: tworzenie, odblokowanie, blokowanie.
//!
//! Format pliku `vault.rune`:
//! ```text
//! [4 bajty LE: długość headera][header JSON: ArgonParams][EncryptedFile bytes]
//! ```
//! Niezaszyfrowany header pozwala `unlock` poznać parametry Argon2 bez
//! zgadywania. EncryptedFile zawiera sól KDF, nonce i szyfrogram VaultMeta.

use std::path::PathBuf;

use parking_lot::Mutex;
use rand::rngs::OsRng;
use rand::RngCore;
use tauri::State;
use zeroize::Zeroize;

use crate::commands::crypto;
use crate::commands::fsutil;
use crate::error::RuneError;
use crate::models::{ArgonParams, EncryptedFile, VaultMeta};

/// Stan vaultu trzymany w pamięci procesu. Klucz nigdy nie trafia na dysk.
#[derive(Default)]
pub struct VaultState {
    pub key: Mutex<Option<[u8; 32]>>,
    // Zapamiętane przy odblokowaniu/utworzeniu — wykorzystywane przez przyszłe
    // operacje na notatkach; obecnie zapisywane do późniejszego użycia.
    #[allow(dead_code)]
    pub vault_path: Mutex<Option<PathBuf>>,
    #[allow(dead_code)]
    pub argon_params: Mutex<Option<ArgonParams>>,
    /// Czy bieżąca sesja działa w trybie duress (wabik). Gdy `true`, operacje
    /// na notatkach/folderach kierują się do katalogów `*_duress`, a prawdziwe
    /// dane pozostają niewidoczne. Frontend nie zna tej flagi — UI jest
    /// nieodróżnialne od trybu normalnego.
    pub is_duress: Mutex<bool>,
    /// Identyfikator aktywnego vaultu (segment podkatalogu w `%APPDATA%/rune/`).
    /// Pusty string traktujemy jak `"default"`. Steruje tym, na którym vaulcie
    /// działają wszystkie komendy danych (notatki/foldery/załączniki).
    pub active_vault: Mutex<String>,
    /// Indeks pełnotekstowy (SQLite FTS5) trzymany wyłącznie w pamięci RAM.
    /// Budowany po odblokowaniu, porzucany przy locku/przełączeniu vaultu —
    /// nigdy nie powstaje plaintextowy plik SQLite na dysku.
    pub search_db: Mutex<Option<rusqlite::Connection>>,
}

/// Katalog danych aplikacji (korzeń wszystkich vaultów): `dirs::data_dir()/rune`.
pub(crate) fn data_dir_rune() -> Result<PathBuf, RuneError> {
    dirs::data_dir()
        .map(|d| d.join("rune"))
        .ok_or(RuneError::DataDir)
}

/// Sanityzuje identyfikator vaultu: dopuszcza tylko `[A-Za-z0-9_-]`, odrzuca
/// puste/„..". Pusty wynik mapuje na `"default"` (ochrona przed path traversal,
/// bo id steruje nazwą katalogu na dysku).
pub(crate) fn sanitize_vault_id(id: &str) -> String {
    let clean: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if clean.is_empty() {
        "default".to_string()
    } else {
        clean
    }
}

/// Identyfikator aktywnego vaultu (z fallbackiem na `"default"`).
pub(crate) fn active_vault_id(state: &State<'_, VaultState>) -> String {
    sanitize_vault_id(&state.active_vault.lock())
}

/// Katalog aktywnego vaultu: `%APPDATA%/rune/{id}`. Wszystkie ścieżki danych
/// (notatki, foldery, załączniki) wywodzą się z tego katalogu, dzięki czemu
/// każdy vault jest w pełni odizolowany na dysku.
pub(crate) fn active_vault_dir(state: &State<'_, VaultState>) -> Result<PathBuf, RuneError> {
    Ok(data_dir_rune()?.join(active_vault_id(state)))
}

/// Inicjalizuje pliki nowego vaultu w katalogu `vdir`: zapisuje `vault.rune`
/// (świeża sól + zaszyfrowane `VaultMeta`) i tworzy katalog `notes/`. Zwraca
/// wyprowadzony klucz (wywołujący odpowiada za jego `zeroize`). Hasło jest
/// zerowane wewnątrz funkcji niezależnie od wyniku.
pub(crate) fn init_vault_at(
    vdir: &std::path::Path,
    password: &mut String,
    params: &ArgonParams,
) -> Result<[u8; 32], String> {
    if let Err(e) = std::fs::create_dir_all(vdir) {
        let _ = e;
        password.zeroize();
        return Err("Could not create vault directory".to_string());
    }
    let vault_path = vdir.join("vault.rune");

    let mut salt = [0u8; 32];
    OsRng.fill_bytes(&mut salt);

    let mut key = match crypto::derive_key(password.as_str(), &salt, params) {
        Ok(k) => k,
        Err(_) => {
            password.zeroize();
            return Err("Key derivation error".to_string());
        }
    };
    password.zeroize();

    let meta = VaultMeta {
        version: 1,
        created_at: now_unix(),
        has_duress: false,
        argon_params: params.clone(),
    };
    let plaintext = match serde_json::to_vec(&meta) {
        Ok(p) => p,
        Err(_) => {
            key.zeroize();
            return Err("Metadata serialization error".to_string());
        }
    };
    let mut enc = match crypto::encrypt(&key, &plaintext) {
        Ok(e) => e,
        Err(_) => {
            key.zeroize();
            return Err("Encryption error".to_string());
        }
    };
    enc.salt = salt;

    let file_bytes = match build_vault_file(params, &enc, None) {
        Ok(b) => b,
        Err(_) => {
            key.zeroize();
            return Err("Vault write error".to_string());
        }
    };
    if fsutil::write_atomic(&vault_path, &file_bytes).is_err() {
        key.zeroize();
        return Err("Could not save vault".to_string());
    }
    if std::fs::create_dir_all(vdir.join("notes")).is_err() {
        key.zeroize();
        return Err("Could not create notes directory".to_string());
    }
    Ok(key)
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Dokleja blob z 4-bajtowym prefiksem długości (LE): `[4 len][bajty bloba]`.
fn append_blob(out: &mut Vec<u8>, enc: &EncryptedFile) {
    let bytes = enc.to_bytes();
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&bytes);
}

/// Odczytuje jeden blob `[4 len][len bajtów]` od `offset`. Zwraca `(blob,
/// nowy offset)` albo `None`, gdy długość jest niepoprawna lub wykracza poza bufor.
fn read_blob(data: &[u8], offset: usize) -> Option<(EncryptedFile, usize)> {
    let start = offset.checked_add(4)?;
    if start > data.len() {
        return None;
    }
    let len = u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]) as usize;
    if len < 44 {
        // 32 salt + 12 nonce — minimum sensownego EncryptedFile.
        return None;
    }
    let end = start.checked_add(len)?;
    if end > data.len() {
        return None;
    }
    let enc = EncryptedFile::from_bytes(&data[start..end]).ok()?;
    Some((enc, end))
}

/// Składa plik vault: `[4 hlen][header JSON][blob real]([blob duress])`.
/// Bloby są prefiksowane długością, więc plik może pomieścić dwa niezależne
/// szyfrogramy (prawdziwy i duress) bez dwuznaczności.
fn build_vault_file(
    params: &ArgonParams,
    real: &EncryptedFile,
    duress: Option<&EncryptedFile>,
) -> Result<Vec<u8>, RuneError> {
    let header = serde_json::to_vec(params)?;
    let mut out = Vec::new();
    out.extend_from_slice(&(header.len() as u32).to_le_bytes());
    out.extend_from_slice(&header);
    append_blob(&mut out, real);
    if let Some(d) = duress {
        append_blob(&mut out, d);
    }
    Ok(out)
}

/// Parsuje header i zwraca `(ArgonParams, reszta bajtów po headerze)`.
fn parse_vault_header(data: &[u8]) -> Result<(ArgonParams, &[u8]), RuneError> {
    if data.len() < 4 {
        return Err(RuneError::Format);
    }
    let header_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let end = 4usize.checked_add(header_len).ok_or(RuneError::Format)?;
    if data.len() < end {
        return Err(RuneError::Format);
    }
    let params: ArgonParams = serde_json::from_slice(&data[4..end])?;
    Ok((params, &data[end..]))
}

/// Parsuje pełny plik vault: header + blob real (+ opcjonalny blob duress).
///
/// Obsługuje dwa formaty bez migracji:
/// - **nowy**: bloby prefiksowane długością (akceptowany tylko gdy prefiksy
///   pokrywają cały bufor — co dla losowych danych starego formatu jest
///   praktycznie niemożliwe);
/// - **starszy**: cała reszta po headerze to pojedynczy `EncryptedFile` bez
///   prefiksu (vaulty utworzone przed dodaniem duress).
fn parse_vault_file(
    data: &[u8],
) -> Result<(ArgonParams, EncryptedFile, Option<EncryptedFile>), RuneError> {
    let (params, rest) = parse_vault_header(data)?;

    if let Some((real, off)) = read_blob(rest, 0) {
        if off == rest.len() {
            return Ok((params, real, None));
        }
        if let Some((duress, off2)) = read_blob(rest, off) {
            if off2 == rest.len() {
                return Ok((params, real, Some(duress)));
            }
        }
    }

    // Starszy format: pojedynczy EncryptedFile zajmujący całą resztę.
    let real = EncryptedFile::from_bytes(rest)?;
    Ok((params, real, None))
}

#[tauri::command]
pub async fn create_vault(
    mut password: String,
    params: ArgonParams,
    state: State<'_, VaultState>,
) -> Result<(), String> {
    if let Err(e) = params.validate() {
        password.zeroize();
        return Err(e);
    }

    let vdir = active_vault_dir(&state).map_err(|_| {
        password.zeroize();
        "Data directory not found".to_string()
    })?;

    // Kosztowna derywacja Argon2 (+ zapis plików) na osobnym wątku blokującym,
    // żeby nie zablokować runtime'u async — inaczej długie „Maximum" zawiesza
    // inne komendy (np. auto-lock aktualnie odblokowanego vaultu). `init_vault_at`
    // zeruje hasło niezależnie od wyniku.
    let params_bg = params.clone();
    let vdir_bg = vdir.clone();
    let mut key = tauri::async_runtime::spawn_blocking(move || {
        let mut pw = password;
        init_vault_at(&vdir_bg, &mut pw, &params_bg)
    })
    .await
    .map_err(|_| "Vault creation failed".to_string())??;

    // Zarejestruj vault na liście (pierwsze uruchomienie → „default" jako „Personal").
    let id = active_vault_id(&state);
    let name = if id == "default" {
        "Personal".to_string()
    } else {
        id.clone()
    };
    crate::commands::vaults::register_vault(&id, &name);
    crate::commands::vaults::touch_last_opened(&id);

    *state.key.lock() = Some(key);
    *state.vault_path.lock() = Some(vdir.join("vault.rune"));
    *state.argon_params.lock() = Some(params);
    *state.is_duress.lock() = false;
    *state.search_db.lock() = None; // świeży vault — brak indeksu
    key.zeroize();
    let _ = fsutil::acquire_lock(&vdir); // świeży vault — nikt inny go nie trzyma
    Ok(())
}

#[tauri::command]
pub async fn unlock_vault(
    mut password: String,
    state: State<'_, VaultState>,
) -> Result<bool, String> {
    let base = match active_vault_dir(&state) {
        Ok(d) => d,
        Err(_) => {
            password.zeroize();
            return Err("Data directory not found".to_string());
        }
    };
    let vault_path = base.join("vault.rune");

    let data = match std::fs::read(&vault_path) {
        Ok(d) => d,
        Err(_) => {
            password.zeroize();
            return Err("Vault does not exist".to_string());
        }
    };

    let (params, real_enc, duress_enc) = match parse_vault_file(&data) {
        Ok(v) => v,
        Err(_) => {
            password.zeroize();
            return Err("Corrupted vault file".to_string());
        }
    };

    // Argon2 (nawet dwa wyprowadzenia: prawdziwe + duress) i weryfikacja tagu
    // GCM na osobnym wątku blokującym — długie „Maximum" nie zawiesza wtedy
    // runtime'u async ani UI. Zwraca `Some((klucz, is_duress))` przy trafieniu.
    let params_bg = params.clone();
    let outcome =
        tauri::async_runtime::spawn_blocking(move || -> Result<Option<([u8; 32], bool)>, ()> {
            // 1. Prawdziwe hasło.
            let mut real_key =
                crypto::derive_key(&password, &real_enc.salt, &params_bg).map_err(|_| ())?;
            if crypto::decrypt(&real_key, &real_enc).is_ok() {
                password.zeroize();
                return Ok(Some((real_key, false)));
            }
            real_key.zeroize();

            // 2. Hasło duress (jeśli skonfigurowane). Wynik zewnętrznie identyczny.
            if let Some(d) = duress_enc {
                let mut duress_key =
                    crypto::derive_key(&password, &d.salt, &params_bg).map_err(|_| ())?;
                if crypto::decrypt(&duress_key, &d).is_ok() {
                    password.zeroize();
                    return Ok(Some((duress_key, true)));
                }
                duress_key.zeroize();
            }

            password.zeroize();
            Ok(None)
        })
        .await
        .map_err(|_| "Unlock failed".to_string())?
        .map_err(|_| "Key derivation error".to_string())?;

    // Żadne hasło nie pasuje — błąd nierozróżnialny (złe hasło / uszkodzone dane).
    let Some((mut key, is_duress)) = outcome else {
        return Ok(false);
    };

    // Ochrona przed jednoczesnym otwarciem tego samego vaultu w drugiej instancji
    // Rune (współbieżny zapis mógłby uszkodzić pliki). Blokada po martwym procesie
    // jest po cichu przejmowana; żywy właściciel → jasny komunikat.
    if let Err(e) = fsutil::acquire_lock(&base) {
        key.zeroize();
        return Err(e);
    }

    if is_duress {
        let _ = std::fs::create_dir_all(base.join("notes_duress"));
    }

    // Odzyskaj lub sprzątnij osierocone pliki `*.tmp` po ewentualnej wcześniejszej
    // awarii w trakcie atomowego zapisu (teraz mamy klucz do weryfikacji).
    fsutil::recover_tmp_files(&base, &key);

    *state.key.lock() = Some(key);
    *state.vault_path.lock() = Some(vault_path);
    *state.argon_params.lock() = Some(params);
    *state.is_duress.lock() = is_duress;
    key.zeroize();
    crate::commands::vaults::touch_last_opened(&active_vault_id(&state));
    Ok(true)
}

#[tauri::command]
pub async fn lock_vault(app: tauri::AppHandle, state: State<'_, VaultState>) -> Result<(), String> {
    // Zamknij wszystkie okna tablic — nie mogą przeżyć zablokowania vaultu
    // (klucz znika, a okno mogłoby pokazywać odszyfrowaną scenę).
    crate::commands::whiteboard::close_all_whiteboard_windows(&app);

    // Zwolnij plik-blokadę zanim wyczyścimy aktywny vault (potem znika ścieżka).
    if let Ok(dir) = active_vault_dir(&state) {
        fsutil::release_lock(&dir);
    }

    let mut guard = state.key.lock();
    if let Some(ref mut k) = *guard {
        k.zeroize();
    }
    *guard = None;
    drop(guard);
    *state.is_duress.lock() = false;
    *state.search_db.lock() = None; // indeks znika z pamięci przy locku
    Ok(())
}

// Uwaga: komenda async pobierająca `State<'_, _>` musi zwracać `Result`
// (ograniczenie Tauri/Rust), dlatego zwracamy `Result<bool, String>`.
#[tauri::command]
pub async fn is_vault_unlocked(state: State<'_, VaultState>) -> Result<bool, String> {
    Ok(state.key.lock().is_some())
}

#[tauri::command]
pub async fn vault_exists(state: State<'_, VaultState>) -> Result<bool, String> {
    Ok(active_vault_dir(&state)
        .map(|p| p.join("vault.rune").exists())
        .unwrap_or(false))
}

#[tauri::command]
pub async fn get_vault_params(state: State<'_, VaultState>) -> Result<Option<ArgonParams>, String> {
    let path = active_vault_dir(&state)
        .map_err(|_| "Data directory not found".to_string())?
        .join("vault.rune");

    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => return Ok(None),
    };

    match parse_vault_header(&data) {
        Ok((params, _)) => Ok(Some(params)),
        Err(_) => Err("Corrupted vault file".to_string()),
    }
}

/// Ustawia (lub zmienia) hasło duress. Wymaga prawdziwego hasła do
/// uwierzytelnienia, żeby nikt poza właścicielem nie mógł podmienić wabika.
///
/// Dodaje drugi, niezależny szyfrogram do `vault.rune` (świeża sól + nonce),
/// zawierający decoy `VaultMeta`. Po jego skonfigurowaniu wpisanie hasła duress
/// na ekranie Unlock odblokowuje pusty zestaw notatek (katalog `notes_duress`),
/// a prawdziwe dane pozostają niedostępne i niewidoczne.
#[tauri::command]
pub async fn set_duress_password(
    mut real_password: String,
    mut duress_password: String,
    state: State<'_, VaultState>,
) -> Result<(), String> {
    if duress_password.is_empty() {
        real_password.zeroize();
        duress_password.zeroize();
        return Err("Duress password cannot be empty".to_string());
    }
    if duress_password == real_password {
        real_password.zeroize();
        duress_password.zeroize();
        return Err("Duress password must differ from your real password".to_string());
    }

    let base = match active_vault_dir(&state) {
        Ok(d) => d,
        Err(_) => {
            real_password.zeroize();
            duress_password.zeroize();
            return Err("Data directory not found".to_string());
        }
    };
    let vault_path = base.join("vault.rune");

    let data = match std::fs::read(&vault_path) {
        Ok(d) => d,
        Err(_) => {
            real_password.zeroize();
            duress_password.zeroize();
            return Err("Vault does not exist".to_string());
        }
    };

    let (params, real_enc, _old_duress) = match parse_vault_file(&data) {
        Ok(v) => v,
        Err(_) => {
            real_password.zeroize();
            duress_password.zeroize();
            return Err("Corrupted vault file".to_string());
        }
    };

    // Uwierzytelnij prawdziwym hasłem (musi odszyfrować prawdziwy blob).
    let mut real_key = match crypto::derive_key(&real_password, &real_enc.salt, &params) {
        Ok(k) => k,
        Err(_) => {
            real_password.zeroize();
            duress_password.zeroize();
            return Err("Key derivation error".to_string());
        }
    };
    let mut real_meta_plain = match crypto::decrypt(&real_key, &real_enc) {
        Ok(p) => p,
        Err(_) => {
            real_key.zeroize();
            real_password.zeroize();
            duress_password.zeroize();
            return Err("Invalid password".to_string());
        }
    };
    real_password.zeroize();

    // Odznacz w prawdziwej metadanej, że istnieje wabik, i zaszyfruj ją na nowo
    // tym samym kluczem (świeży nonce, zachowana sól derywacji prawdziwego klucza).
    let mut real_meta: VaultMeta = match serde_json::from_slice(&real_meta_plain) {
        Ok(m) => m,
        Err(_) => {
            real_meta_plain.zeroize();
            real_key.zeroize();
            duress_password.zeroize();
            return Err("Corrupted vault file".to_string());
        }
    };
    real_meta_plain.zeroize();
    real_meta.has_duress = true;

    let real_plain2 = match serde_json::to_vec(&real_meta) {
        Ok(p) => p,
        Err(_) => {
            real_key.zeroize();
            duress_password.zeroize();
            return Err("Metadata serialization error".to_string());
        }
    };
    let mut new_real_enc = match crypto::encrypt(&real_key, &real_plain2) {
        Ok(e) => e,
        Err(_) => {
            real_key.zeroize();
            duress_password.zeroize();
            return Err("Encryption error".to_string());
        }
    };
    new_real_enc.salt = real_enc.salt; // sól, z której pochodzi prawdziwy klucz
    real_key.zeroize();

    // Zbuduj blob duress: świeża sól + decoy VaultMeta.
    let mut duress_salt = [0u8; 32];
    OsRng.fill_bytes(&mut duress_salt);
    let mut duress_key = match crypto::derive_key(&duress_password, &duress_salt, &params) {
        Ok(k) => k,
        Err(_) => {
            duress_password.zeroize();
            return Err("Key derivation error".to_string());
        }
    };
    duress_password.zeroize();

    let decoy = VaultMeta {
        version: 1,
        created_at: now_unix(),
        has_duress: false, // wabik nie ujawnia, że jest wabikiem
        argon_params: params.clone(),
    };
    let decoy_plain = match serde_json::to_vec(&decoy) {
        Ok(p) => p,
        Err(_) => {
            duress_key.zeroize();
            return Err("Metadata serialization error".to_string());
        }
    };
    let mut duress_enc = match crypto::encrypt(&duress_key, &decoy_plain) {
        Ok(e) => e,
        Err(_) => {
            duress_key.zeroize();
            return Err("Encryption error".to_string());
        }
    };
    duress_enc.salt = duress_salt;
    duress_key.zeroize();

    // Zapisz vault z dwoma blobami i przygotuj katalog wabików.
    let file_bytes = build_vault_file(&params, &new_real_enc, Some(&duress_enc))
        .map_err(|_| "Vault write error".to_string())?;

    // Kopia bezpieczeństwa poprzedniego `vault.rune` tuż przed nadpisaniem —
    // trzymana do potwierdzenia udanego zapisu, po czym usuwana. Gdyby atomowy
    // zapis zawiódł, przywracamy poprzedni plik z kopii.
    let backup = vault_path.with_extension("rune.bak");
    let _ = std::fs::copy(&vault_path, &backup);
    if fsutil::write_atomic(&vault_path, &file_bytes).is_err() {
        let _ = std::fs::rename(&backup, &vault_path);
        return Err("Could not save vault".to_string());
    }
    let _ = std::fs::remove_file(&backup);
    let _ = std::fs::create_dir_all(base.join("notes_duress"));

    Ok(())
}

/// Czy w `vault.rune` skonfigurowano hasło duress (drugi blob). Nie ujawnia
/// samego hasła — tylko fakt jego istnienia (do statusu w ustawieniach).
#[tauri::command]
pub async fn duress_configured(state: State<'_, VaultState>) -> Result<bool, String> {
    let path = active_vault_dir(&state)
        .map_err(|_| "Data directory not found".to_string())?
        .join("vault.rune");

    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => return Ok(false),
    };

    match parse_vault_file(&data) {
        Ok((_, _, duress)) => Ok(duress.is_some()),
        Err(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> ArgonParams {
        ArgonParams {
            memory_kib: 8192,
            iterations: 1,
            parallelism: 1,
        }
    }

    fn blob(key: &[u8; 32], salt: [u8; 32], plain: &[u8]) -> EncryptedFile {
        let mut enc = crypto::encrypt(key, plain).unwrap();
        enc.salt = salt;
        enc
    }

    /// Nowy format bez duress: pełny obieg build → parse, deszyfracja kluczem real.
    #[test]
    fn vault_round_trip_real_only() {
        let real_key = [1u8; 32];
        let real = blob(&real_key, [9u8; 32], b"{\"real\":true}");
        let bytes = build_vault_file(&params(), &real, None).unwrap();

        let (_p, real_out, duress_out) = parse_vault_file(&bytes).unwrap();
        assert!(duress_out.is_none());
        assert_eq!(real_out.salt, [9u8; 32]);
        assert_eq!(
            crypto::decrypt(&real_key, &real_out).unwrap(),
            b"{\"real\":true}"
        );
    }

    /// Nowy format z duress: oba bloby odczytane, każdy odszyfrowany swoim kluczem.
    #[test]
    fn vault_round_trip_with_duress() {
        let real_key = [1u8; 32];
        let duress_key = [2u8; 32];
        let real = blob(&real_key, [9u8; 32], b"REAL");
        let duress = blob(&duress_key, [7u8; 32], b"DECOY");
        let bytes = build_vault_file(&params(), &real, Some(&duress)).unwrap();

        let (_p, real_out, duress_out) = parse_vault_file(&bytes).unwrap();
        let duress_out = duress_out.expect("duress blob present");
        assert_eq!(crypto::decrypt(&real_key, &real_out).unwrap(), b"REAL");
        assert_eq!(crypto::decrypt(&duress_key, &duress_out).unwrap(), b"DECOY");
        // Klucz real nie odszyfruje bloba duress i odwrotnie (rozdzielność).
        assert!(crypto::decrypt(&real_key, &duress_out).is_err());
        assert!(crypto::decrypt(&duress_key, &real_out).is_err());
    }

    /// Starszy format (pojedynczy EncryptedFile bez prefiksu długości) wciąż
    /// parsuje się jako blob real bez duress — wsteczna kompatybilność.
    #[test]
    fn vault_legacy_single_blob_parses() {
        let real_key = [3u8; 32];
        let real = blob(&real_key, [5u8; 32], b"LEGACY");

        // Ręcznie sklejony stary układ: [4 hlen][header][EncryptedFile bez prefiksu].
        let header = serde_json::to_vec(&params()).unwrap();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(header.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&header);
        bytes.extend_from_slice(&real.to_bytes());

        let (_p, real_out, duress_out) = parse_vault_file(&bytes).unwrap();
        assert!(duress_out.is_none());
        assert_eq!(crypto::decrypt(&real_key, &real_out).unwrap(), b"LEGACY");
    }
}
