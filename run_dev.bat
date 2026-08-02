@echo off
REM ============================================================
REM  Rune - uruchomienie w trybie deweloperskim (Tauri + Vite)
REM ============================================================
REM  Startuje serwer Vite i okno aplikacji Tauri z hot-reloadem.
REM  Zamkniecie okna lub Ctrl+C konczy sesje.

setlocal

REM Przejdz do katalogu, w ktorym lezy ten plik (korzen projektu).
cd /d "%~dp0"

REM Pierwsze uruchomienie? Doinstaluj zaleznosci frontendu.
if not exist "node_modules" (
    echo [run_dev] Brak node_modules - instaluje zaleznosci...
    call npm install
    if errorlevel 1 (
        echo [run_dev] npm install nie powiodlo sie.
        pause
        exit /b 1
    )
)

echo [run_dev] Uruchamiam Rune w trybie dev ^(npm run tauri dev^)...
call npm run tauri dev

REM Zatrzymaj okno otwarte, jesli aplikacja wyjdzie z bledem.
if errorlevel 1 (
    echo.
    echo [run_dev] Aplikacja zakonczyla sie bledem ^(kod %errorlevel%^).
    pause
)

endlocal
