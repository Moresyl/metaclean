# MetaClean capability audit

Audited on 2026-10-07. The public stable version is v0.12.4 at
`007cb6384f5c7d2d4181bb9ac89cf9a317daa705`. The table below records the
published baseline; the release evidence retains earlier qualification scopes.

This ledger describes implemented behavior, its evidence and its limits.
Extension counts and passing tests do not establish universal format support,
complete removal of every possible hidden payload or compatibility with every
application. Detailed run evidence is recorded in `VALIDATION.md`.

| Capability | Verified behavior and evidence | Limit |
| --- | --- | --- |
| Format intake | 116 extensions agree across the Rust engine, frontend, Windows shell, NSIS/MSI cleanup, both READMEs and support policy; `pnpm test:formats` verifies the manifests. | An accepted extension still requires a recognized, structurally valid container. |
| Safe output | Unique safe copies, backups before replacement, source snapshot rechecks and atomic writes are covered in `safe_io.rs` and engine tests. | Concurrent source changes, links and unsafe paths are refused. |
| Cleanup verification | Candidate bytes are re-detected and re-inspected before output allocation and writing. Engine regressions cover residual findings, format changes, truncation and corruption. | Verification covers implemented privacy surfaces; it is not a forensic proof of absence of all information. |
| Audio preservation | Native Opus/Vorbis comment cleanup preserves page layout, codec setup and audio bytes, while retaining validated numeric playback gains. Ten independent FFmpeg cases cover mono, stereo, 5.1, long comments and chained streams. | Unknown codecs, unsupported extensions and malformed structures fail closed; the cleaner does not fully decode audio. |
| Document privacy | Native Office/OpenDocument/EPUB and PDF cleaners have structural and residual-data tests; real Office samples were opened and exported with LibreOffice. Post-v0.11.0 WPS 2019 sample tests found and verified a VML comment-shape fix; see VALIDATION.md. | Microsoft Word, newer WPS releases and complex document fidelity remain unverified. The fix is not included in v0.11.0. Legacy binary Office is refused. |
| Office property fidelity | Package content types and relationships locate relocated, escaped and covered Strict property parts; retained payloads and exact surviving references are independently checked across 42 native copy/replacement cases. Seven corrupt-evidence tests qualify the verifier. | Thirty-two library round trips use direct readers; ten use explicitly recorded relationship views after matching direct-reader limitations. These are not Office application rendering checks. |
| PDF fidelity | Six synthetic copy/replace cases preserve rendered pages, searchable text, form fields and JPEG pixels while removing document metadata and private embedded JPEG EXIF. Validated JPEG orientation, density and ICC are retained. Independent MuPDF and Poppler rendering, pypdf parsing and Pillow checks pass. | These samples do not qualify arbitrary PDFs, complex forms, XFA, digital signatures or accessibility. |
| Image fidelity | Six independent HEIF/AVIF cases preserve frames, display pixels, alpha and ICC. Forty JPEG copy/replace cases remove previews and private APP0/APP2/APP14 payloads while preserving compressed scans, density, colors, ICC and pixels; validated MPF indexes and HDR XMP resource/display fields are rebuilt. Four separate actual HDR samples retain byte-identical complete decoded output. Eight PNG cases remove time and private/unknown ancillary payloads while preserving chunks, pixels, profiles, resolution, APNG and defined HDR/layout data; unknown critical chunks are refused. | Exact chunk retention does not qualify tone mapping in all viewers. The covered samples do not qualify all encoders, print drivers, gain maps, depth images, multi-picture layouts, animation or RAW. Unknown ancillary removal can discard editing information. |
| Batch isolation | Bounded intake, per-file outcomes, cancellation, count-only progress and source-path identity are tested across Rust and frontend boundaries. | Slow-device cancellation latency and general peak-memory bounds remain unqualified. |
| Content fingerprints | Successful results, details, history and JSON reports carry source/output SHA-256. Desktop tests independently hash the actual files. | Hashes describe cleanup-time content, exclude filesystem attributes and do not prove metadata removal. |
| Desktop workflow | Queue search/filtering, safe-copy cleanup, persistent preferences, keyboard navigation, RTL and accessibility checks run on Windows, macOS and Linux. | Automated scenarios do not replace a usability study with representative users. |
| Localization | All 32 published locale catalogs are checked for completeness; supported-scope counts are checked in every translated locale. | Catalog completeness does not establish independent linguistic review. |
| Release integrity | The five-platform release matrix runs applicable package smoke tests. Independent public downloads verify all 19 checksum-listed assets, five updater signatures and rejection of modified package bytes. | Updater signatures are separate from OS code signing; Apple signing/notarization remains unavailable. |

## Unreleased WebP qualification

Current source removes private application chunks at the WebP container and
animation-frame levels. Twenty-four independent local engine cases cover
lossy/lossless pixels, alpha, full/offset animation frames, copy/replacement and
both ICC settings. Actual compressed/control bytes, 36 decoded frames, profile
policy and source/backup hashes are checked; seven checker regressions reject
damaged evidence. The actual Windows application also clears all 23 previously
retained private blocks from the five original regression samples without
changing their media or preserved profiles. This is current-source evidence,
separate from the v0.12.4 published baseline above. Unknown-extension removal
may discard application editing information; the samples do not qualify every
encoder or viewer. Image and release source workflows now require the expanded
WebP gate.

Six actual Windows native rendered-font observations additionally identify
Noto Sans SC for measured Chinese text and Segoe UI family faces for Latin text.
They do not establish every node, other operating systems or the original
application's runtime typography. Detailed scopes remain in `VALIDATION.md`.

## Published JPEG qualification

Published source `007cb6384f5c7d2d4181bb9ac89cf9a317daa705` passes three-platform
desktop CI and independent image/PDF and release-source workflows. Forty JPEG copy/replacement
cases retain compressed scans, pixels and defined display fields while removing
private APP2/APP14 data. Validated MPF indexes clean every indexed JPEG and
rebuild lengths/offsets; defined HDR XMP fields and resource associations are
reconstructed without identity metadata. A separate local decoder verifies
byte-identical HDR output for two ISO and two legacy XMP samples. Malformed or
conflicting structures fail closed. This evidence does not qualify arbitrary
gain maps, depth images, encoders, viewers or multi-picture layouts. Version
metadata and refreshed captures pass release/documentation gates and 31 desktop
tests on each of three operating systems. Two native documentation scenarios
produce 17 visually reviewed assets displaying 0.12.4; three deployed capture
assets were independently downloaded and matched against committed bytes.

## Release evidence

- [v0.12.4 CI](https://github.com/Moresyl/metaclean/actions/runs/37512901175)
  passes main checks and all three desktop jobs after a targeted Linux retry
  for an APT repository HTTP 403. Windows Rust coverage is 92.74%; the release's
  Linux source coverage is separately 92.38%.
- [v0.12.4 release](https://github.com/Moresyl/metaclean/actions/runs/37518116349)
  passes source validation, five builds, package smoke checks and finalization.
- [v0.12.4 public assets](https://github.com/Moresyl/metaclean/actions/runs/37520670305)
  verifies 19 checksum-listed files and five signatures/tamper-rejection cases.
- [Published v0.12.4](https://github.com/Moresyl/metaclean/releases/tag/v0.12.4)
  has 20 assets. All six public verification workflows pass applicable
  0.12.3 → 0.12.4 → 0.12.3 transitions, including both Windows signed application
  updates and MSI repair. The independently hashed crash inventory preserves
  64 sources and two committed outputs with zero other fixture files and no
  automatic resume. The public portable executable matches the crash binary.
  Actual GitHub/Pages feeds, release notes, platform URLs and signatures agree.
  Both DMG jobs use arm64 runners, including the x86_64 package. Exact run IDs,
  warnings, original failures and qualification limits remain in `VALIDATION.md`.

Earlier v0.12.3 evidence:

- [Qualified CI](https://github.com/Moresyl/metaclean/actions/runs/37477283119)
  passed the main quality gates and 31 desktop cases on all three operating
  systems; a targeted second macOS round also passed at the same source.
- [Release pipeline](https://github.com/Moresyl/metaclean/actions/runs/37481870179)
  passed source validation, five platform builds, package checks and finalization.
- [Independent public-asset verification](https://github.com/Moresyl/metaclean/actions/runs/37484479678)
  checked downloaded bytes and signatures against the tagged public key.
- [Published v0.12.3](https://github.com/Moresyl/metaclean/releases/tag/v0.12.3)
  contains 20 assets, including the checksum manifest. The public Pages update
  feed was checked against the release's five platform URLs and signatures.

v0.12.3 updates ordinary tooltip geometry, typography, theme surfaces,
description association, cancellation and root-mounted positioning. Reduced
motion disables entrance animations so surfaces immediately reach their final
state. Native regression checks cover both themes, repeated mounts and four
entrance classes; actual normal-motion and reduced-motion browser keyboard
checks also pass. Documentation captures originally displayed v0.12.3; the
current candidate refresh displays v0.12.4 and is qualified separately above.
All six public qualification workflows passed `0.12.2 → 0.12.3 → 0.12.2`
where applicable, including both Windows signed application updates and MSI
repair. The independently hashed public crash inventory preserves 64 sources
and one complete output, with zero other fixture files and no automatic resume.
The public portable executable matches that crash application's version and
hash. Both DMG jobs used arm64 runners, including the x86_64 package. Actual
GitHub and Pages feed bytes match. Earlier tooltip failures, the successful
same-source macOS repeat and unresolved driver-warning causes remain recorded
in `VALIDATION.md`; passing runs are not a universal runtime guarantee.

v0.12.2 adds Office property identity/URI handling, PNG privacy classification,
updated desktop controls, native pickers and menu/command-panel containment.
All six public qualification workflows passed `0.12.1 → 0.12.2 → 0.12.1`
where applicable, including both Windows signed application updates and MSI
repair. The public crash sample preserves 64 sources and two complete outputs,
with zero other fixture files and no automatic resume. Actual public feed bytes
match GitHub and Pages. Native documentation captures for that release displayed
v0.12.2 before the current refresh.
Both DMG package checks used arm64 runners, including the x86_64 package.
Its later documentation-only CI exposed a macOS context-menu display failure.
Diagnostic qualification and a targeted repeat subsequently passed, but the
original cause remains unconfirmed. Exact public run evidence and the subsequent
v0.12.3 qualification are recorded separately in `VALIDATION.md`.

v0.12.1 prevents About content and actions from overflowing narrow containers.
Its final CI passed nineteen desktop cases on each of three operating systems,
including the English/Chinese 283px containment regression. All six public
qualification workflows passed 0.12.0 → 0.12.1 → 0.12.0 transitions where applicable,
including real Windows signed update/restart and MSI repair. The public crash
sample preserved sixty-four sources and seven committed outputs, with no
additional fixture files or automatic resume. Current native documentation
captures for that release display v0.12.1. Exact run IDs, records and limits are in `VALIDATION.md`.

v0.12.0 refreshes the shared desktop design rules, compact layouts and full-window
dialogs, and pins manual publication to an immutable source. Its final CI passed
eighteen desktop cases on each of three operating systems. Public DEB and both
DMG upgrade/rollback checks passed; both Windows NSIS architectures passed real
application update/restart, storage preservation and damaged-installer rejection.
Public MSI install/upgrade/repair/rollback/removal also passed. The public crash sample preserved sixty-four
sources and two committed outputs, with no additional fixture files or automatic
resume on either restart. See `VALIDATION.md` for the exact runs and limits.

v0.11.10 adds JPEG APP0 preview/private-data cleanup with independent compressed
scan and pixel checks. Public 0.11.9 → 0.11.10 → 0.11.9 NSIS/MSI/DEB/DMG
transitions passed, including MSI repair and both Windows application-triggered
updates. Its crash sample preserved sixty-four sources and two committed outputs,
with no additional files in that inventory. `VALIDATION.md` links the actual runs
and documents their limits.

The previous v0.11.9 added PNG `tIME` cleanup with independent pixel/chunk checks. Its public
0.11.8 → 0.11.9 → 0.11.8 NSIS/MSI/DEB/DMG transitions passed, including MSI
repair and both Windows application-triggered updates. The first NSIS attempt
failed while seeding the old version's synthetic state; a diagnostic-only harness
change preceded two successful full reruns, and the original cause is unconfirmed.
Its crash sample preserved sixty-four sources and one committed output, with no
additional files in that inventory. These results do not establish arbitrary
migration compatibility or orphan-free recovery. `VALIDATION.md` links each run.

The VML note-shape fix verified with WPS 2019 is included in v0.11.1.

v0.11.7 revalidates source bytes with a 64 KiB buffer while retaining exact-length,
link and metadata checks. Twelve isolated Windows memory measurements compared
the previous whole-buffer algorithm with the new guard; observed peaks for a
256 MiB source fell from 516.66–516.67 MiB to about 260.75 MiB. This measures
revalidation, including fixture allocation, and does not establish whole-app
peak memory. The benchmark method and all sizes are recorded in `VALIDATION.md`.
Its public 0.11.6 → 0.11.7 → 0.11.6 NSIS/MSI/DEB/DMG checks passed, including
both Windows application-triggered updates and MSI repair. The public crash
check preserved sixty-four sources and one committed output, with no additional
files in its complete inventory. This single interruption point does not qualify
all temporary-file cleanup or physical power-loss behavior.

v0.11.6 adds exact JPEG print-density retention and classifies retained
orientation as informational. Its public NSIS/MSI/DEB/DMG transitions from
0.11.5 and back passed, including both Windows application-triggered updates.
The crash check preserved sources and committed output, but recorded one
additional file without identifying it; orphaned-temporary-file cleanup remains
unqualified. Detailed evidence and hosted-machine limits are in `VALIDATION.md`.

v0.11.5 fixes merged PDF widget field-name removal and supports bounded
ASCII85-wrapped JPEG metadata cleanup without pixel re-encoding. Its release
source gate includes all six independent PDF fidelity cases. Development-only
undici dependencies were patched separately from the application runtime.
Its public crash regression and 0.11.4 → 0.11.5 → 0.11.4 NSIS/MSI/DEB/DMG
transition checks passed within the recorded hosted-machine and synthetic-state
limits, including application-triggered Windows updates in both architectures.

v0.11.4 updates the yanked indirect `chacha20` release to 0.10.2. Seven remaining
Rust warnings are classified in `VALIDATION.md`. Its public crash regression and
the 0.11.3 → 0.11.4 → 0.11.3 NSIS/MSI/DEB/DMG transition checks all passed within
the recorded hosted-machine and synthetic-state limits.

The v0.11.2 text cleaner preserves contextual Unicode behavior while avoiding a
complete character array. Three isolated synthetic 64/256 MiB runs observed about
43% lower peak working set, with source/output byte and hash checks passing.
Detailed measurements and workload limits are recorded in `VALIDATION.md`; this
does not establish a universal memory or throughput improvement across formats.

Six isolated native benchmark processes also passed on Windows 11; their small
text batches observed up to 14.63 MiB peak working set. The sampling method and
scope are recorded in `VALIDATION.md`. This does not establish a memory upper
bound or cover the desktop WebView, large documents or slow storage.

Public v0.11.8 desktop measurements in
[run 36528753753](https://github.com/Moresyl/metaclean/actions/runs/36528753753)
passed three 64 MiB and three 256 MiB text cleanups with exact source/output/audit
hash checks. Aggregate private-memory maxima were 272.02–273.76 MiB and
654.48–659.94 MiB; working sets were 504.95–506.64 MiB and 889.04–892.42 MiB.
All peak inventories contained seven application/WebView processes. Public
upgrade and rollback checks passed for NSIS, MSI, DEB and both DMGs. The public
crash run preserved all sources but left one cleaned temporary file after forced
termination; this is not an orphan-free guarantee. See `VALIDATION.md` for links,
sampled-process limitations and exact scope.

Earlier public v0.11.7 desktop measurements in
[run 36523943242](https://github.com/Moresyl/metaclean/actions/runs/36523943242)
validate process creation times and include the application and WebView tree.
Three 64 MiB and three 256 MiB text cleanups passed source/output and audit-hash
checks. Observed aggregate working-set maxima ranged from 501.81–507.93 MiB
and 953.24–1,145.31 MiB respectively; all peak inventories contained seven
application/WebView processes. Earlier PID-only measurements cannot establish
process ownership retrospectively. Shared pages may be counted more than once;
sampling, host differences, rejected observations and limits are in `VALIDATION.md`.

Public Windows x64 and x86 NSIS installers passed isolated 0.11.0 → 0.11.1 → 0.11.0
install/upgrade/manual-downgrade sequences, version/launch checks, truncated-package
rejection with the old executable intact, and final uninstall in
[run 36471560641](https://github.com/Moresyl/metaclean/actions/runs/36471560641).
The x86 package was tested on a 64-bit Windows host, not a native 32-bit OS.
An additional [run 36472802532](https://github.com/Moresyl/metaclean/actions/runs/36472802532)
verified preservation of synthetic preferences and history/fingerprints through
the same sequence in both installed WebViews, including applied dark theme.
[Run 36473320724](https://github.com/Moresyl/metaclean/actions/runs/36473320724)
also passed the real application's update button, public signed download,
installation, automatic restart and storage checks for both Windows architectures.
[Run 36482289761](https://github.com/Moresyl/metaclean/actions/runs/36482289761)
repeated the real application-triggered update from 0.11.1 to 0.11.2 on x64 and
x86, including preserved synthetic data, damaged-installer rejection, manual
downgrade and uninstall.
Arbitrary historical schema migrations and interrupted-update recovery remain
outside this evidence. Public x64 MSI packages separately passed installation,
upgrade, manual downgrade, registration/version/launch checks and removal in
[run 36475350834](https://github.com/Moresyl/metaclean/actions/runs/36475350834).
[Run 36475617820](https://github.com/Moresyl/metaclean/actions/runs/36475617820)
also damaged the installed executable and verified that MSI repair restored its
exact SHA-256, registration and launchability before downgrade and removal.

Public Linux DEBs also passed package-manager installation, upgrade, manual
downgrade, launch checks and removal on Ubuntu 22.04 amd64 in
[run 36473955463](https://github.com/Moresyl/metaclean/actions/runs/36473955463).
Public macOS ARM and Intel DMGs passed isolated manual bundle replacement,
rollback, identity/architecture checks and launch checks in
[run 36474805222](https://github.com/Moresyl/metaclean/actions/runs/36474805222).
Both macOS jobs used ARM hosts; Intel execution used compatibility support.
These checks do not establish Linux/macOS application-triggered updates,
data preservation or Apple Gatekeeper acceptance.

The 0.11.1/0.11.2 release pair subsequently passed the same
[MSI transitions and repair](https://github.com/Moresyl/metaclean/actions/runs/36482694133),
[DEB transitions](https://github.com/Moresyl/metaclean/actions/runs/36482701851) and
[ARM/Intel DMG replacement](https://github.com/Moresyl/metaclean/actions/runs/36482709218)
checks, with the same scope limitations.

## Product boundaries and remaining qualification

Forced termination of the public v0.11.2 application reproduced a missing
recovery notice in two hosted runs, despite intact source files and committed
outputs. The WebView-only marker did not provide the expected restart prompt;
see the failed-run evidence in `VALIDATION.md`. The v0.11.3 candidate's native
path-free record passed the same real interruption and two-restart scenario in
[run 36486936517](https://github.com/Moresyl/metaclean/actions/runs/36486936517),
with all source and committed-output hashes preserved. The public v0.11.3
portable package passed the same scenario in
[run 36490451100](https://github.com/Moresyl/metaclean/actions/runs/36490451100).
Its 20 published assets, five signed updates and matching release/Pages feeds
were independently checked; Windows x64/x86 real application updates from
0.11.2 also passed. MSI (including executable repair), DEB and both DMG
architectures passed the 0.11.2 → 0.11.3 → 0.11.2 transition scenarios.
See the run evidence and host limitations in `VALIDATION.md`. These checks do not
qualify power loss or installer interruption.

Inspection exposes categories and counts, not raw identity, location or comment
values. Raw-value forensic inspection, statistical text rewriting, pixel-domain
watermark removal, generic archive rewriting and unknown binary formats are
outside the current contract in `SUPPORT_POLICY.md`.

Microsoft Word/newer-WPS interoperability, Apple signing/notarization, complete installed
application update/rollback recovery, slow-device behavior and full-application
memory qualification beyond the measured Windows text scenarios remain open. They must not be described as completed merely because
unit tests or release packaging passed. The roadmap in `docs/PLAN.md` tracks these
items separately from the completed release.
