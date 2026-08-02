//! Lokalny auto-backup: kopiuje cały katalog `%APPDATA%/rune/` (już zaszyfrowane
//! pliki) do wybranego folderu. Hasło nie jest potrzebne — kopiujemy szyfrogramy.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::commands::timeutil;
use crate::commands::vault::data_dir_rune;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    pub backup_path: String,
    pub files_copied: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub path: String,
    pub created_at: String,
    pub size_bytes: u64,
}

/// Rekurencyjnie kopiuje katalog. Zwraca liczbę skopiowanych plików.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<u32> {
    std::fs::create_dir_all(dst)?;
    let mut count = 0;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            count += copy_dir_all(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
            count += 1;
        }
    }
    Ok(count)
}

fn dir_size(p: &Path) -> u64 {
    let mut total = 0;
    if let Ok(rd) = std::fs::read_dir(p) {
        for e in rd.flatten() {
            match e.file_type() {
                Ok(ft) if ft.is_dir() => total += dir_size(&e.path()),
                Ok(_) => {
                    if let Ok(m) = e.metadata() {
                        total += m.len();
                    }
                }
                Err(_) => {}
            }
        }
    }
    total
}

/// Katalogi backupu posortowane chronologicznie (nazwa = znacznik czasu).
fn list_backup_dirs(base: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(rd) = std::fs::read_dir(base) {
        for e in rd.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                && e.file_name().to_string_lossy().starts_with("rune-backup-")
            {
                v.push(e.path());
            }
        }
    }
    v.sort();
    v
}

/// Usuwa najstarsze backupy ponad `keep_last` (0 = bez limitu).
fn prune(base: &Path, keep_last: u32) {
    if keep_last == 0 {
        return;
    }
    let mut dirs = list_backup_dirs(base);
    while dirs.len() > keep_last as usize {
        let oldest = dirs.remove(0);
        let _ = std::fs::remove_dir_all(&oldest);
    }
}

#[tauri::command]
pub async fn perform_backup(backup_dir: String, keep_last: u32) -> Result<BackupResult, String> {
    let src = data_dir_rune().map_err(|_| "Data directory not found".to_string())?;
    if !src.exists() {
        return Err("Nothing to back up".to_string());
    }

    let base = PathBuf::from(&backup_dir);
    std::fs::create_dir_all(&base).map_err(|_| "Could not create backup directory".to_string())?;

    let dest = base.join(format!("rune-backup-{}", timeutil::now_compact()));
    let files_copied = copy_dir_all(&src, &dest).map_err(|_| "Backup copy failed".to_string())?;

    prune(&base, keep_last);

    Ok(BackupResult {
        backup_path: dest.to_string_lossy().to_string(),
        files_copied,
    })
}

#[tauri::command]
pub async fn get_backup_list(backup_dir: String) -> Result<Vec<BackupInfo>, String> {
    let base = PathBuf::from(&backup_dir);
    let mut out = Vec::new();
    for d in list_backup_dirs(&base) {
        let created_at = d
            .metadata()
            .ok()
            .and_then(|m| m.created().or_else(|_| m.modified()).ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|x| timeutil::iso_from_secs(x.as_secs()))
            .unwrap_or_default();
        out.push(BackupInfo {
            path: d.to_string_lossy().to_string(),
            created_at,
            size_bytes: dir_size(&d),
        });
    }
    out.reverse(); // najnowsze pierwsze
    Ok(out)
}
