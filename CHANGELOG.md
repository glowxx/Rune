# Changelog

All notable changes to Rune are documented here. This project follows Semantic
Versioning.

## [0.1.0] - 2026-07-13

### Added

- Initial public release: encrypted local-first notes, attachments, whiteboards,
  search, organizational views, exports, imports, and local backups.
- Project documentation, security-reporting policy, verification scripts, and CI.
- Frontend security tests and a single `npm run verify` release-quality gate.

### Security

- Sanitized Markdown previews and print exports with DOMPurify to prevent stored
  script injection from note content.
- Replaced poisoning `std::sync::Mutex` state locks with `parking_lot::Mutex`.
- Added direct Argon2/AES-GCM round-trip, tamper, wrong-key, and nonce tests.
- Restricted vault import/export and DOCX export to absolute paths with the
  expected file extensions.
- Updated XML-processing dependencies to `quick-xml 0.41` or newer, addressing
  the RustSec CPU- and memory-exhaustion advisories.

### Changed

- Updated SvelteKit, Vite, CodeMirror, Tauri CLI, and related dependencies to
  remove all known moderate, high, and critical npm advisories.
- Split editor, graph, and Markdown dependencies into separate production chunks.
- Added RustSec auditing to the local verification command and GitHub Actions.
