# MetaClean capability audit

Audited on 2026-09-29 for the published v0.11.4 release at
`5748de3e7eeb8a876a934d0bd7b928361bb33c5d`.

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
| Batch isolation | Bounded intake, per-file outcomes, cancellation, count-only progress and source-path identity are tested across Rust and frontend boundaries. | Slow-device cancellation latency and peak-memory qualification still need dedicated evidence. |
| Content fingerprints | Successful results, details, history and JSON reports carry source/output SHA-256. Desktop tests independently hash the actual files. | Hashes describe cleanup-time content, exclude filesystem attributes and do not prove metadata removal. |
| Desktop workflow | Queue search/filtering, safe-copy cleanup, persistent preferences, keyboard navigation, RTL and accessibility checks run on Windows, macOS and Linux. | Automated scenarios do not replace a usability study with representative users. |
| Localization | All 32 published locale catalogs are checked for completeness; supported-scope counts are checked in every translated locale. | Catalog completeness does not establish independent linguistic review. |
| Release integrity | The five-platform release matrix runs applicable package smoke tests. Independent public downloads verify all 19 checksum-listed assets, five updater signatures and rejection of modified package bytes. | Updater signatures are separate from OS code signing; Apple signing/notarization remains unavailable. |

## Release evidence

- [Candidate CI](https://github.com/Moresyl/metaclean/actions/runs/36494120243)
  passed the main quality gates and desktop tests on all three operating systems.
- [Release pipeline](https://github.com/Moresyl/metaclean/actions/runs/36495605281)
  passed source validation, five platform builds, package checks and finalization.
- [Independent public-asset verification](https://github.com/Moresyl/metaclean/actions/runs/36497748414)
  checked downloaded bytes and signatures against the tagged public key.
- [Published v0.11.4](https://github.com/Moresyl/metaclean/releases/tag/v0.11.4)
  contains 20 assets, including the checksum manifest. The public Pages update
  feed was checked against the release's five platform URLs and signatures.

The VML note-shape fix verified with WPS 2019 is included in v0.11.1.

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

Separate public v0.11.3 desktop measurements in
[run 36491998506](https://github.com/Moresyl/metaclean/actions/runs/36491998506)
include the application and WebView process tree. Three 64 MiB and three 256 MiB
text cleanups passed complete source/output and audit-hash checks. Observed
aggregate working-set maxima ranged from 503.79–571.55 MiB and
887.26–1,243.54 MiB respectively. Shared pages can be counted more than once;
sampling, host differences and timing limits are detailed in `VALIDATION.md`.

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
