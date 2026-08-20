# Changelog

All notable changes to MetaClean are documented here. The project follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.4.2] - 2026-08-20

### Added

- The installed MetaClean version is now always visible in the sidebar and in
  Settings without waiting for a network update check.
- Available updates now open a dedicated prompt with the target version,
  publication date and concrete release notes. Dismissal is remembered per
  version, and the prompt can be reopened from the update badge.

### Changed

- Settings now use a compact native-style category layout for appearance,
  cleaning preferences, system updates and safety controls.
- The desktop window is fixed at 1100 by 570 pixels, cannot be maximized or
  resized, restores position only, and blocks keyboard and wheel zoom shortcuts.
- Interface typography now uses a small shared type scale with black text in
  light mode and white text in dark mode, while flatter controls and system
  fonts make the application feel closer to a native desktop utility.

### Fixed

- Fixed Issue #2 by limiting the application menu to macOS. Linux AppImage and
  Windows builds no longer display the empty WebView window menu.
- Removed the fullscreen command from the remaining macOS Window menu so the
  fixed desktop layout cannot be expanded through the native menu.

## [0.4.1] - 2026-08-18

### Added

- Linux AppImage builds now embed the standard `gh-releases-zsync` update
  information and publish a matching `.AppImage.zsync` asset. Compatible
  AppImage update tools can discover the latest GitHub Release and download
  only changed blocks.
- Release gates now inspect the Windows PE subsystem and the AppImage
  `.upd_info`/zsync pair before any package can reach the public release.

### Changed

- The AppImage delta channel is generated during the original Tauri bundle so
  the existing minisign signature continues to cover the final AppImage used
  by MetaClean's in-app updater.

### Fixed

- Windows release executables now use the GUI subsystem and no longer open an
  extra black console window when MetaClean starts.

## [0.4.0] - 2026-08-18

### Added

- Cryptographically signed in-app updates for installed Windows builds, both
  macOS architectures and Linux AppImage. The Settings page reports download
  progress, installs the verified package and restarts the application.
- A five-target updater release pipeline that collects signed NSIS, macOS app
  archives and AppImage artifacts, generates a complete static `latest.json`,
  and includes every updater file in the SHA-256 release manifest.
- Explicit runtime capability detection and a package marker for Windows
  portable ZIPs. Portable and non-AppImage Linux builds open the official
  release page instead of attempting an unsafe in-place self-update.

### Changed

- Stable update discovery now uses the native Tauri updater over Rust TLS
  instead of a WebView GitHub API request. Automatic checks remain optional and
  the production WebView CSP remains local-only.
- Download and installation are owned by a narrow Rust command; the frontend
  receives only updater check permission and progress events.

### Security

- Every in-app update must pass the embedded minisign public-key check. Signature
  verification cannot be disabled by a release manifest or remote response.
- Signing is enabled only by the release-specific Tauri configuration, keeping
  private keys out of the repository and ordinary local builds while requiring
  encrypted GitHub Actions secrets for public updater artifacts.

### Fixed

- Fixed production update discovery being blocked by the intentionally strict
  WebView `connect-src` policy even though mocked browser tests passed.
- Update errors now degrade to a visible retry state, concurrent checks remain
  deduplicated, and dismissed versions remain locally persisted.

## [0.3.0] - 2026-08-17

### Added

- Windows x86 NSIS plus x64/x86 portable ZIP release targets. Both installed
  and portable executables must remain open with the expected window title for
  six seconds before their packages can reach the final release job.
- Native desktop menus and navigation accelerators, persisted window size,
  position and maximized state, plus dedicated installed-app accessibility and
  fail-closed IPC scenarios.
- Opt-in macOS provenance-attribute removal. All extended attributes are
  preserved by default; removal is restricted to six known download,
  quarantine and provenance keys, with unrelated attributes retained and a
  real macOS filesystem regression in CI.
- Stable queue sorting by name, extension, source/output size or actionable
  finding count, plus per-file before/after size deltas and a scoped native
  reveal-in-folder action for completed outputs.
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

- WebdriverIO's `deepmerge-ts` chain is forced to 8.0.0, which adds bounded
  circular-reference handling for CVE-2026-40345; the supply-chain gate runs
  both vulnerable recursive-object public API shapes.
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
[0.3.0]: https://github.com/Moresyl/metaclean/compare/v0.2.0...v0.3.0
[0.4.0]: https://github.com/Moresyl/metaclean/compare/v0.3.0...v0.4.0
[0.4.1]: https://github.com/Moresyl/metaclean/compare/v0.4.0...v0.4.1
[0.4.2]: https://github.com/Moresyl/metaclean/compare/v0.4.1...v0.4.2
[Unreleased]: https://github.com/Moresyl/metaclean/compare/v0.4.2...HEAD
