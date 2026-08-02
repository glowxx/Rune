#!/bin/bash
set -e

echo "================================"
echo " Rune — Installer Builder"
echo "================================"
echo

# Check deps
command -v node >/dev/null 2>&1 || { echo "[ERROR] Node.js not found."; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "[ERROR] Rust/Cargo not found."; exit 1; }

echo "[1/3] Installing frontend dependencies..."
npm ci

echo "[2/3] Building Tauri installer..."
npm run tauri build

echo "[3/3] Done!"
echo
echo "Installer location: src-tauri/target/release/bundle/"
ls src-tauri/target/release/bundle/
