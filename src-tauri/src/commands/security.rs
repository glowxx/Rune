//! Anti-screenshot: wykluczenie okna z przechwytywania ekranu.
//!
//! Na Windows używa `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` — okno
//! pozostaje widoczne dla użytkownika na monitorze, ale jest czarne/puste w
//! zrzutach ekranu, nagraniach i udostępnianiu pulpitu. Na innych platformach
//! komendy są no-op (zwracają sukces), a `capture_protection_supported` = false.

/// Czy bieżąca platforma wspiera ochronę przed przechwytywaniem (tylko Windows).
#[tauri::command]
pub fn capture_protection_supported() -> bool {
    cfg!(target_os = "windows")
}

/// Włącza/wyłącza wykluczenie okna z przechwytywania ekranu.
#[tauri::command]
pub async fn toggle_capture_protection(
    window: tauri::WebviewWindow,
    enabled: bool,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        set_affinity(&window, enabled)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, enabled); // brak wsparcia poza Windows — no-op
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn set_affinity(window: &tauri::WebviewWindow, enabled: bool) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
    };

    let hwnd = HWND(window.hwnd().map_err(|e| e.to_string())?.0);
    let affinity = if enabled {
        WDA_EXCLUDEFROMCAPTURE
    } else {
        WDA_NONE
    };
    // SAFETY: HWND pochodzi z żywego okna Tauri; wywołanie to czysty FFI do Win32.
    unsafe {
        SetWindowDisplayAffinity(hwnd, affinity).map_err(|e| e.to_string())?;
    }
    Ok(())
}
