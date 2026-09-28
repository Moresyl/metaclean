# MetaClean capability audit

Audited on 2026-09-29 for the published v0.11.1 release at
`b0a9bd5bda8b11921b1290069e1738ba6b416db1`.

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

- [Candidate CI](https://github.com/Moresyl/metaclean/actions/runs/36465554165)
  passed the main quality gates and desktop tests on all three operating systems.
- [Release pipeline](https://github.com/Moresyl/metaclean/actions/runs/36467341453)
  passed source validation, five platform builds, package checks and finalization.
- [Independent public-asset verification](https://github.com/Moresyl/metaclean/actions/runs/36469907196)
  checked downloaded bytes and signatures against the tagged public key.
- [Published v0.11.1](https://github.com/Moresyl/metaclean/releases/tag/v0.11.1)
  contains 20 assets, including the checksum manifest. The public Pages update
  feed was checked against the release's five platform URLs and signatures.

The VML note-shape fix verified with WPS 2019 is included in v0.11.1.

Six isolated native benchmark processes also passed on Windows 11; their small
text batches observed up to 14.63 MiB peak working set. The sampling method and
scope are recorded in `VALIDATION.md`. This does not establish a memory upper
bound or cover the desktop WebView, large documents or slow storage.

## Product boundaries and remaining qualification

Inspection exposes categories and counts, not raw identity, location or comment
values. Raw-value forensic inspection, statistical text rewriting, pixel-domain
watermark removal, generic archive rewriting and unknown binary formats are
outside the current contract in `SUPPORT_POLICY.md`.

Microsoft Word/newer-WPS interoperability, Apple signing/notarization, complete installed
application update/rollback recovery, slow-device behavior and peak-memory
measurements remain open. They must not be described as completed merely because
unit tests or release packaging passed. The roadmap in `docs/PLAN.md` tracks these
items separately from the completed release.
