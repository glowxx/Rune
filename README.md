# Rune

**A private, local-first workspace for encrypted notes, connected knowledge, and focused work.**

Rune is a cross-platform desktop application built with Tauri, Svelte, and Rust. It keeps your vaults on your device, encrypts their contents at rest, and combines Markdown writing with backlinks, graph navigation, Kanban planning, calendar views, whiteboards, encrypted attachments, backups, and time tracking.

No account is required. Rune has no cloud sync, advertising, or telemetry.

[Releases](https://github.com/glowxx/Rune/releases) · [Changelog](CHANGELOG.md) · [Report a bug](https://github.com/glowxx/Rune/issues/new?template=bug_report.md) · [Request a feature](https://github.com/glowxx/Rune/issues/new?template=feature_request.md) · [Security](SECURITY.md)

> [!IMPORTANT]
> Rune is currently at version **0.1.0**. The project has not received an independent security audit. Read the [security model and limitations](#security-model) before relying on Rune for sensitive or high-risk data.

## Contents

- [Why Rune](#why-rune)
- [Features](#features)
- [Security model](#security-model)
- [Network and privacy](#network-and-privacy)
- [Getting started](#getting-started)
- [Keyboard shortcuts](#keyboard-shortcuts)
- [Development](#development)
- [Build installers](#build-installers)
- [Project structure](#project-structure)
- [Contributing and security reports](#contributing-and-security-reports)

## Why Rune

Rune is designed for people who want the convenience of a modern knowledge workspace without moving their notes to a hosted service.

- **Local ownership** — vault data stays in the operating system's application-data directory.
- **Encryption by default** — notes, attachments, whiteboards, and organizational data are encrypted on disk.
- **Connected thinking** — wikilinks, backlinks, tags, and an interactive graph connect related notes.
- **One focused workspace** — writing, planning, visual thinking, calendar review, and time tracking live in the same application.
- **Portable data** — encrypted vault export/import, local backups, and per-note document export are built in.

## Features

### Write and connect

- Markdown editor powered by CodeMirror 6, with formatting commands and syntax support.
- Edit, rendered preview, and split-view modes.
- `[[wikilinks]]`, contextual backlinks, tags, folders, pinned notes, and note duplication.
- Full-text search with contextual snippets; the FTS5 index exists only in memory while a vault is unlocked.
- Interactive D3 knowledge graph generated from links between notes.
- Focus mode for distraction-free writing.

### Organize and plan

- Multiple isolated vaults with creation, switching, renaming, and deletion workflows.
- Customizable Kanban boards backed by existing notes.
- Calendar view that discovers dates and task progress inside note content.
- Optional per-note timers, time logs, and cumulative summaries.
- Encrypted file attachments with inline image and PDF support.
- Excalidraw whiteboards associated with notes.

### Protect and move your data

- Adjustable Argon2id key-derivation presets for different hardware profiles.
- Manual lock, inactivity lock, and an always-available panic-lock shortcut.
- Optional duress password that opens a separate decoy note area.
- Optional screen-capture protection on supported Windows systems.
- Portable, password-encrypted `.vault` export and merge-based import.
- Scheduled local backups with configurable frequency and retention.
- Individual note export to Word (`.docx`) or print-ready HTML for PDF output.
- Built-in dark, light, and custom color themes.

## Security model

Rune is built to protect vault data **at rest**. Encryption and key handling are implemented in the Rust backend rather than in the webview.

| Area | Implementation |
| --- | --- |
| Content encryption | AES-256-GCM authenticated encryption with a fresh random nonce for each encrypted payload |
| Password derivation | Argon2id with a random 32-byte salt and selectable memory, iteration, and parallelism costs |
| Key lifetime | The active 256-bit vault key is held in process memory only while the vault is unlocked and is zeroed when locked |
| Search | SQLite FTS5 index created in memory after unlock and discarded on lock |
| Markdown rendering | Untrusted note HTML is sanitized with DOMPurify before preview or print export |
| Writes | Atomic temporary-file replacement and a per-vault process lock reduce corruption and concurrent-write risks |
| Exports | Portable vault exports are encrypted independently and can use a password different from the active vault |
| Backups | Copies contain the already-encrypted on-disk vault data |

Rune does **not** protect against:

- malware, keyloggers, or a compromised operating system;
- memory inspection while a vault is unlocked;
- physical observation of the screen;
- loss of both the primary data and every backup;
- a weak or reused vault password.

There is no password recovery service and no remote account that can reset a vault. Keep your password and verified backups safe. See [SECURITY.md](SECURITY.md) for the full reporting policy and current security scope.

## Network and privacy

The core application does not require an account or contact a Rune service. There is no telemetry or cloud synchronization, and normal note, search, planning, export, and backup workflows are local.

The whiteboard view is the current exception: it loads pinned React and Excalidraw assets from `unpkg.com`. Consequently, whiteboards require network access when those assets are not available, and the domain is explicitly permitted by Rune's Content Security Policy. No vault content is intentionally sent to that service.

## Getting started

For packaged builds, check [GitHub Releases](https://github.com/glowxx/Rune/releases). To run Rune from source, install:

- [Node.js](https://nodejs.org/) 20 or newer;
- the [Rust stable toolchain](https://rustup.rs/);
- the [native prerequisites required by Tauri 2](https://v2.tauri.app/start/prerequisites/) for your operating system.

Clone and start the desktop application:

```bash
git clone https://github.com/glowxx/Rune.git
cd Rune
npm ci
npm run tauri dev
```

On Windows, `run_dev.bat` provides the same development startup flow and installs frontend dependencies when needed.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl/Cmd + N` | Create a note |
| `Ctrl + Shift + N` | Create a folder |
| `Ctrl/Cmd + W` | Close the active note |
| `Ctrl + Shift + F` | Toggle focus mode |
| `Ctrl + Shift + L` | Panic lock immediately |
| `Ctrl/Cmd + B` | Bold text |
| `Ctrl/Cmd + I` | Italic text |
| `Ctrl/Cmd + K` | Insert a link |
| `Esc` | Close the active overlay or leave focus mode |

## Development

### Technology

| Layer | Stack | Responsibility |
| --- | --- | --- |
| Desktop shell | Tauri 2 | Native windowing, capability boundaries, packaging, and IPC |
| Frontend | Svelte 5, SvelteKit, TypeScript | Application UI, editor state, and user workflows |
| Backend | Rust | Cryptography, vault lifecycle, file operations, search, export, and backup |
| Editor | CodeMirror 6, Marked | Markdown editing and rendered preview |
| Visualization | D3, Excalidraw | Note graph and whiteboards |
| Search | SQLite FTS5 via `rusqlite` | In-memory full-text index |

### Commands

The complete local quality gate also requires
[`cargo-audit`](https://github.com/rustsec/rustsec/tree/main/cargo-audit). Install
it once with `cargo install cargo-audit --locked`.

| Command | Purpose |
| --- | --- |
| `npm run dev` | Start the Vite frontend only |
| `npm run tauri dev` | Start Rune with the Rust backend and frontend hot reload |
| `npm run check` | Synchronize SvelteKit and run TypeScript/Svelte checks |
| `npm run test:frontend` | Run frontend security and utility tests with Vitest |
| `npm run test:rust` | Run the Rust test suite |
| `npm run format:rust` | Verify Rust formatting |
| `npm run lint:rust` | Run Clippy and treat warnings as errors |
| `npm run audit:dependencies` | Fail on moderate, high, or critical npm advisories |
| `npm run audit:rust` | Check Cargo.lock against the current RustSec advisory database |
| `npm run verify` | Run the complete local release-quality gate |
| `npm run build` | Build the static frontend |
| `npm run tauri build` | Build and package the desktop application |

Before opening a pull request, run the same quality gate used by CI:

```bash
npm run verify
```

CI repeats these checks and builds Rune on Windows, Linux, and macOS.

## Build installers

Build the platform-appropriate Tauri packages:

```bash
npm ci
npm run tauri build
```

Artifacts are written under `src-tauri/target/release/bundle/`.

Convenience scripts are also included:

- `build-installer.bat` builds an MSI package on Windows.
- `build-installer.sh` builds the packages supported by the current Unix-like platform.
- `npm run build:installer` is the direct Windows MSI command.

## Project structure

```text
Rune/
├── src/                         # Svelte application
│   ├── lib/api/                 # Typed wrappers around Tauri commands
│   ├── lib/components/          # Editor, navigation, views, and dialogs
│   ├── lib/stores/              # Client-side application state
│   └── routes/                  # SvelteKit desktop entry route
├── src-tauri/                   # Rust backend and desktop packaging
│   ├── src/commands/            # Vault, notes, crypto-adjacent workflows, and I/O
│   ├── capabilities/            # Tauri capability declarations
│   └── icons/                   # Application icons
├── static/                      # Static runtime assets
├── docs/                        # Project documentation and image assets
└── .github/                     # CI, Dependabot, and issue templates
```

The frontend never reads vault files directly. It calls a narrow set of Tauri commands; the Rust layer owns key derivation, encryption, validation, persistence, search indexing, and data portability.

## Contributing and security reports

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting a change. Cryptography, key handling, vault-format changes, and other security-sensitive work require additional review and tests.

Do not disclose vulnerabilities in public issues. Follow the private process described in [SECURITY.md](SECURITY.md).

## License

Rune is available under the [MIT License](LICENSE).
