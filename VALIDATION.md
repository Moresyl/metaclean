# MetaClean validation status

Last audited: 2026-08-15

This file records evidence, not intent. A row is complete only when the named artifact or runtime check exists.

## Milestones

| Gate | Status | Evidence |
|---|---|---|
| M0: PDF structural rewrite | Complete | `drops_metadata_bytes_from_incremental_history` proves old Info metadata bytes are absent after full `lopdf` serialization. |
| M0: image format decision | Complete | JPEG, PNG and WebP have container-level cleanup and malformed-input tests. TIFF, HEIC and RAW are explicitly unsupported rather than being modified unsafely. |
| M0: Office package integrity | Partial | Real DOCX/XLSX/PPTX/ODT samples were cleaned and successfully opened/exported by LibreOffice 26.2.5. Current Word and WPS executables are unavailable on this machine, so those two applications remain unverified. |
| M1: desktop MVP | Complete | Batch drag/drop, native picker, per-file scan reports, safe-copy/replace modes, forced backup, atomic writes and Chinese/English UI are implemented and tested. |
| M2: Office/PDF/shell integration | Complete | DOCX/XLSX/PPTX/ODT/PDF cleaners, 18-extension Windows Explorer integration and launch-path handling are covered by unit and installer tests. |
| M3: Windows release | Complete | Optimized NSIS and MSI packages build, install, run, remain in the tray after window close, register/unregister context commands and uninstall without files or registry state remaining. MSI installs only `metaclean.exe` plus its uninstall shortcut. |
| M3: macOS/Linux release | Partial | GitHub Actions defines Intel and Apple Silicon DMG plus DEB/RPM/AppImage jobs. There is no Git remote/run evidence in this checkout, and Apple signing/notarization secrets are unavailable locally. |

## Automated quality gates

- Frontend: 23 tests. Statements 90.15%, branches 81.04%, functions 87.20%, lines 95.96%.
- Rust: 27 regular tests plus one opt-in external Office compatibility test. Regions 80.93%, lines 80.99%.
- CI fails below 80% for all frontend coverage dimensions and below 80% Rust line coverage.
- Production frontend build, Rust formatting and Rust tests pass locally.

## Windows release artifacts

- `src-tauri/target/release/bundle/nsis/MetaClean_0.1.0_x64-setup.exe`
- `src-tauri/target/release/bundle/msi/MetaClean_0.1.0_x64_en-US.msi`

## Remaining external release gates

1. Open cleaned DOCX/XLSX/PPTX/ODT samples in current Word and WPS builds. LibreOffice 26.2.5 validation is complete.
2. Run the GitHub release matrix on an actual remote and retain successful macOS/Linux job and artifact evidence.
3. Provide Apple Developer signing/notarization credentials and verify both DMGs with Gatekeeper.
