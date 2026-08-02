# Security policy

Rune is an encryption application. Do not disclose vulnerabilities in public
issues or include passwords, plaintext notes, vaults, or exports in reports.

Use GitHub's private vulnerability reporting in the repository's **Security**
tab. If it is unavailable, open a public issue requesting a private contact
channel without technical details.

Include the affected version and platform, impact, and reproducible steps.

## Scope and limitations

Security fixes are applied to the current release line. Rune has not had an
independent security audit. It protects encrypted data at rest, not a system
compromised by malware, keylogging, or memory inspection of an unlocked vault.

Linux builds inherit GTK3/WebKitGTK dependencies from Tauri. RustSec currently
reports informational maintenance and soundness notices for some of those
target-specific crates. The automated security gate fails on vulnerabilities;
these upstream informational notices remain visible for tracking until Tauri's
Linux runtime no longer depends on the affected GTK3 generation.
