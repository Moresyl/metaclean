# Changelog

All notable changes to MetaClean are documented here. The project follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- System, light and dark interface themes with local persistence and
  no-flash initialization before React renders.
- Native MP4, MOV, M4V and M4A privacy cleaning for ISO BMFF/QuickTime user
  data, XMP, author and location atoms without re-encoding or moving media.

### Changed

- GitHub Actions now use their current Node 24-compatible major versions.
- CI runs for branch pushes and pull requests without duplicating the complete
  test suite for version-tag pushes.
- Windows Explorer integration now covers all 26 supported extensions.

### Security

- MP4/QuickTime parsing validates atom bounds, nesting depth, compatible brands
  and required media structure before writing; malformed containers fail closed.

## [0.2.0] - 2026-08-17

### Added

- Stable-release discovery backed by GitHub Releases, including a non-blocking
  launch check, a manual check, a visible update badge, strict release-link
  validation, and a switch that restores fully offline operation.
- Recursive folder intake from the native picker and drag-and-drop, with
  deterministic ordering, duplicate removal, visible skip reasons, and safety
  limits for depth and file count.
- Metadata cleaning for GIF comments/XMP; MP3 ID3v1, ID3v2 and APEv2 tags; WAV
  INFO, XMP, BWF, iXML and related chunks; and FLAC Vorbis comments, pictures
  and XMP application blocks.
- JPEG orientation preservation through a minimal orientation-only EXIF block,
  with an option to remove orientation as well.
- Optional preservation of access/modification timestamps. Output permissions
  are preserved in both safe-copy and replacement modes.

### Changed

- Windows Explorer integration now covers all 22 supported extensions.
- The file picker now has separate file and folder actions, and the queue has a
  dedicated audio-file presentation.
- Privacy documentation now distinguishes local file processing from the
  optional GitHub release request.

### Security

- Recursive intake refuses symbolic links, caps recursion at 64 levels and
  caps one intake at 10,000 files.
- Update discovery accepts only stable, non-draft releases and only opens links
  under the official `Moresyl/metaclean` GitHub release path.
- GIF, MP3, WAV and FLAC parsers reject truncated, out-of-bounds or structurally
  invalid containers before writing output.

### Fixed

- Cleaned copies and replacements no longer lose source permissions or change
  timestamps when timestamp preservation is enabled.
- JPEG cleanup no longer causes rotation changes in viewers that depend on EXIF
  Orientation.

## [0.1.0] - 2026-08-15

### Added

- Offline metadata scanning and cleaning for JPEG, PNG, WebP, PDF, Office,
  OpenDocument, text, Markdown, HTML, SVG, XML, JSON, and CSV files.
- Safe-by-default output with cleaned copies, backups before replacement, and
  atomic writes.
- Cross-platform desktop interface with drag-and-drop batches, cleaning
  history, privacy guidance, and persistent settings.
- English and Simplified Chinese localization.
- System tray integration with open and exit actions.
- Windows File Explorer context-menu integration for supported formats.
- Windows NSIS/MSI, macOS DMG, and Linux DEB/RPM/AppImage release packages.
- Continuous integration, automated multi-platform releases, and frontend and
  Rust test suites.

[0.1.0]: https://github.com/Moresyl/metaclean/releases/tag/v0.1.0
[0.2.0]: https://github.com/Moresyl/metaclean/compare/v0.1.0...v0.2.0
[Unreleased]: https://github.com/Moresyl/metaclean/compare/v0.2.0...HEAD
