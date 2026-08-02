//! Obsługa wielu niezależnych vaultów. Każdy vault to osobny podkatalog
//! `%APPDATA%/rune/{id}/` z własnym `vault.rune`, notatkami i załącznikami.
//! Jawna lista vaultów (bez żadnych sekretów) leży w `%APPDATA%/rune/vaults.json`.
//!
//! Sekrety (klucze, hasła) nigdy nie trafiają do `vaults.json` — plik zawiera
//! tylko id, nazwę, ścieżkę i znacznik ostatniego otwarcia.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;
use zeroize::Zeroize;

use crate::commands::fsutil;
use crate::commands::timeutil;
use crate::commands::vault::{
    active_vault_dir, active_vault_id, data_dir_rune, init_vault_at, sanitize_vault_id, VaultState,
};
use crate::models::ArgonParams;

/// Wpis listy vaultów (camelCase — wymiana z JS). Bez danych wrażliwych.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub last_opened: String,
}

/// Ścieżka pliku `vaults.json`.
fn vaults_file() -> Option<PathBuf> {
    data_dir_rune().ok().map(|d| d.join("vaults.json"))
}

/// Odczytuje listę vaultów. Brak pliku / błąd parsowania = pusta lista.
pub fn read_vaults_list() -> Vec<VaultInfo> {
    let Some(p) = vaults_file() else {
        return Vec::new();
    };
    let Ok(data) = std::fs::read(&p) else {
        return Vec::new();
    };
    serde_json::from_slice(&data).unwrap_or_default()
}

/// Zapisuje listę vaultów (tworzy katalog korzenia, gdy trzeba).
fn write_vaults_list(list: &[VaultInfo]) -> Result<(), String> {
    let p = vaults_file().ok_or_else(|| "Data directory not found".to_string())?;
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json = serde_json::to_vec_pretty(list).map_err(|_| "Serialization error".to_string())?;
    fsutil::write_atomic(&p, &json).map_err(|_| "Could not write vaults list".to_string())
}

/// Dodaje wpis vaultu na listę, jeśli jeszcze nie istnieje (idempotentnie).
pub fn register_vault(id: &str, name: &str) {
    let mut list = read_vaults_list();
    if list.iter().any(|v| v.id == id) {
        return;
    }
    list.push(VaultInfo {
        id: id.to_string(),
        name: name.to_string(),
        path: id.to_string(),
        last_opened: timeutil::now_iso(),
    });
    let _ = write_vaults_list(&list);
}

/// Aktualizuje znacznik ostatniego otwarcia danego vaultu.
pub fn touch_last_opened(id: &str) {
    let mut list = read_vaults_list();
    let mut changed = false;
    for v in list.iter_mut() {
        if v.id == id {
            v.last_opened = timeutil::now_iso();
            changed = true;
        }
    }
    if changed {
        let _ = write_vaults_list(&list);
    }
}

/// Jednorazowa migracja do układu wielo-vaultowego. Przenosi dane leżące
/// bezpośrednio w `%APPDATA%/rune/` do podkatalogu `default/` i rejestruje go
/// na liście. Bezpieczna do wielokrotnego wywołania (no-op gdy już zmigrowano).
pub fn migrate_to_multi_vault() {
    let Ok(base) = data_dir_rune() else {
        return;
    };
    let vault_rune = base.join("vault.rune");
    let default_dir = base.join("default");

    if vault_rune.exists() && !default_dir.exists() {
        if std::fs::create_dir_all(&default_dir).is_err() {
            return;
        }
        for item in [
            "vault.rune",
            "folders.rune",
            "folders_duress.rune",
            "attachments_meta.rune",
            "attachments_meta_duress.rune",
        ] {
            let src = base.join(item);
            if src.exists() {
                let _ = std::fs::rename(&src, default_dir.join(item));
            }
        }
        for dir in ["notes", "attachments", "notes_duress", "attachments_duress"] {
            let src = base.join(dir);
            if src.exists() {
                let _ = std::fs::rename(&src, default_dir.join(dir));
            }
        }
    }

    // Zabezpieczenie: gdy istnieje `default/vault.rune`, upewnij się że figuruje
    // na liście (np. po migracji albo gdy `vaults.json` zostało skasowane).
    if default_dir.join("vault.rune").exists() {
        register_vault("default", "Personal");
    }
}

// ── Komendy ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_vaults() -> Result<Vec<VaultInfo>, String> {
    Ok(read_vaults_list())
}

/// Tworzy nowy, niezależny vault i od razu go odblokowuje (klucz ląduje w
/// stanie Rusta — użytkownik trafia prosto do edytora nowego vaultu).
#[tauri::command]
pub async fn create_new_vault(
    name: String,
    mut password: String,
    params: ArgonParams,
    state: State<'_, VaultState>,
) -> Result<VaultInfo, String> {
    if let Err(e) = params.validate() {
        password.zeroize();
        return Err(e);
    }
    let name = name.trim().to_string();
    if name.is_empty() {
        password.zeroize();
        return Err("Vault name cannot be empty".to_string());
    }

    let id = Uuid::new_v4().to_string();
    let base = match data_dir_rune() {
        Ok(b) => b,
        Err(_) => {
            password.zeroize();
            return Err("Data directory not found".to_string());
        }
    };
    let vdir = base.join(&id);

    // Kosztowna derywacja Argon2 (+ zapis) na wątku blokującym — patrz `create_vault`.
    // `init_vault_at` zeruje hasło niezależnie od wyniku.
    let params_bg = params.clone();
    let vdir_bg = vdir.clone();
    let mut key = tauri::async_runtime::spawn_blocking(move || {
        let mut pw = password;
        init_vault_at(&vdir_bg, &mut pw, &params_bg)
    })
    .await
    .map_err(|_| "Vault creation failed".to_string())??;

    register_vault(&id, &name);
    touch_last_opened(&id);

    // Aktywuj i odblokuj nowy vault.
    *state.active_vault.lock() = id.clone();
    *state.key.lock() = Some(key);
    *state.vault_path.lock() = Some(vdir.join("vault.rune"));
    *state.argon_params.lock() = Some(params);
    *state.is_duress.lock() = false;
    *state.search_db.lock() = None; // nowy vault — indeks zbuduje frontend
    key.zeroize();
    let _ = fsutil::acquire_lock(&vdir); // świeży vault — nikt inny go nie trzyma

    Ok(VaultInfo {
        id: id.clone(),
        name,
        path: id,
        last_opened: timeutil::now_iso(),
    })
}

/// Przełącza aktywny vault. Zawsze blokuje bieżącą sesję (zeruje klucz) i
/// ustawia nowy aktywny vault. Zwraca `true`, gdy docelowy vault ma już plik
/// `vault.rune` (front pokaże ekran Unlock) — w praktyce zawsze, bo przełączamy
/// na istniejący wpis listy.
#[tauri::command]
pub async fn switch_vault(vault_id: String, state: State<'_, VaultState>) -> Result<bool, String> {
    let id = sanitize_vault_id(&vault_id);

    // Zwolnij plik-blokadę opuszczanego vaultu (aktywny jeszcze wskazuje stary).
    if let Ok(dir) = active_vault_dir(&state) {
        fsutil::release_lock(&dir);
    }

    {
        let mut guard = state.key.lock();
        if let Some(ref mut k) = *guard {
            k.zeroize();
        }
        *guard = None;
    }
    *state.is_duress.lock() = false;
    *state.active_vault.lock() = id.clone();
    *state.search_db.lock() = None; // porzuć indeks poprzedniego vaultu

    let exists = data_dir_rune()
        .map(|d| d.join(&id).join("vault.rune").exists())
        .unwrap_or(false);
    Ok(exists)
}

/// Usuwa vault wraz z całym katalogiem na dysku oraz wpisem na liście.
/// Nie można usunąć aktywnego vaultu (najpierw trzeba się przełączyć).
#[tauri::command]
pub async fn delete_vault(vault_id: String, state: State<'_, VaultState>) -> Result<(), String> {
    let id = sanitize_vault_id(&vault_id);
    if id == active_vault_id(&state) {
        return Err("Cannot delete the active vault".to_string());
    }

    let base = data_dir_rune().map_err(|_| "Data directory not found".to_string())?;
    let vdir = base.join(&id);
    if vdir.exists() {
        std::fs::remove_dir_all(&vdir).map_err(|_| "Could not delete vault folder".to_string())?;
    }

    let mut list = read_vaults_list();
    list.retain(|v| v.id != id);
    write_vaults_list(&list)?;
    Ok(())
}

/// Zmienia wyświetlaną nazwę vaultu (id i katalog pozostają bez zmian).
#[tauri::command]
pub async fn rename_vault(vault_id: String, new_name: String) -> Result<(), String> {
    let id = sanitize_vault_id(&vault_id);
    let name = new_name.trim().to_string();
    if name.is_empty() {
        return Err("Vault name cannot be empty".to_string());
    }

    let mut list = read_vaults_list();
    let mut found = false;
    for v in list.iter_mut() {
        if v.id == id {
            v.name = name.clone();
            found = true;
        }
    }
    if !found {
        return Err("Vault not found".to_string());
    }
    write_vaults_list(&list)
}
