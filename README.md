# MetaClean

[简体中文](README.zh-CN.md)

[![CI](https://github.com/Moresyl/metaclean/actions/workflows/ci.yml/badge.svg)](https://github.com/Moresyl/metaclean/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Moresyl/metaclean?include_prereleases)](https://github.com/Moresyl/metaclean/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/core-Rust-orange.svg)](src-tauri)

A local-first desktop app that scans and removes private metadata, Office revisions, PDF properties, and invisible Unicode before you share files.

## Screenshot

![MetaClean desktop interface showing the local file privacy cleaning workflow](assets/metaclean-screenshot.png)

## Download

Download the latest Windows, macOS or Linux package from [GitHub Releases](https://github.com/Moresyl/metaclean/releases/latest). No Python, Perl, ExifTool or cloud account is required.

## Supported formats

- JPEG: EXIF/GPS, XMP, IPTC, comments, and JUMBF/C2PA segments
- PNG: EXIF, textual metadata, and C2PA/JUMBF chunks
- WebP: EXIF, XMP, and C2PA chunks
- DOCX/XLSX/PPTX/ODT: author/application properties, comments, and custom XML; DOCX insertions are accepted and deleted revisions are removed
- PDF: removes Info/XMP and fully reserializes the document to discard old incremental metadata
- TXT/Markdown/HTML/SVG/XML/JSON/CSV: invisible Unicode plus generator/author metadata in Markdown, HTML, and SVG

Statistical text watermarks, pixel-domain watermarks, video, legacy binary Office files, and unknown binary formats are intentionally out of scope.

## Desktop experience

- Drop multiple files or open them from the native file picker
- Optional Windows File Explorer command for all 18 supported extensions (under **Show more options** on Windows 11)
- Closing the main window keeps MetaClean in the system tray; right-click the tray icon to reopen or exit
- Native minimize/maximize controls, persistent Chinese/English UI, output mode, and local cleanup history

## Safety

- 256 MiB input cap; 512 MiB expanded Office archive cap
- Refuses symlink inputs and destinations
- Writes to a temporary file and atomically replaces the destination
- Always creates a `.bak` file before replacing an original
- Creates a `.cleaned` safe copy by default
- No upload API, telemetry, or cloud processing

## Development

```powershell
pnpm install
pnpm tauri dev
```

```powershell
pnpm test
pnpm test:coverage
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build
```

Tagged releases build NSIS/MSI for Windows, DMG for both Apple Silicon and Intel macOS, and DEB/RPM/AppImage for Linux through GitHub Actions. macOS signing and notarization require the Apple secrets documented in the release workflow.

## Responsible use

Process only content you own or are authorized to handle. MetaClean is intended for privacy and file hygiene, not academic fraud, false provenance, or misleading claims.
