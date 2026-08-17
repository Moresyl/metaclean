# MetaClean validation status

Last audited: 2026-08-17

This file records evidence, not intent. A row is complete only when the named artifact or runtime check exists.

## Milestones

| Gate | Status | Evidence |
|---|---|---|
| M0: PDF structural rewrite | Complete | `drops_metadata_bytes_from_incremental_history` proves old Info metadata bytes are absent after full `lopdf` serialization. |
| M0: image/media format decision | Complete | JPEG/JPE, PNG, WebP and GIF; MP3, WAV and FLAC; and 17 validated ISO BMFF/QuickTime extensions have container-level cleanup and malformed-input tests. ISO media cleanup preserves file length and media offsets. TIFF, HEIC, RAW and unimplemented containers are explicitly unsupported rather than being modified unsafely. |
| M0: Office package integrity | Partial | Real DOCX/XLSX/PPTX/ODT samples were cleaned and successfully opened/exported by LibreOffice 26.2.5. Current Word and WPS executables are unavailable on this machine, so those two applications remain unverified. |
| M1: desktop MVP | Complete | Batch drag/drop, recursive file/folder intake, native pickers, per-file scan reports, safe-copy/replace modes, forced backup, atomic writes, candidate-byte verification, independently persisted ICC/sRGB fidelity control, version discovery and twenty-six complete interface locales are implemented and tested. Every non-source catalog covers all 111 static strings and 13 dynamic message paths; Arabic direction switching has unit and installed-app E2E coverage. |
| M2: Office/PDF/shell integration | Complete | DOCX/XLSX/PPTX/ODT/PDF cleaners, 47-extension Windows Explorer integration and launch-path handling are covered by unit and format-manifest consistency tests. Runtime registration reuses the Rust intake list; NSIS and MSI uninstall manifests are checked against it. |
| M3: Windows release | Complete for v0.2.0; current branch pending next release | Optimized v0.2.0 NSIS and MSI packages build locally and were published by the successful GitHub release matrix. The release executable starts, remains running for a six-second smoke window and exposes the expected `MetaClean` main-window title. The v0.1.0 installer lifecycle test proved install, tray behavior, shell registration cleanup and uninstall cleanup. The current branch expands both installer cleanup paths to 47 extensions and requires a new release artifact before that expansion is considered shipped. |
| M3: macOS/Linux release | Partial | The successful v0.2.0 GitHub matrix published Intel and Apple Silicon DMGs plus DEB/RPM/AppImage packages. Apple signing/notarization secrets are unavailable, so the DMGs are currently unsigned and Gatekeeper verification remains external. |

## Automated quality gates

- Frontend: 108 tests. Statements 94.55%, branches 88.16%, functions 95.03%, lines 97.85%. Every supported extension has its own case-insensitive classification case.
- Rust: 59 regular tests plus one ignored external Office compatibility test. Core regions 80.54%, lines 80.04% after excluding Tauri/platform glue (`lib`, `main`, data-only `models` and `shell_integration`) from the line threshold; cleaners, intake, engine and safe I/O remain included. Every ISO media and UTF-8 text alias traverses scan and clean through the public engine path.
- Installed desktop E2E: 5 WebdriverIO scenarios cover startup/navigation, all 26 locale options and Arabic RTL, theme and ICC preference persistence across real webview reloads, and the Rust `scan_files` IPC boundary. CI #21 passed the preceding 4-scenario/104-frontend-test baseline on Windows, Ubuntu and macOS in 5m40s: `https://github.com/Moresyl/metaclean/actions/runs/32034874179`; the expanded scenario is pending the current branch CI run.
- Cleanup candidates are re-detected and re-inspected before output-path allocation, backup creation or writes. JPEG/PNG/WebP tests cover both ICC preservation and explicit removal, while an engine regression rejects residual traces and format changes.
- CI fails below 80% for all frontend coverage dimensions and below 80% Rust core line coverage.
- `pnpm test:formats` proves the 47-extension Rust intake list, frontend classification, NSIS cleanup and MSI cleanup are complete, duplicate-free and identical.
- `pnpm test:security` proves production has a non-null local-only WebView CSP and rejects `unsafe-eval`, wildcard sources and unbounded HTTP(S) connections.
- `pnpm test:release` proves tag-specific bilingual notes contain every required section and SHA-256 manifests are deterministic, complete and self-excluding. The release workflow downloads all matrix assets only after every build succeeds, then uploads `SHASUMS256.txt`.
- npm's official audit endpoint reports no known dependency vulnerability. WebdriverIO's unfixed `extract-zip` 2.0.1 dependency is replaced on every path by the repository-owned `vendor/extract-zip` 2.0.2 package, which rejects out-of-root symlink targets; `pnpm test:supply-chain` proves a malicious archive cannot create the link. Patched `glob` and `serialize-javascript` versions are also forced through workspace overrides.
- `cargo audit` reports no blocking vulnerability and exits successfully. It emits 17 allowed warnings from inherited GTK/Tauri and Unicode dependency families, including RUSTSEC-2024-0429 in `glib`; these are tracked upstream rather than represented as resolved.
- Production frontend build, TypeScript checking, Rust tests, Rust formatting and both coverage gates pass locally. The production bundle contains no WebdriverIO plugin marker; both Rust test plugins are optional and registered only by the `e2e` Cargo feature.

## Windows release artifacts

- `src-tauri/target/release/bundle/nsis/MetaClean_0.2.0_x64-setup.exe` (2,958,274 bytes; SHA-256 `424245B70E4883C642B0EE55BAAB2EDEDFDAD099B22DF04689ED317030F46E01`)
- `src-tauri/target/release/bundle/msi/MetaClean_0.2.0_x64_en-US.msi` (4,419,584 bytes; SHA-256 `5CDB8500F767AE93184AAF993F102683F39515B73F574033754ED78C8CEE0A12`)

## Remaining external release gates

1. Open cleaned DOCX/XLSX/PPTX/ODT samples in current Word and WPS builds. LibreOffice 26.2.5 validation is complete.
2. Provide Apple Developer signing/notarization credentials and verify both DMGs with Gatekeeper.
