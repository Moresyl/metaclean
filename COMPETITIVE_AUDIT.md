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
| Metadata inspection | Expandable before/after tag values, grouped diff and copy-all | Read-only category/count findings before confirmation; no private values rendered | Gap: detailed local diff is absent |
| Runtime cleanup verification | Reads metadata after processing and distinguishes cleaned, unchanged, refused and verification-failed outcomes | Re-detects the format and re-inspects the exact in-memory candidate before choosing an output path, creating a backup or writing; residual traces fail closed | Exceeds on pre-commit timing |
| JPEG orientation | Optional preservation | Minimal orientation-only EXIF reconstruction, independently switchable | Parity with smaller retained surface |
| ICC color profile | Optional preserve/remove setting | Independently persisted preserve/remove control for JPEG, PNG and WebP, preserving profiles by default | Parity with candidate-byte verification |
| File timestamps | Optional preservation | Access/modified time and permissions preserved by default | Parity |
| macOS extended attributes | Optional `xattr` removal | No extended-attribute scan/removal | Gap |
| PDF privacy | ExifTool reversible update; old metadata may remain recoverable | Full `lopdf` reserialization with a regression test proving old metadata bytes are absent | Exceeds |
| Office privacy | No native Office revision/comment workflow | DOCX/XLSX/PPTX/ODT properties, comments, custom XML and DOCX revisions | Exceeds |
| Audio privacy | No audio extension in the actual application whitelist | MP3/WAV/FLAC tag, artwork, XMP and broadcast metadata cleaning | Exceeds |
| Text/AI traces | Not a primary capability | Invisible Unicode, private-use characters, front matter, HTML/SVG generator and AI attributes | Exceeds |
| Desktop integration | File/folder picker and drag/drop | Adds Windows Explorer context commands and tray workflow | Exceeds on Windows |
| Result table ergonomics | Sorts name/type/size/before/after, shows size delta, reveal-in-folder and copyable errors | Stable name/extension/source-size/output-size/finding-count sorting, per-file size delta, reveal-in-folder, explicit failure text and persistent history | Parity; detailed metadata copy remains tracked separately |
| Native application chrome | Full app menus, keyboard accelerators, window-state restore and macOS dock integration | Tray open/quit and close-to-tray; no full menu/shortcut or geometry restore | Gap |
| Version discovery | No polling; manual Releases link | Optional startup/manual stable-release discovery with official-link validation | Exceeds |
| Actual application intake | README lists 90+ ExifTool writer formats, but both drop/folder paths enforce a 30-extension source whitelist; RAF is then refused and MKV has no writable tags | 47 extensions traverse the real application intake, classification, shell integration and tests | Exceeds on actual explicit intake count |
| Specialized image/video families | TIFF/TIF, HEIC/HEIF, BMP, AVIF, ten RAW extensions and AVI/MKV/WMV are admitted, with documented partial-removal and rendering risks | Refuses these families rather than claiming unsafe cleanup | Gap in family breadth; exceeds on truthful fail-closed behavior |
| Localization | 25 selectable locales, with many non-English catalogs reporting partial coverage | 26 complete interface locales; system detection, persisted selection, tested static/dynamic coverage and Arabic RTL | Exceeds on count and completeness |
| Theme selection | System/dark-mode controls | System, light and dark modes with pre-render initialization and persistence | Exceeds on explicit control |
| Installed-app E2E matrix | Unit and installed-app E2E across three OS families | E2E-featured desktop binaries launch and pass startup/navigation, locale/RTL, persisted theme and Rust IPC scenarios on Windows, Ubuntu and macOS in CI #21 | Parity on OS matrix; narrower scenario count |
| Accessibility verification | Dedicated keyboard/accessibility Playwright scenarios | Semantic component tests and RTL desktop coverage, but no dedicated installed-app accessibility suite | Gap |
| Runtime footprint | Bundles Electron, Perl and ExifTool platform payloads | Native Rust/Tauri cleaners with no ExifTool, Perl or Python runtime | Exceeds |
| Release artifacts | macOS DMG, Linux AppImage/DEB/RPM, Windows x64/ia32 NSIS plus portable build | macOS Intel/Arm DMG, Linux AppImage/DEB/RPM, Windows x64 NSIS and MSI | Different; portable and Windows 32-bit are gaps, MSI is an addition |
| Release integrity | Generates `SHASUMS256.txt` and smoke-tests packaged payloads | Workflow installs or copies every platform package and launch-smokes it before public release creation; Windows NSIS is proven locally, while macOS/Linux execution and final manifest publication await the next tag run | Implemented; cross-platform release proof pending |
| Webview boundary | Hardened Electron navigation/IPC policy and disabled Node attack surfaces | Narrow Tauri command/capability surface, official-link allowlist and explicit local-only production CSP with a regression gate | Parity with smaller IPC surface |
| Dependency security | Pinned ExifTool checksums and release gates | No known npm vulnerability, repository-owned hardened archive extractor, cargo audit gate and test-only driver isolation | Exceeds on npm transitive mitigation; release checksums still pending |

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
write, and ICC/sRGB preservation is independently configurable. The queue now
matches the baseline's stable sorting, before/after size delta and native
reveal-in-folder workflow while retaining explicit per-file failure text.

## Remaining parity backlog

This is the complete known product/release gap list derived from the baseline's
settings schema, intake whitelist, renderer table, application menus, platform
services, release workflow and E2E suite:

1. Add opt-in macOS extended-attribute inspection/removal without deleting unrelated data silently.
2. Provide expandable local before/after metadata values and a copyable diff.
3. Add full desktop menus/keyboard accelerators and persistent window geometry.
4. Safely implement or continue refusing TIFF, HEIC/HEIF, AVIF, BMP, RAW and AVI/MKV/WMV individually; no count-only aliasing is acceptable.
5. Add dedicated installed-app accessibility and broader failure-path E2E scenarios.
6. Execute the next tagged release to prove macOS/Linux packaged-app smoke and final checksum publication; Windows NSIS is already proven locally.
7. Decide and document Windows portable/32-bit support rather than implying artifact parity.

Future work must preserve MetaClean's fail-closed and irreversible-cleaning
policy instead of accepting formats whose private metadata cannot be removed
safely.
