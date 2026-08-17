# MetaClean validation status

Last audited: 2026-08-17

This file records evidence, not intent. A row is complete only when the named artifact or runtime check exists.

## Milestones

| Gate | Status | Evidence |
|---|---|---|
| M0: PDF structural rewrite | Complete | `drops_metadata_bytes_from_incremental_history` proves old Info metadata bytes are absent after full `lopdf` serialization. |
| M0: image/media format decision | Complete | JPEG, PNG, WebP and GIF; MP3, WAV and FLAC; and MP4/MOV/M4V/M4A have container-level cleanup and malformed-input tests. ISO BMFF cleanup preserves file length and media offsets. TIFF, HEIC, RAW and unimplemented media containers are explicitly unsupported rather than being modified unsafely. |
| M0: Office package integrity | Partial | Real DOCX/XLSX/PPTX/ODT samples were cleaned and successfully opened/exported by LibreOffice 26.2.5. Current Word and WPS executables are unavailable on this machine, so those two applications remain unverified. |
| M1: desktop MVP | Complete | Batch drag/drop, recursive file/folder intake, native pickers, per-file scan reports, safe-copy/replace modes, forced backup, atomic writes, version discovery and twenty-six complete interface locales are implemented and tested. Every non-source catalog covers all 111 static strings and 13 dynamic message paths; Arabic direction switching has unit and installed-app E2E coverage. |
| M2: Office/PDF/shell integration | Complete | DOCX/XLSX/PPTX/ODT/PDF cleaners, 26-extension Windows Explorer integration and launch-path handling are covered by unit and installer tests. |
| M3: Windows release | Complete | Optimized v0.2.0 NSIS and MSI packages build locally and were published by the successful GitHub release matrix. The release executable starts, remains running for a six-second smoke window and exposes the expected `MetaClean` main-window title. The v0.1.0 installer lifecycle test previously proved install, tray behavior, shell registration cleanup and uninstall cleanup; the same WiX/NSIS integration is retained and expanded to 22 extensions. |
| M3: macOS/Linux release | Partial | The successful v0.2.0 GitHub matrix published Intel and Apple Silicon DMGs plus DEB/RPM/AppImage packages. Apple signing/notarization secrets are unavailable, so the DMGs are currently unsigned and Gatekeeper verification remains external. |

## Automated quality gates

- Frontend: 62 tests. Statements 94.49%, branches 87.65%, functions 94.96%, lines 97.67%.
- Rust: 51 regular tests plus one ignored external Office compatibility test. Core regions 81.22%, lines 80.77% after excluding Tauri/platform glue (`lib`, `main`, data-only `models` and `shell_integration`) from the line threshold; cleaners, intake, engine and safe I/O remain included.
- Installed desktop E2E: 4 WebdriverIO scenarios pass against E2E-featured Windows, macOS and Linux/Xvfb binaries, covering startup/navigation, all 26 locale options and Arabic RTL, theme persistence across a real webview reload, and the Rust `scan_files` IPC boundary. GitHub CI #19 completed the base gate and all three desktop jobs successfully in 8m58s: `https://github.com/Moresyl/metaclean/actions/runs/32031760324`.
- CI fails below 80% for all frontend coverage dimensions and below 80% Rust core line coverage.
- npm's official audit endpoint reports no known dependency vulnerability. WebdriverIO's unfixed `extract-zip` 2.0.1 dependency is replaced on every path by the repository-owned `vendor/extract-zip` 2.0.2 package, which rejects out-of-root symlink targets; `pnpm test:supply-chain` proves a malicious archive cannot create the link. Patched `glob` and `serialize-javascript` versions are also forced through workspace overrides.
- `cargo audit` reports no blocking vulnerability and exits successfully. It emits 17 allowed warnings from inherited GTK/Tauri and Unicode dependency families, including RUSTSEC-2024-0429 in `glib`; these are tracked upstream rather than represented as resolved.
- Production frontend build, TypeScript checking, Rust tests, Rust formatting and both coverage gates pass locally. The production bundle contains no WebdriverIO plugin marker; both Rust test plugins are optional and registered only by the `e2e` Cargo feature.

## Windows release artifacts

- `src-tauri/target/release/bundle/nsis/MetaClean_0.2.0_x64-setup.exe` (2,958,274 bytes; SHA-256 `424245B70E4883C642B0EE55BAAB2EDEDFDAD099B22DF04689ED317030F46E01`)
- `src-tauri/target/release/bundle/msi/MetaClean_0.2.0_x64_en-US.msi` (4,419,584 bytes; SHA-256 `5CDB8500F767AE93184AAF993F102683F39515B73F574033754ED78C8CEE0A12`)

## Remaining external release gates

1. Open cleaned DOCX/XLSX/PPTX/ODT samples in current Word and WPS builds. LibreOffice 26.2.5 validation is complete.
2. Provide Apple Developer signing/notarization credentials and verify both DMGs with Gatekeeper.
