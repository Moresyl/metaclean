<div align="center">

<img src="assets/metaclean-icon.svg" alt="MetaClean" width="88" height="88">

# MetaClean

**Strip private metadata from your files before you share them.**

Local file processing · Optional update check · No ExifTool, Python or Perl · Rust core

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

- **Your files never leave your computer.** No upload API, telemetry, or cloud processing. The optional update check requests only the official GitHub Releases endpoint and can be disabled.
- **Nothing to install first.** One binary. No Python, Perl, ExifTool, or runtime to set up.
- **Scan first, then decide.** Scanning is read-only. You see a per-file report of what was found and confirm before anything is written.
- **Safe by default.** Originals are backed up before replacement, writes are atomic, and the default mode produces a `.cleaned` copy instead of overwriting.
- **Honest about limits.** Formats that cannot be cleaned safely are rejected outright, never silently passed through.

## Download

Grab the latest package from [GitHub Releases](https://github.com/Moresyl/metaclean/releases/latest).

| Platform | Packages |
| --- | --- |
| Windows | `.exe` (NSIS) · `.msi` |
| macOS | `.dmg` — Apple Silicon and Intel |
| Linux | `.deb` · `.rpm` · `.AppImage` |

## What it removes

| Format | Extensions | Cleaned |
| --- | --- | --- |
| JPEG | `.jpg` `.jpeg` | EXIF/GPS, XMP, IPTC, comments, JUMBF/C2PA segments |
| PNG | `.png` | EXIF, textual metadata, C2PA/JUMBF chunks |
| WebP | `.webp` | EXIF, XMP, C2PA chunks |
| GIF | `.gif` | Comments and XMP application metadata without re-encoding frames |
| Audio | `.mp3` `.wav` `.flac` | ID3/APEv2, RIFF INFO/XMP/BWF/iXML, FLAC Vorbis comments, pictures and XMP |
| Video | `.mp4` `.mov` `.m4v` `.m4a` | ISO BMFF/QuickTime user data, XMP, author and location atoms without moving media bytes |
| Office | `.docx` `.xlsx` `.pptx` `.odt` | Author and application properties, comments, custom XML. DOCX revisions are resolved — insertions accepted, deletions removed |
| PDF | `.pdf` | Info dictionary and XMP, then a full reserialization that discards metadata stranded in incremental-update history |
| Text & markup | `.txt` `.md` `.markdown` `.html` `.htm` `.svg` `.xml` `.json` `.csv` | Invisible Unicode, plus generator/author metadata in Markdown front matter, HTML and SVG |

**Deliberately out of scope:** statistical text watermarks, pixel-domain watermarks, unsupported video containers, legacy binary Office files (`.doc` / `.xls` / `.ppt`), and unknown binary formats. MetaClean refuses these rather than modifying them unsafely.

## Safety guarantees

- Always writes a `.bak` file before replacing an original
- Produces a `.cleaned` safe copy by default
- Writes to a temporary file, then atomically replaces the destination
- Refuses symlinks as both input and destination
- Caps input at 256 MiB, and expanded Office archives at 512 MiB
- Malformed or unsupported files fail without touching the source

## The desktop app

- Drag in files or folders, or recursively import a folder from the native picker
- Four panes: **Clean**, **History**, **Privacy**, and **Settings**
- Optional Windows File Explorer command across all 26 supported extensions — on Windows 11 it lives under **Show more options**
- Closing the window keeps MetaClean in the system tray; right-click the tray icon to reopen or exit
- Preserves JPEG display orientation and file timestamps by default without retaining private EXIF fields
- Finds stable updates through GitHub Releases, with automatic checks independently switchable off
- Twenty-four complete interface languages spanning Europe and Asia; system/light/dark theme, output mode, fidelity options, and local cleanup history persist between sessions

## Build from source

Requires [Rust](https://rustup.rs), [Node.js](https://nodejs.org) and [pnpm](https://pnpm.io), plus the [Tauri system dependencies](https://tauri.app/start/prerequisites/) for your platform.

```bash
pnpm install
pnpm tauri dev      # run the app in development
```

Run the full check suite before opening a pull request:

```bash
pnpm test:coverage                              # frontend tests, 80% floor
pnpm build                                      # typecheck + production bundle
cargo test --manifest-path src-tauri/Cargo.toml # Rust core tests
pnpm tauri build                                # platform installers
```

Pushing a version tag builds the whole matrix through GitHub Actions: NSIS and MSI for Windows, DMG for Apple Silicon and Intel macOS, and DEB/RPM/AppImage for Linux. macOS signing and notarization need the Apple secrets documented in the release workflow; without them the macOS job still produces unsigned bundles.

Test coverage and release evidence are tracked in [VALIDATION.md](VALIDATION.md).
Release changes are recorded in [CHANGELOG.md](CHANGELOG.md).

## Contributing

Pull requests are welcome — start with [CONTRIBUTING.md](CONTRIBUTING.md). For anything security-related, follow [SECURITY.md](SECURITY.md) and use private vulnerability reporting rather than a public issue.

## Responsible use

Process only content you own or are authorized to handle. MetaClean exists for privacy and file hygiene — not for academic fraud, false provenance, or misleading claims about where a file came from.

## License

[MIT](LICENSE) © Moresyl
