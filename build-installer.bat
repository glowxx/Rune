@echo off
setlocal

:: Always run from the project root, even when launched from a shortcut.
cd /d "%~dp0"

echo ================================
echo  Rune - MSI Installer Builder
echo ================================
echo.

:: Check Node
where node >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Node.js not found. Install from https://nodejs.org
    pause & exit /b 1
)

:: Check Cargo
where cargo >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Rust/Cargo not found. Install from https://rustup.rs
    pause & exit /b 1
)

:: Vite/Rolldown loads a native .node file which Windows cannot replace while
:: the Rune development server is still running. Stop the Node process serving
:: Rune's fixed development port; unrelated Node apps are ignored.
echo [preflight] Checking for running Rune development processes...
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\stop-project-node-processes.ps1" -ProjectRoot "%CD%"
if errorlevel 1 (
    echo [ERROR] Could not check or stop Rune development processes.
    pause & exit /b 1
)

echo [1/3] Installing frontend dependencies...
call npm ci
if errorlevel 1 (
    echo [WARN] npm ci failed. Waiting for Windows to release file handles and retrying...
    timeout /t 2 /nobreak >nul
    call npm ci
    if errorlevel 1 (
        echo [ERROR] npm ci failed after retry.
        echo [HINT] Close Rune, terminals running npm/Vite, and temporarily check antivirus quarantine.
        pause & exit /b 1
    )
)

echo [2/3] Building Tauri MSI installer...
call npm run tauri build -- --bundles msi
if errorlevel 1 (
    echo [ERROR] Build failed. Check output above.
    pause & exit /b 1
)

echo [3/3] Done!
echo.
echo Installer location:
echo   src-tauri\target\release\bundle\msi\
echo.

:: Open the output folder
start "" "src-tauri\target\release\bundle\msi\"

pause
endlocal
