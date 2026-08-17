# MetaClean validation status

Last audited: 2026-08-18

This file records evidence, not intent. A row is complete only when the named artifact or runtime check exists.

## Milestones

| Gate | Status | Evidence |
|---|---|---|
| M0: PDF structural rewrite | Complete | `drops_metadata_bytes_from_incremental_history` proves old Info metadata bytes are absent after full `lopdf` serialization. |
| M0: image/media format decision | Complete | JPEG/JPE, PNG, WebP and GIF; MP3, WAV and FLAC; and 17 validated ISO BMFF/QuickTime extensions have container-level cleanup and malformed-input tests. ISO media cleanup preserves file length and media offsets. TIFF, HEIC, RAW and unimplemented containers are explicitly unsupported rather than being modified unsafely. |
| M0: Office package integrity | Partial | Real DOCX/XLSX/PPTX/ODT samples were cleaned and successfully opened/exported by LibreOffice 26.2.5. Current Word and WPS executables are unavailable on this machine, so those two applications remain unverified. |
| M1: desktop MVP | Complete | Batch drag/drop, recursive file/folder intake, native pickers, per-file scan reports, stable multi-key queue sorting, source/output size deltas, scoped reveal-in-folder actions, native menus/accelerators, window-state restore, safe-copy/replace modes, forced backup, atomic writes, candidate-byte verification, independently persisted ICC/sRGB and macOS xattr controls, native signed update discovery/install and twenty-six complete interface locales are implemented and tested. Every non-source catalog covers all 111 static strings and 13 dynamic message paths; Arabic direction switching has unit and installed-app E2E coverage. |
| M2: Office/PDF/shell integration | Complete | DOCX/XLSX/PPTX/ODT/PDF cleaners, 47-extension Windows Explorer integration and launch-path handling are covered by unit and format-manifest consistency tests. Runtime registration reuses the Rust intake list; NSIS and MSI uninstall manifests are checked against it. |
| M3: Windows release | Complete for v0.3.0 | The successful v0.3.0 release matrix published launch-smoked x64 NSIS/MSI, x86 NSIS and architecture-labelled x64/x86 portable ZIPs. Local installation/extraction proof also kept each package active for six seconds with the `MetaClean` title before clean uninstall/removal. |
| M3: macOS/Linux release | Complete for unsigned v0.3.0 artifacts | The successful v0.3.0 matrix copied and launch-smoked both Intel and Apple Silicon DMGs, then installed and launch-smoked the Linux DEB before publishing DEB/RPM/AppImage assets. Apple signing/notarization secrets remain unavailable, so Gatekeeper qualification is an external gate rather than a completed claim. |

## Automated quality gates

- Frontend: 115 tests. Statements 92.88%, branches 84.61%, functions 94.75%, lines 95.94%. Every supported extension has its own case-insensitive classification case; queue tests cover stable sorting, size deltas, cleanup errors and output reveal, while update tests cover native signed discovery, progress, listener cleanup, runtime capability and portable fallback.
- Rust: 63 regular tests plus one ignored external Office compatibility test. Core regions retain the 80% coverage gate; cleaners, intake, engine and safe I/O remain included. Strict Clippy passes with warnings denied, including the updater runtime tests and cleanup of the prior complex-type and loop-compiled-regex warnings.
- Installed desktop E2E: 9 WebdriverIO scenarios cover startup, keyboard navigation, all 26 locale options and Arabic RTL, named controls/landmarks, theme plus ICC/xattr preference persistence, the Rust IPC boundary, updater runtime capability and fail-closed missing-input scan/cleanup. All nine pass locally; the v0.3.0 three-family baseline remains CI #27 at `https://github.com/Moresyl/metaclean/actions/runs/32041289728`, while the v0.4.0 cross-platform run is pending publication.
- Cleanup candidates are re-detected and re-inspected before output-path allocation, backup creation or writes. JPEG/PNG/WebP tests cover both ICC preservation and explicit removal, while an engine regression rejects residual traces and format changes.
- On macOS, every extended attribute is copied by default. Opt-in removal filters only six known provenance/download keys; CI runs a real filesystem round trip proving a private key is removed while a custom key survives.
- CI fails below 80% for all frontend coverage dimensions and below 80% Rust core line coverage.
- `pnpm test:formats` proves the 47-extension Rust intake list, frontend classification, NSIS cleanup and MSI cleanup are complete, duplicate-free and identical.
- `pnpm test:security` proves production has a non-null local-only WebView CSP, rejects `unsafe-eval`, wildcard sources and unbounded HTTP(S) connections, and limits opener access to the official release URL plus reveal-in-folder.
- `pnpm test:release` runs 17 checks proving version metadata stays synchronized, tag-specific bilingual notes contain every required section, normal and updater assets are collected without omissions/duplicates, Tauri minisign payloads are structurally validated, `latest.json` covers five exact updater targets, and SHA-256 manifests are deterministic, complete and self-excluding. Release run #7 (`32048902129`, successful attempt 3) remains the published v0.3.0 baseline.
- npm's official audit endpoint reports no known dependency vulnerability. WebdriverIO's unfixed `extract-zip` 2.0.1 dependency is replaced on every path by the repository-owned `vendor/extract-zip` 2.0.2 package, which rejects out-of-root symlink targets. Its `deepmerge-ts` chain is forced to 8.0.0 for CVE-2026-40345. `pnpm test:supply-chain` proves both the archive escape and recursive-object denial-of-service regressions are closed. Patched `glob` and `serialize-javascript` versions are also forced through workspace overrides.
- `cargo audit` reports no blocking vulnerability and exits successfully. It emits 17 allowed warnings from inherited GTK/Tauri and Unicode dependency families, including RUSTSEC-2024-0429 in `glib`; these are tracked upstream rather than represented as resolved.
- Production frontend build, TypeScript checking, Rust tests, Rust formatting and both coverage gates pass locally. The production bundle contains no WebdriverIO plugin marker; both Rust test plugins are optional and registered only by the `e2e` Cargo feature.

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
