# MetaClean competitive audit

Baseline audited on 2026-08-17 against ExifCleaner 4.2.0 at
`szTheory/exifcleaner` commit `3e94dcf0014a7853b36db992be4ff53a1827d5b3`
(2026-08-03). The audit reads implementation and tests, not only README claims.

This is an evidence ledger, not a claim that every competitor feature is
already matched. A row is complete only when the repository contains the named
implementation and tests.

| Capability | ExifCleaner baseline | MetaClean current | Status |
|---|---|---|---|
| Safe copy and replacement | Save-as-copy option | Safe copy by default; replacement always creates a backup and uses atomic output | Exceeds |
| Folder recursion | Recursive file/folder intake | Recursive picker and drag/drop with skip reasons, symlink refusal, 64-level and 10,000-file limits | Exceeds on safety |
| Unsupported intake accounting | Folder summaries count skipped files and unreadable roots | Counts every skipped item, returns the first concrete issue and reports when a safety limit is reached | Exceeds on bounded failure reporting |
| Metadata inspection | Expandable before/after tag values, grouped diff and copy-all | Read-only category/count findings before confirmation; raw GPS, identity and provenance values never enter UI state, history, screenshots, clipboard or accessibility APIs | Deliberate privacy-minimizing divergence; less suitable for forensic inspection |
| Runtime cleanup verification | Reads metadata after processing and distinguishes cleaned, unchanged, refused and verification-failed outcomes | Re-detects the format and re-inspects the exact in-memory candidate before choosing an output path, creating a backup or writing; residual traces fail closed | Exceeds on pre-commit timing |
| JPEG orientation | Optional preservation | Minimal orientation-only EXIF reconstruction, independently switchable | Parity with smaller retained surface |
| ICC color profile | Optional preserve/remove setting | Independently persisted preserve/remove control for JPEG, PNG and WebP, preserving profiles by default | Parity with candidate-byte verification |
| File timestamps | Optional preservation | Access/modified time and permissions preserved by default | Parity |
| macOS extended attributes | Optional blanket `xattr` removal | Read-only scan plus explicit opt-in removal of six known provenance/download keys; all xattrs are preserved by default and unrelated Finder/resource-fork/tag/custom keys survive | Exceeds on selective preservation; real macOS filesystem regression gated in CI |
| PDF privacy | ExifTool reversible update; old metadata may remain recoverable | Full `lopdf` reserialization with a regression test proving old metadata bytes are absent | Exceeds |
| Office privacy | No native Office revision/comment workflow | DOCX/XLSX/PPTX/ODT properties, comments, custom XML and DOCX revisions | Exceeds |
| Audio privacy | No audio extension in the actual application whitelist | MP3/WAV/FLAC tag, artwork, XMP and broadcast metadata cleaning | Exceeds |
| Text/AI traces | Not a primary capability | Invisible Unicode, private-use characters, front matter, HTML/SVG generator and AI attributes | Exceeds |
| Desktop integration | File/folder picker and drag/drop | Adds Windows Explorer context commands and tray workflow | Exceeds on Windows |
| Result table ergonomics | Sorts name/type/size/before/after, shows size delta, reveal-in-folder and copyable errors | Stable name/extension/source-size/output-size/finding-count sorting, per-file size delta, reveal-in-folder, explicit failure text and persistent history | Parity; detailed metadata copy remains tracked separately |
| Native application chrome | Full app menus, keyboard accelerators, window-state restore and macOS dock integration | Native app/navigation/window menus, cross-platform accelerators, close-to-tray workflow and size/position/maximized-state restore | Parity; adds tray workflow |
| Version discovery | No polling; manual Releases link | Optional startup/manual stable-release discovery with official-link validation | Exceeds |
| Actual application intake | README lists 90+ ExifTool writer formats, but both drop/folder paths enforce a 30-extension source whitelist; RAF is then refused and MKV has no writable tags | 47 extensions traverse the real application intake, classification, shell integration and tests | Exceeds on actual explicit intake count |
| Specialized image/video families | TIFF/TIF, HEIC/HEIF, BMP, AVIF, ten RAW extensions and AVI/MKV/WMV are admitted, with documented partial-removal and rendering risks | Each family is explicitly refused with a container-specific rationale and an evidence requirement for future support | Deliberate safety divergence; narrower but fail-closed |
| Localization | 25 selectable locales, with many non-English catalogs reporting partial coverage | 26 complete interface locales; system detection, persisted selection, tested static/dynamic coverage and Arabic RTL | Exceeds on count and completeness |
| Theme selection | System/dark-mode controls | System, light and dark modes with pre-render initialization and persistence | Exceeds on explicit control |
| Installed-app E2E matrix | Unit and installed-app E2E across three OS families | Eight installed-webview scenarios cover startup, keyboard navigation, locale/RTL, accessibility, persisted theme/fidelity, Rust IPC and failure paths; local Windows proof and three-OS CI #27 are complete | Parity on OS matrix and broader safety paths |
| Accessibility verification | Dedicated keyboard/accessibility Playwright scenarios | Dedicated installed-app checks enforce language, landmarks, named navigation and zero unnamed buttons/form controls; keyboard navigation is separately exercised | Parity |
| Runtime footprint | Bundles Electron, Perl and ExifTool platform payloads | Native Rust/Tauri cleaners with no ExifTool, Perl or Python runtime | Exceeds |
| Release artifacts | macOS DMG, Linux AppImage/DEB/RPM, Windows x64/ia32 NSIS plus portable build | macOS Intel/Arm DMG, Linux AppImage/DEB/RPM, Windows x64 NSIS/MSI, x86 NSIS and architecture-labelled x64/x86 portable ZIPs | Exceeds with MSI and dual-architecture portable packages |
| Release integrity | Generates `SHASUMS256.txt` and smoke-tests packaged payloads | v0.3.0 workflow #7 installed, extracted or copied and launch-smoked all five platform builds before publishing ten platform packages plus a complete ten-entry `SHASUMS256.txt` | Parity proven; adds MSI and dual-architecture portable validation |
| Webview boundary | Hardened Electron navigation/IPC policy and disabled Node attack surfaces | Narrow Tauri command/capability surface, official-link allowlist and explicit local-only production CSP with a regression gate | Parity with smaller IPC surface |
| Dependency security | Pinned ExifTool checksums and release gates | No known npm vulnerability, repository-owned hardened archive extractor, cargo audit gate, test-only driver isolation and published per-package SHA-256 manifest | Exceeds on npm transitive mitigation and runtime isolation |

## Release boundary

The current MetaClean branch is stronger on irreversible PDF cleaning, Office
revisions, text/AI traces, atomic backup semantics, Windows shell integration,
secure version discovery, localization completeness and explicit theme control.
It now matches the three-family installed-app CI matrix while keeping the test
driver absent from production builds. Its explicit real-app intake grew from 26
to 47 extensions without routing unknown binary formats through a generic
rewrite, exceeding ExifCleaner's 30-extension application whitelist even though
the latter's README separately enumerates 90+ ExifTool writer formats. Candidate
bytes are now re-detected and re-inspected before any copy, backup or replacement
write, and ICC/sRGB preservation is independently configurable. macOS extended
attributes are preserved by default and only known provenance keys are removed
after explicit selection, rather than deleting unrelated Finder state. Native
menus, keyboard accelerators and window geometry restore are now present. The queue now
matches the baseline's stable sorting, before/after size delta and native
reveal-in-folder workflow while retaining explicit per-file failure text.
The no-value UI boundary and the individual refusal decisions for TIFF,
HEIF/AVIF, BMP, camera RAW, AVI, Matroska/WebM and ASF/WMV are formal product
policy in `SUPPORT_POLICY.md`, not implied future support.

## Remaining parity backlog

No known product or release parity backlog remains within the audited ExifCleaner
4.2.0 baseline. The raw-value metadata viewer and unsafe generic rewriting of
specialized containers remain deliberate privacy/safety divergences, not missing
deliverables. Word/WPS interoperability and Apple signing/notarization are
external qualification gates tracked in `VALIDATION.md`.

Future work must preserve MetaClean's fail-closed and irreversible-cleaning
policy instead of accepting formats whose private metadata cannot be removed
safely.
