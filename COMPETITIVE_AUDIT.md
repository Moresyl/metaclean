# MetaClean competitive audit

Baseline audited on 2026-08-17 against ExifCleaner 4.2.0 at
`szTheory/exifcleaner` commit `3e94dcf0014a7853b36db992be4ff53a1827d5b3`
(2026-08-03), and re-audited on 2026-08-23 against the ExifCleaner 4.2.1 source
tree. The audit reads implementation and tests, not only README claims.

A second baseline was added on 2026-08-23 and refreshed on 2026-08-24 against
`guillaumemeyer/watermarks-remover` main commit
`bcf497260af41b7af5ece3ca359e9631320f9c1a`, 142 commits beyond its v0.5.0 tag.
It is the AI-provenance cleaner MetaClean was originally built to replace with a
desktop application and is compared in its own section below.

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
| PDF privacy | ExifTool reversible update; old metadata may remain recoverable | Full `lopdf` reserialization with regression tests proving old metadata bytes are absent and embedded JPEG XObjects have their private blocks removed | Exceeds |
| Office privacy | No native Office revision/comment workflow | DOCX/XLSX/PPTX/ODT properties, comments, custom XML and DOCX revisions, plus EPUB Dublin Core terms, reader sediment from Calibre/Sigil/Kobo/Apple/Adobe, and a `dcterms:modified` stamp pinned to the epoch rather than dropped | Exceeds |
| Audio privacy | Only `.m4a` appears in the actual application whitelist | MP3/WAV/FLAC tag, artwork, XMP and broadcast metadata cleaning, plus WMA and the MPEG-4 audio brands — 10 audio extensions in all | Exceeds |
| Text/AI traces | Not a primary capability | Every Unicode noncharacter, reserved default-ignorable range, private-use character, front matter and HTML/SVG generator or AI attribute, with context preservation for legitimate emoji, scripts, flags, CJK variants and directional text | Exceeds |
| Desktop integration | File/folder picker and drag/drop | Adds Windows Explorer context commands and tray workflow | Exceeds on Windows |
| Result table ergonomics | Sorts name/type/size/before/after, shows size delta, reveal-in-folder and copyable errors | Stable name/extension/source-size/output-size/finding-count sorting, per-file size delta, reveal-in-folder, explicit failure text, persistent history and a per-file detail panel naming the detected format, source/output/backup paths and each finding category as removed, kept or pending | Exceeds on per-file disclosure without exposing values |
| Native application chrome | Full app menus, keyboard accelerators, window-state restore and macOS dock integration | Native menus and accelerators plus a fixed 1180 × 720 enterprise workspace, compact icon rail, self-drawn product caption, command palette, rebuilt context menus and persistent local-status bar | Exceeds |
| Version discovery | No polling; manual Releases link | Optional startup/manual stable-release discovery with official-link validation | Exceeds |
| Actual application intake | README lists 90+ ExifTool writer formats, but `SUPPORTED_EXTENSIONS` in `src/domain/files/file_types.ts` is a 30-entry whitelist and both drop/folder paths enforce it | 91 extensions traverse the real application intake, classification, shell integration and tests, and every one of the baseline's 30 is inside them | Exceeds; strict superset of the competitor's real intake |
| Specialized image/video families | TIFF/TIF, HEIC/HEIF, BMP, AVIF, ten RAW extensions and AVI/M4A/WMV are admitted through ExifTool, with documented partial-removal and rendering risks | TIFF, HEIF/AVIF, BMP, 23 camera raw formats, AVI, Matroska/WebM and ASF/WMV are each cleaned by a native offset-preserving strategy — in-place IFD compaction, item-extent zeroing, RIFF `JUNK` renaming, EBML `Void` stamping and ASF padding-GUID stamping — so no strip, cue or item offset ever shifts | Exceeds; support without the rewrite risk |
| Raw negative writes | `.raf` is refused outright ("RAF metadata removal is disabled because writing this format can damage the original"); every other raw format is forced to a copy and can never replace the original | RAF's embedded JPEG preview — which carries the full EXIF block including GPS and serial number — is cleaned in place and zero-padded to its original extent; raw formats support replacement because a compacted directory cannot move sensor data | Exceeds on the one file family where refusal leaks the most |
| Localization | 26 shipped catalogs, but only `en` (126 keys) and `ro` are complete; 24 of 26 are partial, most at 61/126 keys and `pt` at 13/126 | 32 complete interface locales — a superset of all 26 the baseline ships, plus zh-TW, el, id, ko, ms and nb — every one carrying all 133 source strings, with a catalog that throws at module load on any gap and a test that puts each new locale under coverage automatically | Exceeds on count and, decisively, on completeness |
| Theme selection | System/dark-mode controls | System, light and dark modes with pre-render initialization and persistence | Exceeds on explicit control |
| Installed-app E2E matrix | Unit and installed-app E2E across three OS families | Nine installed-webview scenarios cover startup, keyboard navigation, locale/RTL, accessibility, persisted theme/fidelity, Rust IPC, update-channel reporting and failure paths, with the locale assertion derived from the shipped list rather than hard-coded; local Windows proof and three-OS CI #27 are complete | Parity on OS matrix and broader safety paths |
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
to 91 extensions without routing unknown binary formats through a generic
rewrite, and now strictly contains ExifCleaner's 30-extension application
whitelist even though the latter's README separately enumerates 90+ ExifTool
writer formats. Candidate bytes are re-detected and re-inspected before any
copy, backup or replacement write, and ICC/sRGB preservation is independently
configurable. macOS extended attributes are preserved by default and only known
provenance keys are removed after explicit selection, rather than deleting
unrelated Finder state. Native menus, keyboard accelerators and window-position
restore are present, alongside a self-drawn title bar, a command palette and
rebuilt context menus. The queue matches the baseline's stable sorting,
before/after size delta and native reveal-in-folder workflow while retaining
explicit per-file failure text, and now discloses per-file format, paths and
finding state without ever surfacing a metadata value.

The seven container families this audit previously recorded as deliberate
refusals — TIFF, HEIF/AVIF, BMP, camera RAW, AVI, Matroska/WebM and ASF/WMV —
are now supported, and the evidence bar that gated them was met rather than
lowered. Each has a native cleaner that removes metadata without relocating a
single byte, so the offsets those containers depend on are as true after the
clean as before it; the per-family strategies and their tests are recorded in
`SUPPORT_POLICY.md`. What remains refused there is short and principled: legacy
binary Office, statistical text watermarks, pixel-domain watermarks and unknown
binaries. The no-value UI boundary remains formal product policy.

## watermarks-remover baseline

`guillaumemeyer/watermarks-remover` main at
`bcf497260af41b7af5ece3ca359e9631320f9c1a` is an agent skill plus a local
Python HTTP service. It is the project MetaClean set out to replace with
something a non-developer can run, and the comparison is therefore about
delivery as much as capability.

| Capability | watermarks-remover baseline | MetaClean current | Status |
|---|---|---|---|
| Delivery | Python service the user starts, driven by an agent skill over HTTP on `127.0.0.1:8765`; no desktop application | Signed installers for Windows, macOS and Linux with no Python, Perl or runtime to set up, and no local port to open | Exceeds; this is the reason MetaClean exists |
| Layer A — invisible Unicode | Invisible Unicode, exotic spaces, bidi controls, tag characters, noncharacters and reserved ranges, via deterministic scripts | The same classes plus all private-use planes in `cleaners/text.rs`; all 66 noncharacters and reserved default-ignorable ranges are covered | Exceeds; adds private-use planes |
| Joiner correctness | Context rules preserve selected emoji and complex-script use while stripping floating controls | Context preservation also covers expanded emoji symbols, complete tag flags, CJK variants, Egyptian/Duployan/music layout controls and paired directional embeddings/isolates | Exceeds on the verified preservation set |
| Layer B — statistical watermarks | Agent rewrite plus an optional `rewrite_text.py` hook | Deliberately out of scope, documented as such in both READMEs and `SUPPORT_POLICY.md` | Divergence by design; see below |
| File formats | PNG, JPEG, WebP, AVIF, HEIC, BMP, GIF, TIFF, SVG, PDF, DOCX, XLSX, PPTX, EPUB, ODT, HTML, Markdown, MP4/MOV/M4A/M4V, WAV, MP3 | All of the above are inside the 91-extension allowlist, alongside 23 camera raw formats, AVI, ASF/WMV, Matroska/WebM and FLAC | Exceeds; strict superset |
| C2PA / provenance | C2PA, EXIF, XMP and document properties, including WAV C2PA and C2PA carried by an ID3v2 prefix before FLAC | The same, plus JUMBF/C2PA in JPEG, PNG, WebP and HEIF item payloads, WAV C2PA, ID3-prefixed FLAC, DOCX revision resolution, deep PDF JPEG cleanup and embedded raster data-URI cleanup | Exceeds |
| Auditability | HTTP response and agent transcript | A versioned JSON report exported atomically from the desktop queue, with aggregate and per-file outcomes but no raw sensitive metadata values | Exceeds on enterprise-safe local evidence |
| Privacy of the operation itself | Content is posted to a local HTTP service; Layer B routes the text through an agent | Files are never read by anything but the local process, and there is no network path for content in any mode | Exceeds |

Layer B stays out of scope because it cannot be implemented without breaking the
guarantee that makes MetaClean worth installing. A statistical watermark lives in
the author's word choices, so removing it means having a language model rewrite
the prose — which means either shipping a model or sending the text to one. Both
contradict "your files never leave your computer." Byte-level cleaning is the
whole product boundary, and this is the edge of it, not an unfinished feature.

## Remaining parity backlog

No known in-scope product or release parity backlog remains within either baseline after
the 2026-08-24 watermarks-remover main refresh. The
one delta against ExifCleaner is a distribution channel rather than a
capability: it publishes an AUR package (`paru -S exifcleaner-bin`) and
MetaClean does not. That is a publishing decision, out of scope for the source
tree.

The raw-value metadata viewer and the Layer B statistical rewrite remain
deliberate privacy divergences, not missing deliverables. Word/WPS
interoperability and Apple signing/notarization are external qualification gates
tracked in `VALIDATION.md`.

Future work must preserve MetaClean's fail-closed and irreversible-cleaning
policy instead of accepting formats whose private metadata cannot be removed
safely.
