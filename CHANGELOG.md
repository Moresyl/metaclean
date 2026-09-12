# Changelog

All notable changes to MetaClean are documented here. The project follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Added `DESIGN.md`, a project-owned interaction and visual reference derived
  from the navigation, CTA, footer and information-grouping inspiration list
  supplied during design review, without publishing workstation paths.
- Added `docs/ARCHITECTURE.md` and an automated `pnpm test:docs` gate so the
  processing pipeline, IPC privacy boundary, ownership map and documentation
  claims have one reviewable source of truth.
- Added an MSI install/launch/uninstall smoke gate alongside the existing NSIS
  and portable checks; the release workflow now qualifies both Windows package
  families before collecting assets.
- Added `scripts/preflight-windows.ps1`, a non-publishing Windows candidate gate
  that builds the debug NSIS/MSI pair, packages the portable ZIP and runs all
  three install/launch/uninstall smoke checks in one repeatable command.
- Added a mixed-failure release benchmark covering 96 valid files, 16 unsupported
  files and 16 missing paths; it proves failed items do not abort valid cleanup
  or allocate an output file.

### Changed

- Parallel scanning now draws work from a bounded dynamic queue instead of two
  static path chunks, keeping both workers useful when file sizes or storage
  latency are uneven while returning reports in the original input order.
- Embedded image cleanup now combines risk counting with candidate decoding in
  one bounded pass, avoiding a second full decode before the required final
  verification scan.
- Secondary pages now load on demand, and Rollup keeps React, icons and each
  page family in stable cacheable chunks; the production entry chunk is now
  about 234 KiB instead of a single 525 KiB application chunk.
- Text and markup inspection now reuses the normalized Unicode pass and avoids
  allocating a second output when an ASCII file or embedded image needs no
  change; the native release benchmark records the new 128-file baseline in
  `VALIDATION.md`.
- README, README.zh-CN and VALIDATION now describe the five-page application,
  About support surface, 113-extension boundary and current automated evidence.
- The VitePress documentation chrome now uses Chinese labels for search, outline,
  edit, update and theme controls, and its homepage coverage metric is checked
  against the current validation ledger so visible facts cannot drift silently.
- Text intake now covers CSS, SCSS, Less and common INI/CONF/CFG/TOML/Properties
  configuration files, with frontend classification and Windows uninstall
  manifests kept in lockstep with the native Rust allowlist.
- Replaced the stale “未开工” project plan with a current-state roadmap and
  added a document index; repository descriptions now distinguish verified
  capability, external validation limits and the next evidence gates.
- Upgraded the desktop E2E service to 1.4.0 so embedded WebDriver runs no
  longer report a missing external `tauri-driver`; the configured Edge/WebView
  path remains the real Windows execution path.

### Fixed

- Atomic copy and replacement writes no longer turn a committed output into a
  reported failure when Windows needs a post-rename read-only permission
  sync; the sync is now best effort after the commit while all other metadata
  remains prepared on the temporary file.

- Direct scan, clean and directory-intake IPC now de-duplicate path aliases
  before enforcing the 10,000-item batch limit, so repeated drag/drop or shell
  aliases do not consume the user-visible budget.

- Local history loading now rejects oversized payloads and entries outside the
  native result, path and label bounds before they reach the history page,
  preventing malformed storage from causing an unbounded render or quota loop;
  corrupt top-level history is removed so the same failure is not retried on
  every launch.

- Refreshed the Windows release benchmark ledger with the latest 128-file
  clean/scan throughput and tail-latency evidence after the write and intake
  consistency fixes.

- Native update checks and package downloads now have a five-minute request
  deadline, so a stalled network cannot leave one-click update busy forever
  while still allowing normal installer-sized downloads to complete.

- Scan, clean and cancellation IPC now reject batch identifiers over 128 bytes
  before registering them, while tokenless legacy cleanup remains supported.

- History keeps the newest records within a 10,000-result render budget in
  addition to its 100-batch cap, preventing long-lived installations from
  accumulating a million-row history surface without presenting a truncated
  batch as if it were complete.

- Release asset collection and updater-manifest generation now share the
  client's exact stable `major.minor.patch` contract, rejecting prereleases,
  padded components and unsafe numeric versions before publishing metadata.

- Long history cards and result rows now use native content visibility, keeping
  off-screen audit rows accessible without paying their full paint cost during
  scrolling.

- History validation now rejects duplicate source paths before React receives
  them, preventing duplicate-key row reuse from showing the wrong audit result.

- Failed replacement rows now keep any generated backup path visible and add
  direct copy and reveal actions, including the same recovery path in history;
  users can locate an intact backup without guessing which failure details to
  expand.

- Native updater version checks now bound reviewed-version input and return a
  fixed changed-release message instead of echoing untrusted IPC or feed text.

- History persistence now checks the serialized storage budget before writing;
  an unusually large complete batch remains available in the current session
  without replacing an older recoverable history snapshot.

- Markdown front matter now recognizes both YAML `---` and TOML `+++` fences,
  while only removing targeted author and generator fields inside the opening
  metadata block.

- Markdown cleanup no longer consumes unrelated blank lines before a targeted
  front-matter field; indentation matching is now restricted to spaces and
  tabs on the field's own line.

- HTML metadata cleanup now parses quote-aware start tags, so `>` inside an
  attribute value, lookalike text in another attribute, and raw-text content in
  `script`, `style`, `textarea` or `title` are preserved while targeted
  metadata attributes are removed. HTML comments, including an unterminated
  comment at end of file, are also preserved; comment-like text inside
  attributes and ordinary less-than text is not treated as a comment.
  Similar-but-unrelated names such as `data-airport` are preserved.

- All localized supported-scope descriptions now state the current 113
  extensions and 24 text formats directly; the runtime no longer performs a
  broad numeric replacement that could alter unrelated translated copy.

- Recursive directory intake now applies the same Windows case, separator,
  device-prefix and UNC alias identity rules as direct IPC batches, preventing
  duplicate queue entries when overlapping paths are expanded; aliases are
  discarded before they can consume the 10,000-file result budget. Dot segments
  are folded lexically as well, while relative leading parents remain intact.

- HTML metadata cleaning now handles the legal unquoted form of generator,
  author, AI and C2PA attributes used by compact templates, with a regression
  proving only the targeted metadata nodes are removed.
- Malformed, oversized or unrecognized `data:image` payloads are now reported
  as residual embedded-image risk instead of being silently counted as clean;
  cleanup leaves those bytes untouched and the engine refuses a false-safe
  output.
- Cleanup calls from legacy clients that omit `batchId` now participate in the
  same RAII activity guard as cancellable batches, so closing the window cannot
  terminate an in-progress write merely because the compatibility token is
  empty.
- The native updater now validates the reviewed version as a stable three-part
  release before comparing it, so malformed, prerelease or directly-invoked
  IPC arguments cannot bypass the install contract; the frontend and Rust
  boundaries now reject the same malformed release shapes before an update can
  appear installable. Windows read-only source files also have a public-engine
  regression proving copy mode succeeds while replacement is rejected before a
  backup is created.
- Generic filesystem errors now say that a read/write operation failed instead
  of incorrectly labelling output, backup and replacement failures as input
  read errors.
- HTML/SVG metadata could be missed when a zero-width Unicode character split
  a sensitive attribute name, because structural matching ran before Unicode
  normalization. The scanner now normalizes first, the public UTF-16 engine
  path is covered, and nested SVG at the depth budget fails closed as residual
  instead of being reported clean.
- MSI smoke testing now refuses to overwrite an existing MetaClean installation
  and preserves diagnostic logs on failure instead of deleting evidence or a
  user's installed copy as part of candidate validation.
- Native benchmark reporting now records the first processed result before
  sorting latencies for p95, instead of incorrectly labelling the minimum
  latency as the first-result measurement.
- Directory expansion now participates in the same native read-task guard as
  scanning, so closing from the window, tray or application menu cannot race
  an unfinished recursive intake.
- Windows shell integration now has a recorded local round-trip smoke result:
  all 113 supported extension commands install and remove cleanly from HKCU.
- The Windows candidate preflight now runs that per-user shell integration check
  automatically and refuses to overwrite pre-existing command keys.
- A batch-level scan IPC failure now returns affected rows to the ready state,
  preserving the visible error while keeping the scan action available for a
  retry instead of trapping the queue in a false completed state.
- Native read and cleanup guards now move into their blocking workers and clean
  up through RAII; duplicate cleanup batch IDs are rejected without replacing
  the existing cancellation handle.
- Upgraded Vitest to 4.1.11 and pinned the WebdriverIO/Mocha `js-yaml` chain
  to 4.3.2, closing the current official npm audit findings without changing
  the production runtime dependency surface.
- Windows batch intake now treats ordinary, slash-normalized, device-prefixed
  and UNC spellings of the same path as one item, preventing duplicate work
  when Explorer or a launcher supplies mixed path forms.
- Update prompts now trap keyboard focus, expose their release notes to assistive
  technology and restore focus to the invoking control when dismissed. Repeated
  paths received through native IPC are de-duplicated before scan, expansion or
  cleanup so one file cannot be processed more than once in a batch.
- Command palette focus is now contained in the palette as well, including when
  its filtered result list has no enabled command.
- Queue rows now expose a direct copy-path action instead of requiring a context
  menu; the existing context menu remains available for keyboard and pointer
  workflows.
- The queue toolbar now copies all current output/source paths in one action,
  using one path per line for shell, ticket and review workflows.
- About-page diagnostics and path copying now share the same clipboard fallback,
  so locked-down WebViews can still use the selection-based copy path.
- Long cleanup batches now emit privacy-safe count-only progress events and show
  `completed/total` in the local status bar without exposing paths or content.
- Interrupted cleanup batches now leave only a path-free progress marker; the
  next launch explains what happened and asks for a safe re-import instead of
  guessing how to resume file operations. Progress persistence is throttled so
  recovery bookkeeping stays off the per-file hot path.
- A release-only native benchmark now exercises 128 nested mixed-size fixtures
  and reports scan/clean throughput plus first-result and p95 cleanup latency.
- Cleanup progress is now tagged with a per-run batch token, so delayed events
  from an earlier batch cannot overwrite the current status bar.
- Windows queue/result reconciliation now uses the same case-, slash-, device-
  prefix- and UNC-normalized path identity as the native batch boundary, so
  Explorer aliases cannot produce duplicate rows or false missing-result errors;
  Unicode case folding is shared as well for international filenames.
- The native cleanup request accepts the token as an optional field, preserving
  compatibility with older desktop clients while the current UI sends it.
- Large cleanup batches can now be cancelled safely between files; completed
  results remain recorded and unprocessed entries stay retryable.
- Window close requests are blocked while a native scan or cleanup task is
  active and explain that the user can wait or cancel cleanup, preventing
  silent partial work.
- Source bytes are now guarded together with modification time, readonly
  permissions and extended attributes before a cleaned copy or replacement
  backup is allocated, then checked again immediately before replacement. A
  source metadata race therefore fails closed instead of producing output from
  a stale candidate; a partially allocated readonly copy is also removed on
  that failure path.
- About-page project actions use link semantics, the app icon has explicit
  dimensions, status/error announcements are live regions, and long file
  queues skip painting rows outside the scrollport.
- Desktop E2E navigation now waits for lazy-loaded Settings sections and uses
  link selectors for the About support surface, keeping the real-app gate
  aligned with the production DOM.

## [0.7.1] - 2026-08-27

### Added

- A dedicated About page now exposes the exact running version, platform,
  architecture, application-data directory and executable directory, with
  native reveal and path-copy actions.
- Bounded diagnostics can be copied or saved as JSON for support. Reports
  include only runtime, update and application-path facts and never include
  processed files, cleaning history or file contents.
- Bug reports, feature requests, releases, source code and the MIT license are
  available from one project-support surface, including sidebar, command
  palette, keyboard and macOS menu navigation.

### Changed

- Update state and signed-install actions are shared by Settings, the update
  prompt and About instead of introducing an independent release path.
- GitHub opener access and native save-dialog access are limited to the exact
  official repository paths and capability required by the About page.

### Fixed

- One-click update installation now forwards the version the user reviewed as
  `expectedVersion`, matching the native command contract instead of failing
  with a missing required key.
- Project and support actions are no longer duplicated inside Settings; the
  dedicated About page provides the complete version, diagnostics and support
  workflow.

## [0.7.0] - 2026-08-26

### Added

- Native JPEG XL cleaning retires EXIF, XMP, JUMBF/C2PA, Brotli-wrapped
  metadata and JPEG reconstruction boxes without moving the codestream; naked
  codestreams are recognized and preserved byte-for-byte.
- Native AIFF/AIFC cleaning removes names, authors, copyright, comments, ID3,
  XMP and C2PA without re-encoding samples.
- OpenDocument support now spans 11 ODF document and template extensions,
  including properties, comments, tracked changes, optional settings and
  encrypted-metadata refusal. The real application allowlist grows to 105
  extensions across Rust, the frontend and Windows shell integration.

### Changed

- Input reads are capped at 256 MiB, refuse symbolic links and Windows reparse
  points, and re-check the source before guarded atomic replacement. Copy and
  backup allocation now use no-clobber creation instead of a check-then-write
  race.
- Batch intake and cleaner execution have strict path, traversal, decompression,
  nesting and recursion budgets. Per-file scanner and cleaner panics are
  isolated so one malformed input cannot abort the rest of a batch or expose a
  parser payload.
- JPEG, PNG, WebP, HEIF/AVIF, TIFF/RAW, BMP, GIF, MP3, FLAC, AVI, ASF, PDF,
  Office and embedded-image validation now rejects ambiguous, truncated,
  overlapping or structurally inconsistent metadata instead of attempting a
  best-effort write.

### Fixed

- Missing, duplicate or foreign native scan/cleanup responses can no longer
  mark unrelated files complete or hide work that remains retryable.
- Failed cleanup rows remain actionable, concurrent double-submission is
  blocked before React can rerender, and a new scan clears stale reports and
  output results.
- Corrupt or unavailable browser storage now degrades safely; persisted history
  is structurally validated and bounded to the newest 100 entries.
- OpenDML AVI chains, progressive JPEG metadata after scan data, PNG chunk
  ordering/CRC, WebP extended headers, HEIF item extents, APEv2/ID3 footers,
  PDF embedded JPEG filters and mixed-case Office part names are handled or
  refused according to their container contracts.

## [0.6.1] - 2026-08-25

### Added

- Settings now links directly to the official repository and issue tracker, and
  offers an explicit choice between exiting on close and staying in the tray.
- Signed update checks now fall back from the GitHub Release asset to the
  official GitHub Pages feed when the primary manifest cannot be reached.

### Changed

- Reworked the desktop visual system around shared button and select controls,
  consistent spacing and color tokens, clearer queue states, and scroll-safe
  settings and cleanup panels in both light and dark themes.
- The fallback updater feed is generated only from the exact signed manifest of
  a successful official release and validates every supported platform URL and
  signature before GitHub Pages deployment.

### Fixed

- Restored drag, minimize and close behavior for the custom title bar by adding
  the exact Tauri window capabilities and an explicit non-interactive drag
  handler. Closing exits by default instead of silently appearing inert.
- Update checks and downloads now lead with actionable bilingual network help
  instead of exposing only `error sending request for url`. Native update
  resources are always released, and installation refuses a version that
  changed after the user reviewed it.

## [0.6.0] - 2026-08-24

### Added

- HTML, XHTML, SVG and Markdown now inspect and clean metadata inside embedded
  raster image data URIs, including nested SVG. Processing is bounded to 100
  images, 16 MiB per payload and four nesting levels.
- PDF cleanup now reaches JPEG image XObjects and removes their EXIF, XMP,
  IPTC, comments and provenance blocks while preserving orientation and color
  profiles.
- The queue can export a versioned local JSON audit report containing the
  summary, findings and output status without exposing raw metadata values.
- WAV C2PA chunks and ID3v2/C2PA prefixes before FLAC streams are detected and
  removed without re-encoding audio.

### Changed

- The desktop shell is rebuilt as a fixed 1180 × 720 privacy workspace with a
  compact 60-pixel icon rail, 36-pixel product caption, balanced two-column
  work area and persistent 26-pixel local-status bar.
- Invisible-text cleanup now covers all 66 Unicode noncharacters and reserved
  default-ignorable ranges while preserving legitimate emoji, complex-script,
  CJK variation, flag-tag and directional-text sequences contextually.
- Cleaning, history, privacy and settings surfaces now share one compact type,
  spacing, control-height, panel and state hierarchy.

### Fixed

- Floating bidi overrides and malformed directional controls no longer survive
  text cleanup, while valid paired embeddings and isolates remain intact.
- FLAC files prefixed with an ID3v2 tag are now recognized by the public engine
  instead of being rejected before the native FLAC cleaner can run.
- Deep metadata embedded in PDFs and markup can no longer pass a successful
  top-level scan unnoticed.

## [0.5.0] - 2026-08-24

### Added

- MetaClean now cleans the container families it previously refused, each with a
  strategy that removes metadata without relocating a single byte, so no strip,
  cue, index or item offset ever shifts:
  - TIFF and 23 camera raw formats, by compacting image file directories in
    place and rewriting the pointer behind them. Strip, tile, MakerNote-relative
    and embedded-preview offsets all stay valid.
  - Fujifilm RAF, by cleaning the embedded JPEG preview — which carries the full
    EXIF block, GPS and serial number included — and zero-padding it back to its
    original extent.
  - HEIC, HEIF, AVIF and Canon CR3, at item granularity: EXIF, XMP and C2PA item
    payloads are zeroed where they lie in `mdat` and their extents set to zero,
    leaving the item table that locates the picture intact.
  - AVI, by renaming private chunks to RIFF's own `JUNK` padding tag and zeroing
    the payload, so `idx1` keeps its meaning under either offset convention.
  - Matroska, WebM, MKA, MKS and MK3D, by stamping the EBML `Void` identifier
    over a tag block and widening its length field to absorb the difference.
  - ASF, WMV and WMA, by stamping the padding GUID over content-description and
    metadata objects while the header's object count stays honest.
  - BMP and DIB, covering the reserved header words, V5 embedded ICC profiles
    and EXIF or XMP stapled past the last pixel.
- EPUB is now cleaned alongside the other office formats. Dublin Core terms that
  name a person or a moment are removed, reader sediment from Calibre, Sigil,
  Kobo, epubcheck, Apple and Adobe is deleted, and the `dcterms:modified`
  timestamp EPUB 3 refuses to live without is pinned to the epoch rather than
  dropped. The identifier, title and language the specification requires are
  preserved, because an EPUB missing one is a broken file rather than a private
  one.
- A command palette reaches every window command from the keyboard, and
  right-click context menus are drawn for the surfaces that lost the system ones
  when window decorations were turned off.
- Expanding a file in the queue opens a detail panel with its detected format,
  source, output and backup paths, and every finding category labelled as
  removed, kept or pending.
- Six interface languages: Català, فارسی, Hrvatski, Magyar, മലയാളം and Tiếng Việt.
  Persian joins Arabic in right-to-left layout.

### Changed

- The intake allowlist grew from 47 to 91 extensions across the Rust engine,
  frontend classification, and the NSIS and MSI shell integrations.
- The window draws its own title bar, and grew by exactly the caption's 32
  pixels to 1100 by 602 so the interface below it kept its full height.
- Interface chrome was rebuilt against Windows 11 Fluent metrics, with a green
  accent that no longer tints the neutral surfaces around it.
- Tooltips are served by a single document-level host that reads `data-tip` off
  whatever the pointer or focus ring lands on.
- The shipped locale count is now derived from the locale list by the unit,
  release and end-to-end tests rather than written down in three places.
- `SUPPORT_POLICY.md` now documents the offset-preserving strategy and test bar
  used by every newly supported container family.

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
[0.5.0]: https://github.com/Moresyl/metaclean/compare/v0.4.2...v0.5.0
[0.6.0]: https://github.com/Moresyl/metaclean/compare/v0.5.0...v0.6.0
[0.6.1]: https://github.com/Moresyl/metaclean/compare/v0.6.0...v0.6.1
[0.7.0]: https://github.com/Moresyl/metaclean/compare/v0.6.1...v0.7.0
[0.7.1]: https://github.com/Moresyl/metaclean/compare/v0.7.0...v0.7.1
[Unreleased]: https://github.com/Moresyl/metaclean/compare/v0.7.1...HEAD
