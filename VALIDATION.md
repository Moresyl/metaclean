# MetaClean validation status

Last audited: 2026-09-29

This file records evidence, not intent. A row is complete only when the named artifact or runtime check exists.

## Milestones

| Gate | Status | Evidence |
|---|---|---|
| M0: PDF structural rewrite | Complete | `drops_metadata_bytes_from_incremental_history` proves old Info metadata bytes are absent after full `lopdf` serialization. |
| M0: image/media format decision | Complete | The shared 116-extension allowlist covers native still-image, RAW, audio, video, document, PDF and text/markup cleaners. TIFF/RAW, HEIF/AVIF/CR3, AVI, Matroska/WebM and ASF families use offset-preserving strategies; WAV C2PA and ID3-prefixed FLAC are covered by malformed-input and residual-trace tests. |
| M0: Office package integrity | Partial | Real DOCX/XLSX/PPTX/ODT samples were cleaned and successfully opened/exported by LibreOffice 26.2.5. WPS Office 2019 (11.8.6.11825) DOCX/XLSX/PPTX sample round trips pass with the post-v0.11.0 VML fix described below. Microsoft Word, newer WPS versions and complex document fidelity remain unverified. |
| M1: desktop MVP | Complete | The fixed 1180 × 720 workspace provides a persistent 264px sidebar that collapses to a 64px accessible icon rail, five compact navigation destinations, command palette, native menus, local status bar, per-file reports and value-free JSON audit export. The 44px caption, 26px status bar, neutral light/dark themes, safe-copy/replace, backups, atomic writes, fidelity controls, signed update handling and 32 complete locales are implemented and tested. |
| M2: Office/PDF/shell integration | Complete | DOCX/XLSX/PPTX/ODT/EPUB and PDF cleaners, deep PDF JPEG cleanup, embedded markup data-URI cleanup, 116-extension Windows Explorer integration and launch-path handling are covered by unit and manifest-consistency tests. |
| M3: Windows release | Complete for v0.3.0 | The successful v0.3.0 release matrix published launch-smoked x64 NSIS/MSI, x86 NSIS and architecture-labelled x64/x86 portable ZIPs. Local installation/extraction proof also kept each package active for six seconds with the `MetaClean` title before clean uninstall/removal. |
| M3: macOS/Linux release | Complete for unsigned v0.3.0 artifacts | The successful v0.3.0 matrix copied and launch-smoked both Intel and Apple Silicon DMGs, then installed and launch-smoked the Linux DEB before publishing DEB/RPM/AppImage assets. Apple signing/notarization secrets remain unavailable, so Gatekeeper qualification is an external gate rather than a completed claim. |

## Automated quality gates

- v0.10.0 candidate: frontend 415 tests pass; native library 208 pass and three
  explicitly gated tests remain ignored. Rust line coverage is 84.32% using the
  CI exclusion set. Strict Clippy, formatting, production build and 40 release
  automation tests pass. Two Windows desktop runs each pass all 14 scenarios;
  the added scenario independently hashes actual source/output bytes and checks
  persisted history and expanded details. Native visual inspection confirms the
  collapsible checksum section fits inside the scrolling queue.
- Candidate performance with SHA-256 enabled: 128 files / 25,014,272 bytes clean
  at 204.71 files/s, first result 2.38 ms and p95 9.75 ms. Mixed batch: 96 valid
  and 32 failed inputs, 259.68 files/s, first result 3.28 ms and p95 9.84 ms.
  These workstation measurements are not hardware-independent guarantees.
- npm audit reports no known vulnerabilities. Cargo audit passed against the
  freshly fetched official advisory database at
  `ef03605143a913024f864d2edf476adad5720c93`; eight upstream warnings remain,
  including maintenance, GLib soundness and a yanked transitive version.
  The initial database update failed over the network; a fresh Git clone and
  `cargo audit --db ... --no-fetch` completed the same lockfile audit.
- v0.10.0 was published by successful release run `36448742163`. Independent
  verification run `36452434323` downloaded public artifacts, checked 19 asset
  SHA-256 hashes, verified all five updater package signatures against the tagged
  public key and rejected deliberately tampered package bytes. Pages run
  `36451199277` succeeded and its updater URLs/signatures match the release feed.

## v0.11.0 release validation

- The Opus/Vorbis kernel and product extension intake integration are implemented.
  Release `v0.11.0` at `67b6947f94c5cdd20ea44c04bf772b0ce1c71505` was published
  on 2026-09-28 at 17:59:11 UTC with 20 public assets. Candidate CI
  `36457229373` passed the main checks and all three desktop platforms. Release
  run `36459432379` passed source validation, all five platform builds, applicable
  installer/portable/application smoke tests and finalization.
- Frontend: 418 tests pass, line coverage 93.40% and branch coverage 85.09%.
  Production/documentation builds, format manifests, CSP, supply-chain and all
  40 release checks pass. Two Windows desktop runs each pass 15 scenarios,
  including importing all three Ogg extensions, cleaning copies and native
  reinspection. A real HKCU Explorer command round trip validates all 116
  extensions and removes the test keys cleanly.
- Native library: 219 tests pass, four explicitly gated tests are ignored by
  default. Eleven focused Ogg tests cover comment framing, channel mappings,
  page checksums/sequencing, cross-page comments, mixed codec chains, unchanged
  setup/audio bytes and malformed headers. Strict Clippy passes. Core Rust line
  coverage is 84.96% with the CI exclusion set; the Ogg module reaches 98.35%.
- `pnpm test:audio` independently generates and decodes ten synthetic fixtures
  using FFmpeg: mono, stereo, 5.1, 150,000-character comments and chained streams
  for each codec. All ten pass locally with FFmpeg 7.1: unchanged source files,
  successful clean reinspection, removed artist/title, retained numeric playback
  gain tags, equal file length and identical decoded PCM SHA-256. The command
  requires FFmpeg with libopus/libvorbis on PATH, or `METACLEAN_FFMPEG` set to its
  executable. FFmpeg is a test tool, not an application runtime dependency.
- The decoder test resets output timestamps for chained-stream PCM comparison;
  this does not alter samples. Codec setup/audio payloads are preserved rather
  than fully decoded by the cleaner. Unsupported Ogg codecs and malformed
  structures are refused. Linux CI and release validation both passed this check;
  inspected Linux CI logs record all ten PCM comparisons and 15 desktop scenarios.
- Independent public-download verification `36462092583` passed: all 19 assets
  listed in the checksum manifest matched SHA-256, all five updater packages
  verified against the tagged public key, and modified bytes failed signature
  verification. Pages run `36462010031` succeeded; a direct request to the public
  updater feed returned version `0.11.0` and the same five URLs/signatures as the
  release manifest.

- v0.9.0 native workflow evidence (2026-09-26): all 14 desktop E2E scenarios passed, including real file intake, search, scan and safe-copy cleanup with original-content verification. Both localized capture scenarios passed and generated seven native screenshots per language plus two finite-loop GIFs. English and Chinese default cleanup preferences fit the fixed window without scrolling; documentation checks enforce language-specific image references, dimensions and GIF size budgets.

- Desktop visual verification (2026-09-26): the real 1180 × 720 Windows WebView passed all 13 desktop E2E scenarios and an additional light/dark capture check. The shared README/user-guide screenshot was captured from that native build. Empty-state options fit without scrolling; the action stays pinned. Browser queue intake and light-theme layout were also inspected.

- Latest full frontend run (2026-09-26): 401 tests. Statements 89.57%, branches 84.87%, functions 91.79%, lines 93.39%. Scan, cleanup, About and Windows shell responses are structurally validated before they reach React state, reconciliation or history.

- Frontend: 401 tests. Statements 89.57%, branches 84.87%, functions 91.79%, lines 93.39%. Application integration tests now verify the persistent sidebar preference and `Ctrl/Cmd+B` toggle in addition to queue and persisted-history confirmation, cancellation and final clear behavior; title-bar coverage verifies the accessible expand/collapse action. The app suite also proves that a drag-drop listener which finishes registering after unmount is immediately released, that a later event subscription failure releases earlier listeners, and that an established listener is released while a later subscription remains pending, that late scan and cleanup results cannot update a remounted app, and that stale cancellation responses cannot alter a newer batch, while settings tests prove stale context-menu requests from an unmounted instance cannot overwrite a newer instance and repeated toggles are serialized until the first native result returns. Update-context tests also prove an obsolete provider cannot receive a late check result after unmount, and portable release-page launches are serialized and surface opener failures without an unhandled rejection. About-page tests prove a pending save dialog cannot continue into native report export after that page unmounts and that a failed release-note opener reaches the page error surface. Update utility tests verify a valid signed update remains usable when native cleanup fails, listener cleanup cannot mask a successful restart, malformed download progress cannot enter UI state, and invalid versions do not register native listeners. Queue tests prove audit export is serialized before its save dialog resolves and clipboard notifications stop after unmount. The queue suite covers audit export as well as sorting, size deltas, source/output/backup and batch path copying (including generated backups and Windows alias de-duplication), generated-backup copy/reveal actions, errors and reveal actions; destructive queue clear and row removal now require a focus-contained confirmation dialog, while status-bar state/version, count-only scan and cleanup progress, cancellable scanning and cleanup, incremental retry without discarding completed reports, stale progress-event isolation, crash-safe interrupted-batch recovery markers, Windows path-alias reconciliation including slash-form UNC paths, duplicate native results with first-result preservation, serialized updater checks/installs with fail-closed reviewed-version handling, oversized release-note omission, stale-version clearing after failed re-checks and direct signed installation from the update prompt, focus-contained dialogs, lazy page boundaries, support links, command-palette and context-menu active-descendant semantics, disabled-command keyboard skipping, parent-rerender highlight preservation, command replacement reanchoring, disabled-only result semantics, update-dialog external-link teardown and current localized scope counts are also exercised. Confirmation teardown returns focus to main when a trigger disappears or becomes disabled. App path intake and reveal actions also refuse to start or continue native work after the page unmounts. About-page copy feedback reuses and disposes its timer safely across rapid copies and unmounts, and announces success through an aria-live status node. History persistence additionally rejects oversized and structurally unbounded local payloads before rendering, rejects duplicate entry IDs and duplicate Windows path identities, rejects error diagnostics beyond the native 8 KiB UTF-8 budget, rejects oversized or multibyte crash-recovery markers before parsing, validates drag-and-drop event payloads before queue intake, bounds serialized snapshots before writing, and clears corrupt top-level storage after detection. Version comparisons also cover integers beyond JavaScript's safe-number range. Error-surface tests additionally prove oversized browser/plugin failures are UTF-8-byte bounded before entering user-visible React state, including hostile Error objects whose message coercion throws; multibyte paths, labels, identifiers and dates are rejected when their UTF-8 byte size exceeds the native contract, malformed navigation or close-blocked events are ignored before changing UI state, and browser FileList fallbacks cap intake and skip malformed File-like metadata. Drag/drop and folder-pick path normalization now de-duplicate platform aliases while reading, allow 10,000 unique paths, reject aggregate drag payloads over 64 MiB, keep hover lifecycle events closable even when their path payload is malformed, reject browser selections over 10,000 items, reject revoked or throwing boundary arrays, cap bulk path copy at 16 MiB, preflight audit JSON at 10 MiB and abort after queue unmount, and reject more than 40,000 raw items. Browser folder selections now include their bounded relative path in the queue identity, so same-metadata files from different folders remain separate; Windows browser identities also normalize slash and case aliases before de-duplication. Finding totals now cap at JavaScript's maximum safe integer across scan messages, queue sorting and audit summaries. The visible queue also enforces the 10,000-file native ceiling across repeated imports and reports entries that do not fit. Picker fallback tests now distinguish an unavailable native dialog from malformed or oversized picker responses, surfacing invalid data without opening a second browser picker; App integration now keeps the browser fallback picker available from command-palette actions on any page and reports invalid browser selections instead of silently dropping them. DropZone and the App-root picker now also report revoked, malformed or fully rejected browser file lists instead of silently ignoring a failed selection; native drops whose structurally valid payload contains no processable paths now surface an explicit error instead of silently disappearing.
- Batch path copying includes every available source, generated output and backup path in display order, with platform-aware de-duplication.
- Documentation visual QA: local VitePress rendering at the 1024px browser viewport keeps the hero proof card beside the title despite the persistent sidebar; the mobile breakpoint alone collapses it to one column, and `pnpm test:docs` guards the breakpoint contract.
- Documentation 404: VitePress `themeConfig.notFound` provides a localized recovery page with safe local-processing copy and a return-to-docs action; the generated preview contains the configured 404 metadata and `pnpm test:docs` verifies the recovery contract.
- Localized catalogs carry the current 116-extension/24-text-format scope directly; the locale tests verify every published language without runtime number rewriting.
- Frontend history persistence rejects local payloads over 2,000,000 characters and any entry over the native 10,000-result batch bound before parsing or rendering; path, error and label fields also have explicit length limits, and retained history keeps only complete batches within a 10,000-result render budget overall.
- History persistence also bounds the serialized snapshot before writing: an oversized newest batch stays available for the current session without replacing an older recoverable snapshot.
- History cards and result rows use native `content-visibility` containment so the bounded audit surface remains responsive while preserving the full accessible DOM.
- History loading also rejects duplicate Windows path identities before rendering, matching the native batch de-duplication invariant.
- Failed cleanup rows keep a generated backup path visible and directly actionable, so a replacement failure does not hide the recovery artifact behind an expanded details panel.
- Native updater installation uses a five-minute request deadline for both feed checks and package downloads; the frontend keeps its shorter 15-second discovery timeout for responsive manual checks.
- Native updater reviewed-version inputs are bounded to 128 UTF-8 bytes during Serde deserialization before parsing, and a changed-version error never echoes untrusted feed or IPC text into the UI. Batch identifiers use the same pre-allocation bound for scan, cleanup and cancellation commands while preserving legacy empty cleanup tokens. Audit-report export bounds its destination path and JSON body during Serde deserialization to 32 KiB and 10 MiB before filesystem work.
- Frontend and release tooling bound release-note text at 64K characters before it enters application state or a signed feed; oversized notes are rejected at publish preparation and omitted from runtime state.
- Release asset collection, the frontend updater and native updater use the same bounded stable three-part, no-leading-zero, safe-integer version contract; prerelease, overlong and malformed metadata are rejected before an update feed is written or installed.
- Embedded image cleanup combines bounded risk counting and candidate decoding in one pass, followed by the required independent verification scan.
- Rust: 211 tests total: 208 pass and three explicit tests are ignored by default: the two native batch benchmarks plus the external Office compatibility test, the latter requiring `METACLEAN_OFFICE_SAMPLE_DIR` with real fixtures. Scan and cleanup batch registries reject duplicate tokens and support file-boundary cancellation; tokenless legacy cleanup remains backward compatible while still holding the close guard for the full worker lifetime. Direct scan, clean, directory-intake and startup-argument IPC reject empty paths, cap each path at 32 KiB and the raw request at 64 MiB while streaming deserializers prevent unbounded path-vector, batch-ID, reviewed-version and audit-report materialization, de-duplicate platform path aliases while reading, allow 10,000 unique paths and reject more than 40,000 raw items; recursive expansion applies the same output budget and truncates issue-path echoes at the single-path boundary; the audit-report destination path and body are bounded at 32 KiB and 10 MiB before filesystem work. Parser, filesystem, updater and intake diagnostics are capped at 8 KiB before IPC/UI rendering, with UTF-8-safe truncation. Directory intake also applies the same case, separator, device-prefix, UNC alias and lexical dot-segment identity rules during recursive expansion. Explicit intake paths reached through a symlinked or reparse-point parent are rejected before entering the queue, while recursive traversal avoids rechecking the same parent chain for every child. The two bounded scan workers draw dynamically from uneven work while preserving input-order results; count-only progress and forced out-of-order completion are regression-tested without exposing paths through the desktop event. Batch progress events are throttled to 16 items or 50ms, terminal completion/cancellation counts are always flushed, and WebView emission occurs outside the progress lock. Embedded image cleanup combines bounded risk counting and candidate decoding in one pass, followed by the required independent verification scan. Text cleaners round-trip UTF-8 with/without BOM and BOM-marked UTF-16 LE/BE while preserving encoding, BOM and CRLF; malformed UTF-16 surrogate data is rejected before output allocation; a 1 MiB single-line input is cleaned without truncation; Unicode normalization happens before structured HTML metadata matching; Markdown YAML `---` and TOML `+++` front matter is bounded to the opening metadata block and targeted field removal preserves unrelated blank lines; HTML metadata cleanup uses quote-aware start-tag and attribute scanning, preserves `>` inside values and lookalikes in other attributes or raw-text elements and HTML comments (including unterminated comments), while attribute-embedded markers and plain less-than text are preserved rather than treated as comments, removes only targeted metadata, and preserves unrelated names such as `data-airport`; malformed, oversized and unrecognized declared embedded images remain residual rather than being reported clean; SVG nesting beyond the audit budget follows the same fail-closed rule; and a Windows read-only source is copied safely while replacement is rejected before a backup is created. Safe I/O now rejects symlinked or reparse-point parent directories as well as linked final files before reads, copies or replacements. Atomic writers prepare metadata before commit, re-check the source after the temporary output is synced and immediately before replacement, require the replacement target to remain a regular file, and treat Windows post-rename readonly synchronization as best effort, so a committed output cannot be returned as a write failure. The native updater also rejects malformed or prerelease reviewed versions before installation, applies a five-minute request deadline to checks and downloads, bounds batch identifiers, reviewed versions, audit-report fields and path payloads at the IPC boundary, and generic I/O failures no longer mislabel write failures as read errors. Strict Clippy passes with warnings denied.
- Installed desktop E2E: all 13 WebdriverIO scenarios pass locally against the rebuilt Windows webview. `pnpm test:e2e` now owns its E2E-feature build step, so a normal debug candidate generated by Windows preflight cannot silently replace the embedded-WebDriver binary and make the next run time out before tests start. The scenarios cover startup, keyboard navigation, persistent 264/64px sidebar interaction, all 32 locale options and RTL, named controls/landmarks, About support links, theme and fidelity persistence, the Rust IPC boundary including a missing-batch cancellation response, updater capability and fail-closed missing-input paths. Native tests cover the shared read-task guard used by recursive intake and scanning, RAII cleanup for blocking workers, and duplicate batch-ID protection. The embedded WebDriver diagnostic now reports its configured provider correctly; the upstream service still emits a non-blocking mock-store cleanup warning after the session is already torn down. The scenarios execute through the configured Edge/WebView path and pass 13/13.
- Frontend regression tests cover update-dialog focus containment/restoration, direct queue path copying, duplicate-safe native batch handling, delayed progress-event isolation, scan/cleanup cancellation, retrying only incomplete scan entries and path-free interrupted-batch recovery; recovery progress is throttled off the hot path and native close requests are blocked while an active scan or cleanup task is registered.
- Cleanup candidates are re-detected and re-inspected before output-path allocation, backup creation or writes. JPEG/PNG/WebP tests cover both ICC preservation and explicit removal, while an engine regression rejects residual traces and format changes.
- On macOS, every extended attribute is copied by default. Opt-in removal filters only six known provenance/download keys; CI runs a real filesystem round trip proving a private key is removed while a custom key survives.
- CI fails below 80% for all frontend coverage dimensions and below 80% Rust core line coverage.
- `pnpm test:formats` proves the 116-extension Rust intake list, frontend classification, NSIS cleanup, MSI cleanup, both READMEs and support policy are complete, duplicate-free and identical.
- `pnpm test:security` proves production has a non-null local-only WebView CSP, rejects `unsafe-eval`, wildcard sources and unbounded HTTP(S) connections, and limits opener access to the official release URL plus reveal-in-folder.
- Rust guarded-write tests cover byte, modification-time and permission races; the guarded-write path also compares readonly permissions and extended attributes before a copy/backup is allocated or a replacement is committed.
- `pnpm test:release` runs 37 checks proving version metadata, the 1180 × 720 caption/status layout, the camelCase updater version argument, bounded release notes and tag-specific bilingual notes stay synchronized; package/updater collection, signatures, AppImage zsync, GUI subsystem, `latest.json` and SHA-256 manifests are validated before publication.
- The release workflow now checks out the exact requested tag in a dedicated `validate` job and blocks every platform build until supply-chain, CSP, release, format, documentation, npm audit, frontend coverage/build, Linux Xvfb desktop E2E, Rust format/test/coverage and Cargo audit gates pass on that same source.
- `pnpm test:docs` proves the package metadata, bilingual product descriptions, architecture/document index, design reference boundary and external-validation wording remain synchronized without publishing workstation paths.
- pnpm's official npm audit endpoint reports no known dependency vulnerability. Vitest is pinned to 4.1.11, and the WebdriverIO/Mocha `js-yaml` chain is forced to 4.3.2. WebdriverIO's unfixed `extract-zip` 2.0.1 dependency is replaced on every path by the repository-owned `vendor/extract-zip` 2.0.2 package, which rejects out-of-root symlink targets. Its `deepmerge-ts` chain is forced to 8.0.0 for CVE-2026-40345. `pnpm test:supply-chain` proves both the archive escape and recursive-object denial-of-service regressions are closed. Patched `glob` and `serialize-javascript` versions are also forced through workspace overrides.
- `cargo audit -f src-tauri/Cargo.lock` uses the 2026-09-23 RustSec database. The updater transport is locked to `rustls 0.23.45`, resolving RUSTSEC-2026-0285; the audit still emits eight allowed warnings from inherited GTK/Tauri and Unicode dependency families, including RUSTSEC-2024-0429 in `glib`, plus a yanked `chacha20` entry. Registry yanked-status lookups can time out in this environment, so those warnings remain tracked rather than represented as resolved.
- Production frontend build, TypeScript checking, Rust tests, Rust formatting and both coverage gates pass locally; CI-equivalent `cargo llvm-cov` reports 84.28% Rust line coverage after the embedded-image, quote-aware HTML metadata, Windows intake, IPC path-budget, bounded-diagnostics, atomic-write and output-path consistency hardening. The production bundle contains no WebdriverIO plugin marker; both Rust test plugins are optional and registered only by the `e2e` Cargo feature.
- A repeatable native batch benchmark is available as `pnpm test:benchmark`; it runs two release-mode batches in temporary directories: 128 nested 4 KiB/64 KiB/512 KiB text fixtures, plus a 128-item mixed batch with 96 valid files, 16 unsupported files and 16 missing paths. Both report scan/clean throughput plus first-result and p95 latency, and assert that failures do not abort valid work or allocate output. Slow-disk and peak-memory results still require dedicated machine fixtures.
- Latest local Windows release-mode benchmark on 2026-09-13 produced: 128 files / 25,014,272 payload bytes; scan 136.62 files/s; clean 179.62 files/s; first cleanup result 2.58 ms; cleanup p95 11.53 ms. The mixed 128-item run (96 valid, 32 failed; 19,005,440 payload bytes) produced scan 140.86 files/s; clean 234.83 files/s; first result 2.63 ms; result p95 11.42 ms. These are workstation baselines, not hardware-independent service-level objectives and vary with local disk/cache load.
- Local non-publishing Windows `0.8.0` debug candidate evidence: NSIS installed/launched for six seconds/uninstalled; MSI installed from its product code, launched with `MetaClean` window title for six seconds, uninstalled, and left no registered product or installed executable; x64 portable ZIP unpacked/launched for six seconds/cleaned up. The MSI script is part of the release workflow; updater signatures and platform code signing remain separate gates.
- Windows preflight rerun of `scripts/preflight-windows.ps1` on 2026-09-23 against the `0.8.0` source candidate rebuilt the NSIS/MSI pair and x64 portable ZIP, repeated all three six-second launch/cleanup checks, and installed/validated/removed all 113 per-user Explorer context-menu commands with no residual MetaClean keys; the gate completed without publishing or pushing.
- Branded Windows installer candidate verification on 2026-09-23 rebuilt NSIS and MSI with deterministic 24-bit artwork. The generated NSIS source binds its welcome/finish rail, installer header, uninstall header and both icons; the generated WiX source binds its banner and dialog bitmap. NSIS install/launch/uninstall, x64 portable launch/cleanup and all 113 Explorer command round trips passed. The MSI administrative image extracted a 26,769,408-byte executable successfully; its destructive install/uninstall smoke intentionally stopped because a separate MetaClean 0.8.0 MSI is already registered at `C:\Program Files\MetaClean`, and the gate refuses to replace an existing user installation.
- The localized NSIS candidate was rebuilt and opened on Windows: Simplified Chinese welcome text remained readable with the branded rail, and the uninstall confirmation displayed the branded header without a language-selection prompt. A temporary silent install recorded language `2052`; its GUI and silent uninstall removed the temporary executable and uninstall registration. The repeatable six-second NSIS launch/uninstall smoke and all 40 release-contract tests passed.
- Current debug binary shell integration smoke: `--install-context-menu` created all 116 HKCU Explorer command keys from the native allowlist; `--remove-context-menu` removed all 116 and left zero `MetaClean` keys. This was a local HKCU round trip only and did not alter machine-wide classes.
- `scripts/preflight-windows.ps1` is the repeatable non-publishing Windows candidate gate: it builds the debug NSIS/MSI pair, packages x64 portable output, then invokes NSIS, MSI, portable and per-user Explorer context-menu smoke checks. It never pushes, tags or creates a release; MSI and context-menu smoke refuse to replace existing MetaClean state, and MSI preserves its logs when a validation step fails.

## v0.6.0 release candidate evidence

- Local gates pass for 186 frontend tests with every coverage dimension above
  80%, 100 Rust tests plus one external-fixture test ignored, strict Clippy,
  production and E2E builds, 9 real desktop E2E scenarios, 30 release tests,
  format/CSP/supply-chain checks and npm audit.
- Deep payload regressions cover embedded PDF JPEG metadata, base64 image data
  URIs, WAV C2PA, ID3-prefixed FLAC and the full contextual Unicode cleaner.
- Public cross-platform packages, updater metadata and checksums remain pending
  until the v0.6.0 tag workflow completes; they are not claimed by this local
  release-candidate section.

## v0.4.0 signed-updater release candidate evidence

- Native Tauri updater discovery replaces the production-blocked WebView fetch while the local-only CSP remains unchanged. The frontend has only `updater:allow-check`; download, verification, installation, tray shutdown and restart are owned by the Rust command.
- The release-only Tauri configuration produced a 4,086,597-byte Windows x64 NSIS installer and a non-empty 420-byte updater signature with the new encrypted key. The installer launched for six seconds and uninstalled cleanly; the 5,897,358-byte portable ZIP contained its runtime marker, launched for six seconds and was removed cleanly.
- Local v0.4.0 gates pass: 115 frontend tests with all coverage dimensions above 80%, 17 release-automation tests, 64 Rust tests (63 passed and one external Office sample test ignored), strict Clippy with warnings denied, production build, formatting/CSP/supply-chain checks, npm official audit with no known vulnerabilities, Cargo audit with the same 17 tracked upstream warnings, and 9 real desktop E2E scenarios.
- Five-platform updater packages, the public `latest.json`, and an end-to-end upgrade from an older installed build cannot be claimed until GitHub Actions secrets are saved and the v0.4.0 release matrix completes.

## Published v0.3.0 release evidence

- Public release: `https://github.com/Moresyl/metaclean/releases/tag/v0.3.0`, pointing to commit `654e1d8faed0e3c8a849ec8cc503b14db9786117`.
- Successful five-platform workflow: `https://github.com/Moresyl/metaclean/actions/runs/32048902129` (attempt 3).
- All ten named platform packages return HTTP 200. The published 954-byte `SHASUMS256.txt` contains exactly ten valid SHA-256 entries, one for every platform package.

## WPS interoperability regression — 2026-09-29

The fix described below shipped in v0.11.1 at
`b0a9bd5bda8b11921b1290069e1738ba6b416db1`.

- Candidate CI `36465554165` passed all quality gates and desktop tests on
  Windows, macOS and Linux. Release `36467341453` passed source validation,
  five platform builds, applicable installation/launch checks and finalization.
- The stable, non-draft release was published on 2026-09-28 at 19:05:21 UTC,
  with 20 nonempty assets. Independent verification `36469907196` downloaded
  the public assets, verified all 19 checksum-listed files and all five updater
  signatures against the tagged key, and rejected modified bytes for each.
- Pages deployment `36469791424` succeeded. Both public update feeds were read
  directly and match version 0.11.1 plus all five platform URLs and signatures.

- Registry discovery located WPS Office 2019 enhanced edition 11.8.6.11825 in
  a custom installation directory. The earlier claim that WPS was unavailable
  was based on incomplete executable discovery and is corrected here.
- Three original synthetic fixtures were generated using the installed WPS
  Writer, Spreadsheet and Presentation COM interfaces. No user documents or
  existing application sessions were used. WPS's Writer COM name reports
  `Microsoft Word`; this compatibility string is **not** evidence of a genuine
  Microsoft Word test.
- The published v0.11.0 cleaner removed XLSX comment XML but left its VML note
  shape. WPS reconstructed one empty comment when opening that output. The
  post-release fix removes only VML shapes whose Excel `ClientData` identifies
  a `Note`, retaining shared controls and ordinary drawings byte for byte.
- After the fix, all three cleaned samples opened, saved and reopened in WPS.
  DOCX retained its paragraph and 2-by-2 table with no comments; XLSX retained
  its label, `SUM(B1:B2)` formula and calculated value 20 with no comments;
  PPTX retained both slides and their text. SHA-256 comparisons confirm all
  original fixtures were unchanged. A separate synthetic ODT also passed the
  native external-fixture test; it was not tested in WPS.
- Nineteen Office unit tests and the full native suite (222 passed, four gated
  tests ignored) passed. Regression cases cover mixed note/control VML,
  namespace aliases, escaped attribute values, residual detection and malformed
  structures. This evidence concerns the post-v0.11.0 fix, not the already
  published v0.11.0 binary. These are semantic sample checks, not visual layout
  certification or proof of compatibility with every WPS version or document.

## Native batch memory observations — 2026-09-29

The v0.11.1 native release test executable was built before measurement, then
each existing benchmark was invoked in three separate processes with `--exact
--ignored --nocapture --test-threads=1`. The monitor read Windows
`Process.PeakWorkingSet64` and `PrivateMemorySize64` while each child was alive,
requesting 5 ms waits between samples. Each process produced 23–53 observations.
All six runs passed their success/failure-count and output-allocation assertions.

| Fixture batch | Clean throughput across three runs | Highest observed peak working set | Highest sampled private bytes |
| --- | --- | --- | --- |
| 128 valid files, 25,014,272 payload bytes | 211.27–218.45 files/s | 14.63 MiB | 9.93 MiB |
| 96 valid + 16 unsupported + 16 missing, 19,005,440 payload bytes | 273.77–279.47 results/s | 14.20 MiB | 9.71 MiB |

The host was Windows 11 x64 build 26200 with an Intel Core i7-12700. Working-set
figures are maxima of the OS lifetime-peak counter observed before process exit;
private-byte figures are sampled maxima. Requested polling intervals are not
guaranteed scheduling intervals, and the final unobserved interval may contain
a higher peak. These are observations, **not upper bounds**. The child includes
fixture setup, native scanning/cleanup and test teardown, but excludes Cargo,
the monitor and the desktop WebView. Source fixtures are freshly created local
temporary text files; this is not a cold-cache, slow-device, large-document or
full-application memory qualification. Those broader measurements remain open.

## Published Windows installer upgrade — 2026-09-29

[Verification run 36470788811](https://github.com/Moresyl/metaclean/actions/runs/36470788811)
passed on a disposable GitHub-hosted Windows runner. The workflow downloaded
the public x64 NSIS installers and checksum manifests, checked each installer
hash, then installed 0.11.0, upgraded in place to 0.11.1 and manually downgraded
in place to 0.11.0. Each step verified the executable version, single matching
uninstall registration and a live MetaClean window after six seconds. Final
uninstall removed the executable and registration. The downloadable
`windows-upgrade-evidence` artifact records all three observed versions.

A second [run 36471177273](https://github.com/Moresyl/metaclean/actions/runs/36471177273)
also passed a damaged-package scenario. Before the valid upgrade, the test
truncated a copy of the new installer to half its size and attempted a silent
installation. Rejection preserved the previous executable's SHA-256, its single
version-matching uninstall registration and its ability to launch for six
seconds. The subsequent valid upgrade, manual downgrade and uninstall all passed.
This establishes rejection of that truncated package before replacement; it does
not simulate process termination or power loss during installation.

[Matrix run 36471560641](https://github.com/Moresyl/metaclean/actions/runs/36471560641)
repeated the complete sequence independently for both x64 and x86 NSIS packages.
Both jobs passed; downloaded architecture-labelled evidence records the three
version/launch results and the truncated-installer rejection with the previous
executable hash unchanged. The x86 package ran under the hosted 64-bit Windows
compatibility environment; this is not a native 32-bit Windows OS qualification.

An experimental storage-preservation check in runs `36472009034` and
`36472271783` failed before writing synthetic data: neither architecture exposed
the requested WebView CDP endpoint (`fetch failed`). No storage-preservation
conclusion follows from those runs. The workflow's explicit `verify_storage`
option retains this strict experimental check; default installer evidence marks
`storageVerified` false when this optional check is disabled.

The endpoint issue was resolved using an app-specific HKLM WebView2 debugging
policy on the disposable runner, removed during cleanup. Recent WebView2 runtimes
ignore environment overrides in elevated hosts; see the
[Microsoft explanation](https://github.com/MicrosoftEdge/WebView2Feedback/issues/5645).
[Run 36472802532](https://github.com/Moresyl/metaclean/actions/runs/36472802532)
passed with `verify_storage=true` on both architectures. The real installed
0.11.0 application received synthetic localStorage values for locale, theme,
output mode and a history entry with source/output fingerprints. Exact values
survived the damaged-package rejection, upgrade to 0.11.1 and downgrade to 0.11.0;
the WebView DOM also confirmed that the dark theme was applied on every launch.
Both downloaded artifacts record `storageVerified: true` for all three versions.
This proves preservation of these synthetic values across this release pair,
not arbitrary historical schema migration or every user-data condition.

`scripts/verify-windows-upgrade.ps1` refuses non-hosted environments and existing
MetaClean installations/processes. The local preexisting 0.9.0 MSI installation
was not modified. This verifies x64/x86 NSIS installer transitions and launchability;
it does not yet verify application-triggered signed updates, arbitrary historical
user-data migration, interrupted installation recovery, MSI transitions or other platforms.

## Remaining external release gates

1. Test representative documents in genuine Microsoft Word and newer WPS builds, including layout, complex objects and supported OpenDocument interoperability. The limited WPS 2019 OOXML semantic round trip above and prior LibreOffice 26.2.5 samples do not close that broader qualification.
2. Provide Apple Developer signing/notarization credentials and verify both DMGs with Gatekeeper.
