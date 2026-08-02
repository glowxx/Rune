//! Lekkie formatowanie czasu UTC bez zewnętrznych zależności (algorytm Howarda
//! Hinnanta `civil_from_days`). Używane do nazw backupów i znaczników eksportu.

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// (rok, miesiąc, dzień) z liczby dni od ery (1970-01-01 = 0).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn parts(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let (y, m, d) = civil_from_days(days);
    (
        y,
        m,
        d,
        (rem / 3600) as u32,
        (rem % 3600 / 60) as u32,
        (rem % 60) as u32,
    )
}

/// Znacznik kompaktowy do nazw plików: `YYYY-MM-DD-HH-MM-SS` (UTC).
pub fn now_compact() -> String {
    let (y, mo, d, h, mi, s) = parts(now_secs());
    format!("{y:04}-{mo:02}-{d:02}-{h:02}-{mi:02}-{s:02}")
}

/// Znacznik ISO 8601 (UTC): `YYYY-MM-DDTHH:MM:SSZ`.
pub fn now_iso() -> String {
    let (y, mo, d, h, mi, s) = parts(now_secs());
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// ISO 8601 z dowolnych sekund unix (do `BackupInfo.created_at`).
pub fn iso_from_secs(secs: u64) -> String {
    let (y, mo, d, h, mi, s) = parts(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}
