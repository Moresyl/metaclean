# MetaClean competitive audit

Baseline audited on 2026-08-17 against `szTheory/exifcleaner` commit
`3e94dcf0014a7853b36db992be4ff53a1827d5b3` (2026-08-03).

This is an evidence ledger, not a claim that every competitor feature is
already matched. A row is complete only when the repository contains the named
implementation and tests.

| Capability | ExifCleaner baseline | MetaClean current | Status |
|---|---|---|---|
| Safe copy and replacement | Save-as-copy option | Safe copy by default; replacement always creates a backup and uses atomic output | Exceeds |
| Folder recursion | Recursive file/folder intake | Recursive picker and drag/drop with skip reasons, symlink refusal, 64-level and 10,000-file limits | Exceeds on safety |
| Metadata inspection | Before/after metadata diff | Read-only per-file findings before confirmation; no private values rendered | Different privacy-first design |
| JPEG orientation | Optional preservation | Minimal orientation-only EXIF reconstruction, independently switchable | Parity with smaller retained surface |
| File timestamps | Optional preservation | Access/modified time and permissions preserved by default | Parity |
| PDF privacy | ExifTool reversible update; old metadata may remain recoverable | Full `lopdf` reserialization with a regression test proving old metadata bytes are absent | Exceeds |
| Office privacy | No native Office revision/comment workflow | DOCX/XLSX/PPTX/ODT properties, comments, custom XML and DOCX revisions | Exceeds |
| Text/AI traces | Not a primary capability | Invisible Unicode, private-use characters, front matter, HTML/SVG generator and AI attributes | Exceeds |
| Desktop integration | File/folder picker and drag/drop | Adds Windows Explorer context commands and tray workflow | Exceeds on Windows |
| Version discovery | No polling; manual Releases link | Optional startup/manual stable-release discovery with official-link validation | Exceeds |
| Format breadth | 90+ ExifTool writer formats including video/RAW | 26 explicitly supported extensions; images, audio, MP4/QuickTime video, Office, PDF and text | Gap remains |
| Localization | 25 selectable locales, with many non-English catalogs reporting partial coverage | 5 complete interface locales; system detection, persisted selection and tested static/dynamic coverage | Gap remains on locale count |
| Theme selection | System/dark-mode controls | System, light and dark modes with pre-render initialization and persistence | Exceeds on explicit control |
| Installed-app E2E matrix | Unit and installed-app E2E across three OS families | Unit/coverage gates and local Windows installer evidence | Gap remains |

## Release boundary

The current MetaClean branch is stronger on irreversible PDF cleaning, Office
revisions, text/AI traces, atomic backup semantics, Windows shell integration,
secure version discovery and explicit theme control. ExifCleaner still leads on
broad ExifTool-backed format coverage, localization count and installed-app E2E
breadth. Future work should close those rows without weakening MetaClean's
fail-closed format policy.
