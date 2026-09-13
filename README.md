<div align="center">

<img src="assets/metaclean-icon.svg" alt="MetaClean" width="88" height="88">

# MetaClean

**Strip private metadata from your files before you share them.**

Local file processing · Signed in-app updates · No ExifTool, Python or Perl · Rust core

[![CI](https://img.shields.io/github/actions/workflow/status/Moresyl/metaclean/ci.yml?branch=master&style=flat-square&label=CI)](https://github.com/Moresyl/metaclean/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Moresyl/metaclean?include_prereleases&style=flat-square&color=35966d)](https://github.com/Moresyl/metaclean/releases)
[![License](https://img.shields.io/badge/license-MIT-35966d?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-35966d?style=flat-square)](https://github.com/Moresyl/metaclean/releases/latest)
[![Core](https://img.shields.io/badge/core-Rust-35966d?style=flat-square)](src-tauri)

**English** · [简体中文](README.zh-CN.md)

<img src="assets/metaclean-screenshot.png" alt="MetaClean desktop interface" width="820">

</div>

---

Photos carry GPS coordinates. Word documents carry your name, your employer, and every tracked change you thought you had removed. PDFs keep the metadata of earlier drafts. Text pasted out of an LLM carries invisible Unicode.

MetaClean finds all of it and removes it — entirely on your own machine.

## Why

- **Your files never leave your computer.** No upload API, telemetry, or cloud processing. The optional signed update check requests only MetaClean's official GitHub-hosted feeds and can be disabled.
- **Nothing to install first.** One binary. No Python, Perl, ExifTool, or runtime to set up.
- **Scan first, then decide.** Scanning is read-only. You see a per-file report of what was found and confirm before anything is written.
- **Safe by default.** Originals are backed up before replacement, writes are atomic, and the default mode produces a `.cleaned` copy instead of overwriting.
- **Honest about limits.** Formats that cannot be cleaned safely are rejected outright, never silently passed through.

## Download

Grab the latest package from [GitHub Releases](https://github.com/Moresyl/metaclean/releases/latest). Windows releases include x64 NSIS/MSI installers, x86 NSIS, and x64/x86 portable ZIPs; every executable package is launch-smoked before publication.

| Platform | Packages |
| --- | --- |
| Windows | x64 `.exe` (NSIS) / `.msi` / portable ZIP · x86 `.exe` (NSIS) / portable ZIP |
| macOS | `.dmg` — Apple Silicon and Intel |
| Linux | `.deb` · `.rpm` · `.AppImage` |

## What it removes

113 extensions, handled by native Rust code with no ExifTool. Image and media cleaners preserve encoded payloads; PDF, Office and UTF-8/BOM-marked UTF-16 text use format-aware rewrites whose candidates are re-inspected before writing.

| Format | Extensions | Cleaned |
| --- | --- | --- |
| JPEG | `.jpg` `.jpeg` `.jpe` | EXIF/GPS, XMP, IPTC, comments, JUMBF/C2PA segments; ICC profiles are preserved by default or removable on request |
| PNG | `.png` | EXIF, textual metadata, C2PA/JUMBF chunks; optional ICC profile removal |
| WebP | `.webp` | EXIF, XMP, C2PA chunks; optional ICC profile removal |
| JPEG XL | `.jxl` | Container EXIF, XMP, JUMBF/C2PA, Brotli-wrapped metadata and JPEG reconstruction data are retired in place; naked codestreams are accepted unchanged |
| GIF | `.gif` | Comments and XMP application metadata without re-encoding frames |
| BMP | `.bmp` `.dib` | The reserved header words editors write IDs into, V5 embedded ICC profiles, and EXIF or XMP stapled past the last pixel where no viewer shows it |
| TIFF | `.tif` `.tiff` | EXIF, GPS, IPTC and XMP directories, removed by compacting the image file directory in place so every strip, tile and preview offset stays valid |
| Camera RAW | `.cr2` `.cr3` `.crw` `.nef` `.nrw` `.arw` `.srf` `.sr2` `.orf` `.rw2` `.rwl` `.dng` `.pef` `.srw` `.raf` `.3fr` `.erf` `.mef` `.mos` `.iiq` `.kdc` `.dcr` `.k25` | The same in-place directory compaction, plus MakerNote and GPS directories. Fujifilm's embedded JPEG preview and Canon's CR3 item payloads are cleaned where they lie; sensor data is never rewritten |
| HEIF & AVIF | `.heic` `.heif` `.heics` `.heifs` `.hif` `.avif` `.avifs` | EXIF, XMP and C2PA items zeroed at item granularity, leaving the item table that locates the picture intact |
| Audio | `.mp3` `.wav` `.flac` | ID3/APEv2, RIFF INFO/XMP/BWF/iXML/C2PA, FLAC Vorbis comments, pictures, XMP and prefixed ID3/C2PA |
| AIFF | `.aif` `.aiff` `.aifc` | Native name, author, copyright, annotation and comment chunks plus ID3, XMP and C2PA, without re-encoding samples |
| ISO media | `.mp4` `.mov` `.m4v` `.m4a` `.3g2` `.3gp` `.3gp2` `.3gpp` `.f4a` `.f4b` `.f4p` `.f4v` `.lrv` `.m4b` `.m4p` `.mqv` `.qt` | ISO BMFF/QuickTime user data, XMP, author and location atoms without moving media bytes |
| AVI | `.avi` | Metadata chunks renamed to RIFF's own `JUNK` padding tag and blanked, so the `idx1` index keeps its meaning under either offset convention |
| Matroska & WebM | `.mkv` `.mka` `.mks` `.mk3d` `.webm` | Tags, attachments and writing-application strings retired by stamping EBML `Void` over them in the same bytes, leaving the cue index true |
| ASF | `.asf` `.wmv` `.wma` | Content descriptions and the `WM/` attribute space overwritten with the format's own padding object; the header's object count stays honest |
| Documents | `.docx` `.xlsx` `.pptx` `.odt` `.ods` `.odp` `.odg` `.odf` `.odb` `.odm` `.ott` `.ots` `.otp` `.otg` `.epub` | Author and application properties, comments, custom XML. DOCX and OpenDocument revisions are resolved — insertions accepted, deletions removed. EPUB loses its Dublin Core people and dates plus Calibre/Sigil/Kobo/Apple/Adobe leftovers |
| PDF | `.pdf` | Info dictionary, XMP and metadata inside embedded JPEG images, then a full reserialization that discards metadata stranded in incremental-update history |
| Text & markup | `.txt` `.md` `.markdown` `.html` `.htm` `.xhtml` `.svg` `.xml` `.json` `.csv` `.tsv` `.yaml` `.yml` `.log` `.srt` `.vtt` `.css` `.scss` `.less` `.ini` `.conf` `.cfg` `.toml` `.properties` | Invisible Unicode, generator/author metadata in Markdown front matter, HTML/XHTML and SVG, plus metadata inside embedded image data URIs |

For TIFF/RAW, HEIF/AVIF, JPEG XL, AVI, Matroska/WebM and ASF/WMV containers that
depend on absolute positions, MetaClean does not move the media payload:
metadata is compacted in place, zeroed or replaced by a format-defined padding
element. JPEG/PNG/WebP/GIF/WAV/FLAC/AIFF/ISO media use structural reconstruction
that preserves their content payload, while PDF, Office and text follow their
document models. Every candidate is re-detected and re-inspected before writing.

**Deliberately out of scope:** statistical text watermarks, pixel-domain watermarks, legacy binary Office files (`.doc` / `.xls` / `.ppt`), and unknown binary formats. MetaClean refuses these rather than modifying them unsafely.

## Safety guarantees

- Always writes a `.bak` file before replacing an original
- Produces a `.cleaned` safe copy by default
- Writes to a temporary file, then atomically replaces the destination
- Re-inspects the exact cleaned candidate bytes before creating a backup or writing any output
- Re-checks source bytes, modification time, readonly permissions and extended attributes before allocating output or backup, and again before replacement
- Refuses symlinks and reparse points as input or destination, including paths reached through linked parent directories
- Bounds each IPC path to 32 KiB and each raw path batch to 64 MiB before native work starts
- De-duplicates platform path aliases while reading, keeps at most 10,000 unique files and caps raw input at 40,000 items so repeated drops do not consume the unique-file budget
- Rejects a drag/drop batch whose aggregate path payload exceeds 64 MiB instead of silently processing only a prefix
- Rejects browser file selections over 10,000 items instead of silently truncating the tail
- Caps bulk path-copy output at 16 MiB UTF-8 and asks users to copy oversized lists in smaller batches
- Caps parser, filesystem, updater and intake diagnostics at 8 KiB with UTF-8-safe truncation before they reach IPC/UI surfaces
- Caps input at 256 MiB, and expanded Office archives at 512 MiB
- Malformed or unsupported files fail without touching the source

## Processing model

MetaClean uses a deliberately narrow pipeline: intake rejects symlinks, reparse
points, oversized trees and unsupported folder entries; scanning reads a bounded
snapshot without writing; cleaning produces an in-memory candidate; the native
engine detects and inspects that exact candidate again; only then does it create
a unique copy or a backup followed by an atomic replacement. The UI receives
categories and counts, not raw GPS, author, device or comment values. This keeps
sensitive values out of the React state tree, history, clipboard and audit
exports while still giving every file an explicit outcome.

The application is intentionally fail-closed. Unknown binary formats, legacy
binary Office files and pixel/statistical watermark removal are refused instead
of being routed through a generic rewriter. See [SUPPORT_POLICY.md](SUPPORT_POLICY.md)
for the format-by-format safety bar and [COMPETITIVE_AUDIT.md](COMPETITIVE_AUDIT.md)
for the evidence-backed comparison with other local cleaners.

## The desktop app

- Drag in files or folders, or recursively import a folder from the native picker
- Five panes: **Clean**, **History**, **Privacy**, **Settings**, and **About**
- Optional Windows File Explorer command across all 113 supported extensions — on Windows 11 it lives under **Show more options**
- Closing the window exits MetaClean by default; Settings can instead keep it in the system tray, where the tray menu can reopen or exit it
- About provides version/runtime facts, copyable diagnostics, JSON export, issue/feature/release links and the source/license entry points
- Queue rows expose direct path copying as well as the full context menu, and update/command dialogs keep keyboard focus contained until dismissed
- Large scans and cleanup batches show count-only progress in the local status bar; no file path or content is sent through the progress event
- Large scans and cleanup batches can be cancelled safely between files; scans return only completed reports, cleanup results remain available, and unprocessed files stay retryable
- An abnormal exit leaves only path-free batch counts; the next launch clearly asks for a re-import and scan instead of guessing how to resume writes
- Stable queue sorting by name, extension, source/output size or finding count, with per-file size savings and reveal-in-folder actions for completed outputs
- Versioned local JSON audit-report export with per-file findings and outcomes but no raw metadata values
- Fixed 1180 × 720 enterprise workspace with compact icon navigation and a persistent local-only status bar
- Preserves JPEG display orientation, ICC/sRGB color profiles and file timestamps by default, with independent removal controls
- Preserves every macOS extended attribute by default; an explicit opt-in removes only six known download/provenance attributes and leaves Finder data, resource forks, tags, and custom attributes intact
- Native application menus, `Ctrl/Cmd+1…5` navigation accelerators, command palette, and persisted window size, position, and maximized state
- Checks, downloads and installs cryptographically signed stable updates in installed builds, failing over from the GitHub Release manifest to the official Pages feed; portable Windows packages and non-AppImage Linux builds fall back to the official Releases page
- Automatic update checks are independently switchable off, restoring fully offline operation
- 32 complete interface languages spanning Europe, East and Southeast Asia, South Asia, and right-to-left Arabic and Persian; system/light/dark theme, output mode, fidelity options, and local cleanup history persist between sessions

## Build from source

Requires [Rust](https://rustup.rs), [Node.js](https://nodejs.org) and [pnpm](https://pnpm.io), plus the [Tauri system dependencies](https://tauri.app/start/prerequisites/) for your platform.

```bash
pnpm install
pnpm tauri dev      # run the app in development
```

Run the full check suite before opening a pull request:

```bash
pnpm test:coverage                              # frontend tests, 80% floor
pnpm test:formats                               # extension-manifest consistency
pnpm test:security                              # production WebView CSP policy
pnpm test:release                               # release-note and checksum automation
pnpm test:supply-chain                          # patched dependency regression
pnpm test:docs                                   # product/documentation claim consistency
pnpm docs:build                                  # searchable VitePress documentation site
pnpm test:benchmark                             # optional release-mode native batch benchmark
pnpm build                                      # typecheck + production bundle
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml # Rust core tests
pnpm test:e2e:build && pnpm test:e2e             # real desktop app E2E
pnpm tauri build                                # platform installers
```

On Windows, the non-publishing candidate preflight builds the debug NSIS/MSI
packages, creates the portable ZIP and runs all three install/launch/uninstall
smoke checks in one pass:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\preflight-windows.ps1
```

Every branch build also launches an E2E-only desktop binary on Windows, macOS and Linux. Its embedded WebDriver and test commands are gated behind a Cargo feature and are absent from production bundles. Pushing a version tag builds the whole release matrix through GitHub Actions: x64 NSIS/MSI plus x86 NSIS and x64/x86 portable ZIPs for Windows, DMG for Apple Silicon and Intel macOS, and DEB/RPM/AppImage for Linux. Release builds additionally produce signed updater bundles for five platform targets and a static `latest.json`; publication waits for every package smoke test and a complete SHA-256 manifest. Updater signatures are independent of operating-system code signing. macOS signing and notarization still require Apple credentials; without them the macOS job produces unsigned DMGs.

Test coverage and release evidence are tracked in [VALIDATION.md](VALIDATION.md).
Release changes are recorded in [CHANGELOG.md](CHANGELOG.md).
The deliberate no-value metadata policy, the per-format cleaning strategies and
what stays out of scope are documented in [SUPPORT_POLICY.md](SUPPORT_POLICY.md).
The implementation-facing visual and interaction rules are recorded in [DESIGN.md](DESIGN.md).
The current engineering status, roadmap and evidence boundaries are recorded in [docs/PLAN.md](docs/PLAN.md), with a concise document index in [docs/README.md](docs/README.md).

## Contributing

Pull requests are welcome — start with [CONTRIBUTING.md](CONTRIBUTING.md). For anything security-related, follow [SECURITY.md](SECURITY.md) and use private vulnerability reporting rather than a public issue.

## Responsible use

Process only content you own or are authorized to handle. MetaClean exists for privacy and file hygiene — not for academic fraud, false provenance, or misleading claims about where a file came from.

## License

[MIT](LICENSE) © Moresyl
