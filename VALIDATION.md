# MetaClean validation status

Last audited: 2026-08-24

This file records evidence, not intent. A row is complete only when the named artifact or runtime check exists.

## Milestones

| Gate | Status | Evidence |
|---|---|---|
| M0: PDF structural rewrite | Complete | `drops_metadata_bytes_from_incremental_history` proves old Info metadata bytes are absent after full `lopdf` serialization. |
| M0: image/media format decision | Complete | The shared 91-extension allowlist covers native still-image, RAW, audio, video, document, PDF and text/markup cleaners. TIFF/RAW, HEIF/AVIF/CR3, AVI, Matroska/WebM and ASF families use offset-preserving strategies; WAV C2PA and ID3-prefixed FLAC are covered by malformed-input and residual-trace tests. |
| M0: Office package integrity | Partial | Real DOCX/XLSX/PPTX/ODT samples were cleaned and successfully opened/exported by LibreOffice 26.2.5. Current Word and WPS executables are unavailable on this machine, so those two applications remain unverified. |
| M1: desktop MVP | Complete | The fixed 1180 × 720 enterprise workspace provides batch intake, four compact navigation surfaces, command palette, native menus, local status bar, per-file reports and value-free JSON audit export. Safe-copy/replace, backups, atomic writes, fidelity controls, signed update handling and 32 complete locales are implemented and tested. |
| M2: Office/PDF/shell integration | Complete | DOCX/XLSX/PPTX/ODT/EPUB and PDF cleaners, deep PDF JPEG cleanup, embedded markup data-URI cleanup, 91-extension Windows Explorer integration and launch-path handling are covered by unit and manifest-consistency tests. |
| M3: Windows release | Complete for v0.3.0 | The successful v0.3.0 release matrix published launch-smoked x64 NSIS/MSI, x86 NSIS and architecture-labelled x64/x86 portable ZIPs. Local installation/extraction proof also kept each package active for six seconds with the `MetaClean` title before clean uninstall/removal. |
| M3: macOS/Linux release | Complete for unsigned v0.3.0 artifacts | The successful v0.3.0 matrix copied and launch-smoked both Intel and Apple Silicon DMGs, then installed and launch-smoked the Linux DEB before publishing DEB/RPM/AppImage assets. Apple signing/notarization secrets remain unavailable, so Gatekeeper qualification is an external gate rather than a completed claim. |

## Automated quality gates

- Frontend: 186 tests. Statements 89.65%, branches 81.86%, functions 91.33%, lines 92.89%. The queue suite covers audit export as well as sorting, size deltas, errors and reveal actions; status-bar state/version and updater flows are also exercised.
- Rust: 101 tests total: 100 pass and one external Office compatibility test is ignored unless `METACLEAN_OFFICE_SAMPLE_DIR` supplies real fixtures. Strict Clippy passes with warnings denied.
- Installed desktop E2E: all 9 WebdriverIO scenarios pass locally against the rebuilt Windows webview. They cover startup, keyboard navigation, all 32 locale options and RTL, named controls/landmarks, theme and fidelity persistence, the Rust IPC boundary, updater capability and fail-closed missing-input paths.
- Cleanup candidates are re-detected and re-inspected before output-path allocation, backup creation or writes. JPEG/PNG/WebP tests cover both ICC preservation and explicit removal, while an engine regression rejects residual traces and format changes.
- On macOS, every extended attribute is copied by default. Opt-in removal filters only six known provenance/download keys; CI runs a real filesystem round trip proving a private key is removed while a custom key survives.
- CI fails below 80% for all frontend coverage dimensions and below 80% Rust core line coverage.
- `pnpm test:formats` proves the 91-extension Rust intake list, frontend classification, NSIS cleanup, MSI cleanup, both READMEs and support policy are complete, duplicate-free and identical.
- `pnpm test:security` proves production has a non-null local-only WebView CSP, rejects `unsafe-eval`, wildcard sources and unbounded HTTP(S) connections, and limits opener access to the official release URL plus reveal-in-folder.
- `pnpm test:release` runs 30 checks proving version metadata, the 1180 × 720 caption/status layout and tag-specific bilingual notes stay synchronized; package/updater collection, signatures, AppImage zsync, GUI subsystem, `latest.json` and SHA-256 manifests are validated before publication.
- npm's official audit endpoint reports no known dependency vulnerability. WebdriverIO's unfixed `extract-zip` 2.0.1 dependency is replaced on every path by the repository-owned `vendor/extract-zip` 2.0.2 package, which rejects out-of-root symlink targets. Its `deepmerge-ts` chain is forced to 8.0.0 for CVE-2026-40345. `pnpm test:supply-chain` proves both the archive escape and recursive-object denial-of-service regressions are closed. Patched `glob` and `serialize-javascript` versions are also forced through workspace overrides.
- `cargo audit` reports no blocking vulnerability and exits successfully. It emits 17 allowed warnings from inherited GTK/Tauri and Unicode dependency families, including RUSTSEC-2024-0429 in `glib`; these are tracked upstream rather than represented as resolved.
- Production frontend build, TypeScript checking, Rust tests, Rust formatting and both coverage gates pass locally. The production bundle contains no WebdriverIO plugin marker; both Rust test plugins are optional and registered only by the `e2e` Cargo feature.

## v0.6.0 release candidate evidence

- Local gates pass for 186 frontend tests with every coverage dimension above
  80%, 100 Rust tests plus one external-fixture test ignored, strict Clippy,
  production and E2E builds, 9 real desktop E2E scenarios, 30 release tests,
  format/CSP/supply-chain checks and npm audit.
- Deep payload regressions cover embedded PDF JPEG metadata, base64 image data
  URIs, WAV C2PA, ID3-prefixed FLAC and the full contextual Unicode cleaner.
- Public cross-platform packages, updater metadata and checksums remain pending
  until the v0.6.0 tag workflow completes; they are not claimed by this local
  release-candidate section.

## v0.4.0 signed-updater release candidate evidence

- Native Tauri updater discovery replaces the production-blocked WebView fetch while the local-only CSP remains unchanged. The frontend has only `updater:allow-check`; download, verification, installation, tray shutdown and restart are owned by the Rust command.
- The release-only Tauri configuration produced a 4,086,597-byte Windows x64 NSIS installer and a non-empty 420-byte updater signature with the new encrypted key. The installer launched for six seconds and uninstalled cleanly; the 5,897,358-byte portable ZIP contained its runtime marker, launched for six seconds and was removed cleanly.
- Local v0.4.0 gates pass: 115 frontend tests with all coverage dimensions above 80%, 17 release-automation tests, 64 Rust tests (63 passed and one external Office sample test ignored), strict Clippy with warnings denied, production build, formatting/CSP/supply-chain checks, npm official audit with no known vulnerabilities, Cargo audit with the same 17 tracked upstream warnings, and 9 real desktop E2E scenarios.
- Five-platform updater packages, the public `latest.json`, and an end-to-end upgrade from an older installed build cannot be claimed until GitHub Actions secrets are saved and the v0.4.0 release matrix completes.

## Published v0.3.0 release evidence

- Public release: `https://github.com/Moresyl/metaclean/releases/tag/v0.3.0`, pointing to commit `654e1d8faed0e3c8a849ec8cc503b14db9786117`.
- Successful five-platform workflow: `https://github.com/Moresyl/metaclean/actions/runs/32048902129` (attempt 3).
- All ten named platform packages return HTTP 200. The published 954-byte `SHASUMS256.txt` contains exactly ten valid SHA-256 entries, one for every platform package.

## Remaining external release gates

1. Open cleaned DOCX/XLSX/PPTX/ODT samples in current Word and WPS builds. LibreOffice 26.2.5 validation is complete.
2. Provide Apple Developer signing/notarization credentials and verify both DMGs with Gatekeeper.
