# Contributing

## Setup

Install Node.js 20+, Rust stable, `cargo-audit`, and your platform's native
Tauri build tools.

```bash
cargo install cargo-audit --locked
npm ci
npm run tauri dev
```

Before submitting a pull request, run:

```bash
npm run verify
```

`verify` runs the frontend type check and tests, Rust tests, formatting and
Clippy, npm and RustSec dependency vulnerability gates, and a production
frontend build.

Keep changes focused. Do not log passwords, keys, decrypted content, or real
vault data. Security-sensitive changes require tests and an explanation of their
impact on key handling, encryption, or the vault format.
