# Changelog

All notable changes to MetaClean are documented here. The project follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Per-version validated bilingual release-note files and a final release job
  that publishes one SHA-256 manifest covering every uploaded package. Public
  release creation now waits for installed/copy-launched package smoke tests on
  Windows, macOS and Linux.
- An independently persisted ICC/sRGB preservation control for JPEG, PNG and
  WebP. Profiles remain intact by default and become an explicit informational
  scan finding when removal is selected.
- System, light and dark interface themes with local persistence and
  no-flash initialization before React renders.
- Native MP4, MOV, M4V and M4A privacy cleaning for ISO BMFF/QuickTime user
  data, XMP, author and location atoms without re-encoding or moving media.
- Twenty-six complete interface catalogs, spanning both Chinese scripts, Europe,
  East and Southeast Asia, Greek and Arabic. System-language detection,
  persisted selection, dynamic status messages and Arabic right-to-left layout
  are covered by automated completeness tests.
- Safe aliases for JPEG (`.jpe`), 13 additional ISO BMFF/QuickTime container
  extensions and seven UTF-8 text/markup extensions, increasing explicit intake
  from 26 to 47 extensions without enabling unsafe RAW or HEIC rewrites.
- Real installed-desktop E2E jobs on Windows, macOS and Linux/Xvfb, covering
  startup/navigation, all locales and Arabic RTL, persisted theme state and the
  Rust IPC boundary.

### Changed

- GitHub Actions now use their current Node 24-compatible major versions.
- CI runs for branch pushes and pull requests without duplicating the complete
  test suite for version-tag pushes.
- Windows Explorer integration and both NSIS/MSI uninstall paths now cover all
  47 supported extensions. CI verifies their manifests remain identical to the
  Rust intake and frontend classification lists.

### Security

- The production WebView now enforces an explicit local-only Content Security
  Policy. A CI regression gate rejects missing policy, `unsafe-eval`, and
  unbounded network sources.
- Every cleaned candidate is now format-checked and re-inspected in memory
  before MetaClean chooses an output path, creates a replacement backup or
  writes any bytes. Residual metadata or a changed format fails closed.
- MP4/QuickTime parsing validates atom bounds, nesting depth, compatible brands
  and required media structure before writing; malformed containers fail closed.
- WebdriverIO's vulnerable transitive archive extractor is replaced by a
  repository-owned patched package with an out-of-root symlink regression test;
  dependency audit reports no known npm vulnerability.

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
