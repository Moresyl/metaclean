# MetaClean validation status

Last audited: 2026-10-06

This file records evidence, not intent. A row is complete only when the named artifact or runtime check exists.

## Unreleased desktop scope and PNG rejection — 2026-10-06

The current startup selects the desktop window scope. Rechecking the source CSS
in the native renderer exposes a scope error in earlier fixtures: their generic
main scope uses 12px small type, while the actual desktop scope uses 13px. The
source also declares 430-weight UI body text and a default arrow cursor for
actions. Loading the select component's separate stylesheet confirms that selects
and small/large actions also resolve their control type to 13px. The product now
uses those independently verified desktop declarations. Earlier comparison rows
below remain historical evidence for the explicitly tested scope and source
revision, and do not qualify the corrected desktop scope.

The prior committed batch at 5fba504 has now passed exact-source CI, including
eleven design and sixteen workflow cases on Windows, Linux and macOS. Actual
logs also confirm ten identical FFmpeg decoded outputs and the macOS extended
attribute test. The observer ended on a TLS retrieval failure; the completed
run metadata and full final logs establish success.

The new menu unit cases first fail on the previous implementation, then pass
after focus return, Tab dismissal, Home/End, disabled-entry guards and unique
active-descendant IDs are added. The embedded driver's key mapping omits
Home/End, so its original key-input failure does not prove a product defect.
After delivering the correct keys to the renderer, the previous ordinary native
binary still fails the End boundary regression, and the corrected product passes
all eleven design and sixteen workflow cases. This renderer dispatch is synthetic.
Separate native CDP qualification passes twelve light/dark and LTR/RTL cases:
Home/End reach enabled boundaries, arrows wrap, and Escape/Tab/Shift+Tab dismiss
without moving workspace scroll or losing the preceding search focus. All sixty
recorded keydown events have isTrusted=true. Menu opening in that fixture remains
synthetic; the evidence qualifies the actual product key handlers, not the source
application runtime. The updated frontend passes 448 tests and all
coverage thresholds, with 93.88% line coverage.

PNG parsing now refuses unrecognized critical chunks at inspection, cleanup
and residual-verification boundaries. The old implementation fails the new
regression. Tests cover public/private chunk names before/after image data and
both profile choices. All 248 Windows native tests pass, strict Clippy passes,
and eight independently generated Pillow cases retain exact image chunks,
decoded pixels, profiles, resolution, animation, source/backup bytes and file
modification times while removing the embedded timestamp.

Corrected source CSS comparisons pass twelve segment states, forty-eight action
states, two selects and both shared-menu themes in the native renderer. The
select comparison includes its separate component stylesheet, and action sizes
resolve to 13/14/13px in the actual desktop scope. Shared-menu evidence does not
establish the source context-menu-specific call chain. Oklab comparisons retain
the existing 0.0001 component bound and exact alpha; geometry is exact.
The ordinary desktop build and all twenty-seven native cases pass. Trusted-key
qualification also passes. The ordinary configuration was rebuilt after the
private CDP check, with no research CSS or private debug argument in the binary.
Production frontend build, local-only CSP checks, supply-chain verification,
release tests, format manifests, documentation checks/build and the official
npm-registry audit pass. Exact-source remote qualification for this new batch
and the broader release work remain pending. These changes are unreleased.

## Unreleased settings segments and context-menu placement — 2026-10-06

Settings categories and theme choices now share an independent segmented
control: 32px track, 28px options, 2px padding/gap, ordinary corners and a
selected thumb that follows measured option bounds. Current default rules
were checked separately from the action pill configuration. All twelve
rendered light/dark comparisons pass for selected/unselected, disabled
selected/unselected and focused selected/unselected states. The comparison
uses same-origin source CSS only in an ignored research build under the
unchanged production CSP; it does not execute source JavaScript or establish
the original application's runtime behavior. The paired screenshots were
inspected. Separate product WebView checks use actual CDP pointer and keyboard
input in both themes: hover emphasizes ink and the feedback surface, pressing
scales only the feedback, releasing changes the choice, an arrow moves focus
without changing selection, and Enter selects the focused option. Recorded
pointer, click and keyboard events all have isTrusted=true. This uses an ignored
debug configuration with a separate data directory, not a production setting.

The previous native binary fails the new segment-height assertion at 36px
instead of 32px. Subsequent comparisons expose inactive ink and focused-ink
differences; final colors use 65% main ink in sRGB and emphasize inactive
keyboard focus. Native checks cover settled focus, track/thumb geometry,
category changes, container resizing and narrow RTL selection.

A real queue menu audit exposes a shifted, clipped overlay inside its animated
workspace, with horizontal workspace scrolling. Root mounting and focus with
preventScroll correct the containing block. Corner regressions additionally
expose missing leading/trailing margins; final positioning uses unscaled,
fractional layout dimensions and clamps both axes without weakening the 8px
edge assertions. The final native audit measures an exact 1180x720 root
overlay and a fully contained menu; the corrected screenshot was inspected.
The tracked test checks both corners in LTR/RTL, menu focus, Escape dismissal
and unchanged workspace scroll.

The complete local frontend suite passes 441 tests with 93.79% line coverage
and all configured coverage thresholds. The normal Windows E2E build passes
27 desktop cases: eleven design regressions and sixteen native workflow cases.
Private research builds, failed diagnostics and pending checks are separate
from those ordinary product results. The ordinary native configuration is
restored, its binary contains no private debug argument, and the final
production build passes. All 44 release tests and documentation/format/CSP
checks also pass. These changes are unreleased; exact-source CI for this batch
remains pending.

## Unreleased queue-search alignment — 2026-10-06

Current input component defaults and its child rules were read independently
from select/action rules. The queue uses its soft variant with ordinary corners,
32px height, 12px gutters, 400-weight 12px text and 18px line height. It retains
the native search input and its existing filtering and Escape handler.

The final rendered CSS comparison passes all fourteen light/dark combinations:
normal, focus, read-only focus, disabled, invalid, invalid focus and disabled
plus invalid. It compares container/input geometry and typography, resolved
surface/border/text colors, opacity, cursor and gutters. Source focus attributes
and paired CSS focus pseudo-states explicitly exercise the relevant selectors;
this is not original-application runtime evidence. Real product-browser input
events filter the synthetic queue to zero matches; a trusted Escape key restores
the empty query and one queued file. RTL gutters/adornment offset, forced-color
boundaries/focus and reduced-motion duration also pass.

The preceding native binary fails the new corner regression at 7.5px versus
10px. An initial updated run exposes `outline: none` retaining a 3px computed
width where the input rule specifies `outline: 0`; the implementation now uses
the exact zero-width declaration. Paired focus-state comparison also reveals
that disabled controls need an explicit focus guard. Both corrections retain
the original assertions, and failed logs remain separate from passing evidence.
The complete local frontend suite passes 432 tests with 93.67% line coverage. The
final Windows build passes all 25 desktop cases: nine design regressions and
sixteen native workflow cases. This includes the corrected zero input outline,
input theme/state geometry and About layout checks through the real locale
control. Production build, all 44 release tests and documentation/format/CSP
checks also pass. [CI 37403595467](https://github.com/Moresyl/metaclean/actions/runs/37403595467)
qualifies source `87f6de2` with all four jobs successful: 432 frontend tests,
93.58% frontend and 91.82% Rust line coverage, a clean npm audit, and all
25 desktop cases on each of Windows, Linux and macOS. The actual logs include
both English and Chinese About layout phases on all three platforms.
The browser page review passes all 80 English/Chinese, light/dark, wide/narrow,
empty/populated and five-page cases, including search/select bounds; narrow
and wide queue screenshots were inspected. These changes are unreleased.

## Unreleased native select alignment — 2026-10-06

The current select component's helper configuration was read separately from
the shared action configuration. Its default is outline/md with ordinary
corners, not a pill. Earlier comparison fixtures exercise the optional pill
rules and are retained as diagnostics, not as default-configuration evidence.
The final fixture preserves the original stylesheet layer order and current
main-window scope, and renders the product's actual React select. No extracted
JavaScript is executed and no reference code or assets enter the product.

All sixteen default-configuration light/dark comparisons pass: normal, hover,
pressed, hover plus pressed, focus-visible, disabled, disabled plus hover/pressed
and invalid. Measured properties include height, radius/corner shape, leading
gutter, type, resolved text and inset-border colors, background, cursor, opacity,
indicator dimensions/color/opacity and focus width/color/offset. This is CSS
rendering evidence, not the original installed application's runtime.

Additional product-browser checks use real CDP pointer and keyboard events:
hover feedback appears, ArrowDown changes the native selection and React
receives a trusted change event, and FormData contains the selected value.
RTL indicator placement, forced-color boundaries/focus and reduced-motion
duration also pass. Frontend tests additionally cover forwarded focus, controlled
values, required validity, grouped options and disabled form exclusion.

The native regression fails on the preceding binary's 14px label. The final
Windows build passes all 24 desktop cases, including eight design regressions
and sixteen workflow cases. Both ordinary and invalid focus rings are checked
in both themes. The complete frontend suite passes 432 tests with 93.58% line
coverage. The final default-layout browser review passes all 80 English/Chinese,
light/dark, wide/narrow, empty/populated and five-page cases, including select
bounds; representative narrow screenshots were inspected. Production build,
44 release tests and documentation/format/CSP checks pass. The subsequent
`87f6de2` CI above also passes the select regression on all three platforms;
these changes are unreleased.

[CI 37401489467](https://github.com/Moresyl/metaclean/actions/runs/37401489467)
passes the main checks and the complete Windows/Linux desktop suites. The native
select regression also passes on macOS, but that job fails when the About
dual-language layout test encounters repeated direct script-execution timeouts.
No About geometry assertion fails in that run; the complete CI is still a
failure. The follow-up switches locales through the actual settings select,
waits for the document language using the published locale-to-HTML mapping,
and restores the prior interface/storage state
without reloading between languages. Layout assertions and timeout budgets are
unchanged; phase messages identify language-selection and layout-verification
steps. A first local synchronization check incorrectly expected `zh` instead
of its published `zh-CN` HTML language; that failed run is retained and the
helper now reads the mapping from the source locale definitions. The complete
25-case Windows follow-up passes both languages. Its macOS qualification remains
pending. The documentation run succeeds.

## Unreleased development dependency security — 2026-10-06

[CI 37398711713](https://github.com/Moresyl/metaclean/actions/runs/37398711713)
and [CI 37399120400](https://github.com/Moresyl/metaclean/actions/runs/37399120400)
fail at the dependency-audit gate, before the desktop matrix. They do not
qualify the workspace fix or the macOS action-focus synchronization.

The upstream [source-map-js 1.2.2 release](https://github.com/7rulnik/source-map-js/releases/tag/v1.2.2)
fixes GHSA-68fv-2mgg-jv7q, and the upstream
[smol-toml advisory](https://github.com/squirrelchat/smol-toml/security/advisories/GHSA-r4xh-jqrq-34v2)
identifies 1.9.0 as its patched version. Both are development-toolchain
dependencies. Overrides and the frozen lockfile now resolve those versions;
the lockfile diff changes only the two packages and their consumers.

After a frozen install, the official npm registry audit returns zero advisories
at every severity and no muted advisories. The configured mirror does not
provide the audit endpoint; that initial endpoint error is retained separately
and is not treated as a clean audit. Post-update verification passes all 430
frontend tests with 93.57% line coverage, all 23 Windows desktop cases, all 44
release tests, documentation/format/CSP/archive-extractor checks, and production
and documentation builds. No audit exceptions were added.
[CI 37399834162](https://github.com/Moresyl/metaclean/actions/runs/37399834162)
passes all four jobs on source `e549329e949227246ccbec45d5117968d92c60d5`,
including the clean dependency audit, all 430 frontend tests and all 23 desktop
cases on each of Windows, Linux and macOS. Its documentation pipeline also
passes. This run qualifies the preceding workspace and action-focus fixes.

## Unreleased workspace containment — 2026-10-06

Browser measurements at 390px show empty-intake children extending 59px above
and below a 212px card. A populated queue also clips its heading, clear action
and most of the file-list region. A native narrow-content regression fails on
the preceding binary because intake and options remain in two columns.

The workspace now responds to its named content container rather than only
the window width. Stacked rows retain their content height, queue toolbar groups
wrap, and a 260–360px queue bounds the independently scrolling file list. The
fixed native window, cleanup behavior and file-processing limits are unchanged.

Browser checks cover English/Chinese, light/dark, 1180/390px, all five pages and
empty/populated queue states: 80 cases, with no horizontal action/content overflow,
intake-child spill or overlap with the options region. The queue fixture is an
in-memory browser File; no user document is read or cleaned. Narrow screenshots
were inspected directly for complete intake details, toolbar actions and the
first queued file. The complete Windows desktop suite passes all 23 cases,
including seven design regressions and sixteen workflow cases. Narrow native
fixtures constrain the real content region without resizing the fixed window;
the populated fixture uses the same in-memory browser selection path.

The complete frontend suite passes 430 tests with 93.57% line coverage using
one forked worker. A preceding threaded run fails to start four workers and
does not qualify the complete suite; its logs are retained. Documentation,
116-extension and CSP checks and 44 release-automation tests also pass.

## Unreleased action state alignment — 2026-10-06

Current component defaults and their merged configuration were inspected before
constructing the CSS comparison: pill enabled, squircle disabled for pills, and
pressable scaling disabled. The fixture retains the original stylesheet layer
order and the label/icon structure implied by the inspected component. No
extracted JavaScript is executed. The product fixture renders its actual React
action components through the development server.

All 192 light/dark combinations pass for four variants, three heights and eight
states: normal, hover, active, hover plus active, focus, expanded, disabled and
disabled plus hover/active. Compared properties include typography, geometry,
icon dimensions/offsets, resolved surface/text colors, opacity, cursor, transform,
easing and focus color/width/offset. These are rendered CSS pseudo-state checks,
not evidence of the original installed application's runtime or trusted pointer
interaction.

The native regression fails on the preceding binary's 500-weight label where
the current rule requires 400. Initial updated runs expose test precision issues
after color transitions: translucent Oklab canvas conversions differ by two
8-bit channels, and completed transitions retain coefficients differing by up
to 0.000046 from the directly resolved declaration. The regression now compares
resolved Oklab components within 0.0001 while requiring exact alpha; geometry,
opaque colors and semantics retain exact assertions. All five native design
cases pass after this correction. The complete Windows desktop suite passes all
21 cases: five design regressions and sixteen native workflow cases.
Subsequent cross-platform qualification is recorded below.

[CI 37397170260](https://github.com/Moresyl/metaclean/actions/runs/37397170260)
passes the main checks and Windows/Linux desktop cases, but macOS fails when
the action focus test reads a 3px pseudo-element outline immediately after
focusing; the expected ring is 2px. The follow-up separates focus activation
from inspection and waits for the actual 2px focus-visible ring before retaining
the exact width, offset and color assertions. The Windows design suite passes
all seven cases with this synchronization. Subsequent
[CI 37399834162](https://github.com/Moresyl/metaclean/actions/runs/37399834162)
passes all seven design and sixteen workflow cases on each of Windows, Linux
and macOS, including this exact focus assertion. The initial failed run remains
recorded rather than being represented as a cross-platform pass.

Frontend verification passes all 430 tests with 93.57% line coverage, including
native form submission/default type, disabled activation, forwarded refs and
accessible icon-action states. Documentation/116-extension/CSP checks and all
44 release-automation cases pass. Twenty browser page/theme/width combinations
show no horizontal content/action overflow. Visual inspection and measured child
bounds additionally reveal vertical intake-card spill at 390px in both themes;
the subsequent containment work above corrects this measured layout issue.
These action changes are unreleased.

## Unreleased checkbox state alignment — 2026-10-06

A direct browser comparison loads the supplied current theme and checkbox CSS
in an isolated frame, retaining the layer order from its original HTML entry.
The live application's stylesheet renders a native checkbox fixture cloned
from its actual control. This exercises extracted CSS, not the original
installed application's runtime or native file-processing IPC. Browser
pseudo-states are selected through CDP for computed-style comparison;
these results do not establish trusted pointer interaction.

The preceding implementation differs in unchecked borders, disabled fills,
focus-ring colors and a fixed 4px radius. The current rules use dedicated
state tokens, preserve selected colors on hover and scale the 4px base corner
to 5px where the renderer supports the shared corner-scaling rule. All eighteen
compared light/dark state combinations match border/fill colors, dimensions
and radius; focus cases also match outline width, offset and color.

A native regression fails on the preceding binary at the light unchecked
border. The first updated desktop run passes the original nineteen cases but
the new test fails when its synthetic pointer move does not activate CSS hover.
The embedded driver's inspected implementation dispatches a JavaScript
`MouseEvent`, so native hover coverage cannot be claimed from that action.
Hover remains covered by the direct browser comparison. The native regression
checks the six selection/disabled combinations and focus in both themes.
The corrected Windows run passed all twenty desktop cases: four design
regressions and sixteen workflow cases. Frontend verification passed all
428 tests with 93.58% line coverage; documentation/116-extension/CSP checks,
44 release-automation cases and the documentation build also passed.
[CI 37394121429](https://github.com/Moresyl/metaclean/actions/runs/37394121429)
completed successfully at source `212e558`: all three operating systems passed
four design and sixteen workflow cases, with 428 frontend tests, 93.58% frontend
line coverage and 91.82% Rust line coverage. These changes are not yet included
in published v0.12.1 packages.

## Published v0.12.1 About layout correction — 2026-10-06

The published [v0.12.1](https://github.com/Moresyl/metaclean/releases/tag/v0.12.1)
uses immutable source `5c5dfe67befa88a40763c79e1458f3b09d313ee5`.
It was published at 2026-10-05 23:18:44 UTC with twenty nonempty assets and is
the latest stable release. All five package builds and all six public
qualification workflows passed at this source.

Local v0.12.1 qualification passed 428 frontend tests with 93.58% line coverage,
247 Windows native tests (ten ignored), strict all-target Clippy, formatting,
all nineteen native desktop cases, 44 release automation tests and the
documentation/116-extension gates. The documentation build passed. The desktop
launcher emitted an automatic Edge-driver download warning; both spec files
nevertheless completed all nineteen cases with exit status 0. This does not
establish that the automatic driver download succeeded.

Additional browser review covered five pages and all four settings categories
in English/Chinese, light/dark themes and five viewports
(1180×720, 960×720, 768×540, 590×360 and 390×720). The initial outer-container
checks passed, but screenshot review exposed an inner About scrollbar. A
nested-scroll-region assertion reproduced 317px of content in a 283px region
at the English 390px viewport; this initial pass did not prove inner containment.

The page now responds to its content container, placing update actions and
community links below their preceding content when narrow, with wrapping
diagnostics and footer actions. All 160 browser states passed the expanded
checks for typography, heading weight, control dimensions, accessible names
and outer/inner horizontal containment. Screenshots were visually inspected.
These browser checks do not exercise native IPC or cleaning.

The new desktop regression exercises a 283px About region without changing the
fixed native window. It fails on the preceding local binary with 317px content
against 272px available after the vertical scrollbar. The rebuilt Windows
application passed all nineteen desktop cases, including this English/Chinese
containment check. Frontend verification passed 428 tests with 93.58% line
coverage; production build, documentation and CSP checks also passed.
This correction is included in the published v0.12.1 packages.

The final [CI 37383424915](https://github.com/Moresyl/metaclean/actions/runs/37383424915)
passed all four jobs: 428 frontend tests, 247 Windows native tests (ten ignored),
93.58% frontend and 91.82% Rust line coverage, and nineteen desktop cases on each
of Windows, macOS and Linux. The preceding runtime-fix
[CI 37381268365](https://github.com/Moresyl/metaclean/actions/runs/37381268365)
also passed nineteen desktop cases on all three systems. Downloaded complete
logs were checked for the three design cases and sixteen workflow cases per OS.

Independent [images 37383882399](https://github.com/Moresyl/metaclean/actions/runs/37383882399)
and [PDF 37383886210](https://github.com/Moresyl/metaclean/actions/runs/37383886210)
passed six HEIF/AVIF, sixteen JPEG, eight PNG and six PDF cases at the final
source. Their downloaded records and source/output hashes were inspected.

[Release 37385725038](https://github.com/Moresyl/metaclean/actions/runs/37385725038)
was submitted through the authenticated browser with this exact commit. It
passed all seven jobs on its first attempt. Its source gate repeated all
nineteen Linux desktop cases, 241 Linux native tests (eleven ignored), ten
FFmpeg PCM comparisons and the thirty-six independent fidelity cases, with
91.36% Rust line coverage. Complete source logs and all four fidelity artifacts
were downloaded and inspected. The five package jobs passed their applicable
installation, portable-package and copied-DMG launch checks before finalization
created the immutable tag and published the assets.

| Public qualification | Actual result | Limit |
| --- | --- | --- |
| [Assets 37387791430](https://github.com/Moresyl/metaclean/actions/runs/37387791430) | Passed all 19 checksum-listed downloads and five updater signature/tamper-rejection checks. | Updater signing is separate from OS signing. |
| [Windows NSIS 37387794916](https://github.com/Moresyl/metaclean/actions/runs/37387794916) | Both architectures passed 0.12.0 → 0.12.1 → 0.12.0, real application-triggered signed update/restart, preferences/history/fingerprint preservation, truncated-installer rejection and uninstall. | x86 runs on a 64-bit hosted machine; arbitrary migrations remain unqualified. |
| [MSI 37387798072](https://github.com/Moresyl/metaclean/actions/runs/37387798072) | Install, upgrade, repair, rollback and removal passed. Repair restored the exact executable hash, registration and launch. | One hosted Windows transition, without interrupted-install qualification. |
| [DEB 37387801560](https://github.com/Moresyl/metaclean/actions/runs/37387801560) | All three version/launch records passed 0.12.0 → 0.12.1 → 0.12.0. | This does not qualify RPM installation or every Linux distribution. |
| [DMG 37387805306](https://github.com/Moresyl/metaclean/actions/runs/37387805306) | Both architectures passed the same three-version replacement sequence, with six-second launches. | Intel packages ran on an ARM runner through Rosetta; Apple signing/notarization remains unavailable. |
| [Crash recovery 37387808921](https://github.com/Moresyl/metaclean/actions/runs/37387808921) | The public executable preserved 64 source hashes and verified seven committed outputs; the fixture inventory had zero other files. No automatic resume occurred and the notice cleared on the second restart. | One forced-termination timing sample does not qualify arbitrary power loss or guarantee absence of temporary files at every interruption point. |

All public logs and result artifacts were downloaded and inspected. The repaired
v0.12.1 MSI product code is `{33EE7CAD-452E-488F-9372-0D043B12B1BC}`; the previous
v0.12.0 product code is `{A625F6DE-DEC6-4DD7-A191-00460FA095A9}`.
Release and deployed Pages `latest.json` were fetched independently and were
byte-identical, with SHA-256
`3853aecf8fa7cb0bd6f5225bcd4b0206f8973470fcee363eb8f0a918f3ab53d9`.
The manifest contains version 0.12.1 and the five expected signed platform URLs.

Two localized native capture scenarios passed with the v0.12.1 development
executable at 1180 × 720, exercising intake, scan, search, safe-copy cleanup and
settings. Fifteen PNGs and two finite-loop GIFs were refreshed from these actual
English/Chinese captures. Synthetic fixtures used an ignored workspace demo
directory, saved preferences were restored and the temporary fixture folders
were removed. These captures are development-build evidence, separate from the
public-installer qualifications above.

## Published v0.12.0 and native captures — 2026-10-06

The published [v0.12.0](https://github.com/Moresyl/metaclean/releases/tag/v0.12.0)
uses immutable source `7c53f8d2d236444fba0be5b93b23538f57e573a8`.
It was published at 2026-10-05 20:53:12 UTC with twenty nonempty assets.
The release matrix passed all five platforms, and all six post-publication
qualification workflows passed at this same source as recorded below.

- Local verification passed 428 frontend tests with 93.58% line coverage,
  247 Windows native tests (ten ignored), strict all-target Clippy, formatting,
  44 release automation tests, documentation/CSP/supply-chain checks and the
  116-extension manifest gate. Production and documentation builds passed.
  A threaded frontend attempt had a worker startup timeout; the complete
  single-fork rerun passed all 22 test files without worker errors.
- The rebuilt v0.12.0 Windows application passed eighteen desktop cases.
  A new regression fails on the older binary with an overlay left edge of
  264px instead of 0, and passes after root-mounted overlays. Theme/control
  geometry, visible focus, full-window coverage and focus return are checked.
- Independent [images 37355841002](https://github.com/Moresyl/metaclean/actions/runs/37355841002)
  and [PDF 37355846497](https://github.com/Moresyl/metaclean/actions/runs/37355846497)
  passed six HEIF/AVIF, sixteen JPEG, eight PNG and six PDF cases at the original
  runtime revision `e4afee41700d5fe66d9dbd57059a71d786ae781b`. Later commits change
  captures, documentation and test synchronization. Downloaded records were
  checked for case counts, unchanged JPEG
  pixels/profiles and matching PNG frame/animation inspections.
- Two localized native capture scenarios passed actual intake, scan, search,
  safe-copy cleanup and settings. Fifteen PNGs and two finite-loop GIFs were
  refreshed from this development build, using disposable synthetic files and
  restoring prior preferences. English and Chinese default cleanup preferences
  fit without scrolling. These captures are not public-installer qualification.
- Browser review checked 1180×720 and 590×360 layouts, light/dark themes,
  command search, the compact full-window confirmation and forced-color
  navigation outlines. The official npm audit reports no known vulnerabilities
  after targeted development dependency updates.

The final [CI 37360716150](https://github.com/Moresyl/metaclean/actions/runs/37360716150)
passed all four jobs at the final candidate revision: 428 frontend tests,
247 Windows native tests (ten ignored), 93.58% frontend and 91.82% Rust line
coverage, and eighteen desktop cases on each of Windows, macOS and Linux.
The preceding [CI 37359090620](https://github.com/Moresyl/metaclean/actions/runs/37359090620)
also passed all three desktop platforms after the animation synchronization fix.

[Release 37363399635](https://github.com/Moresyl/metaclean/actions/runs/37363399635)
was dispatched through the authenticated browser at the final immutable source.
Its source gate passed all eighteen Linux desktop cases, 241 Linux native tests
(eleven ignored), 91.36% Rust line coverage, ten FFmpeg PCM cases and the same
thirty-six independent image/PDF cases. Downloaded source logs and all four
fidelity artifacts were inspected. The first attempt passed Windows x64 and
both macOS builds and their actual package launch checks, but Linux and Windows
x86 never acquired hosted runners; both have no executed steps and explicit
runner-allocation failure annotations. The official
[Actions incident](https://stspg.io/c11dc9nb1zdq) also reports hosted-runner
assignment delays beginning before this release. The browser submitted a failed-jobs-only
retry on this same run and source. The eventual third attempt passed every
release gate and published the version; post-publication checks are separate
from the release matrix's package launch checks.

The second attempt again had no Windows x86 runner. Linux started its package
build, then reported a cancelled build step with all subsequent package gates
skipped; its complete log remained unavailable from GitHub's log storage.
After these failures, the browser submitted a normal workflow cancellation.
GitHub confirmed terminal cancellation at 2026-10-05 20:30:49 UTC, and the browser
submitted a third failed-jobs-only retry at the same immutable source. At recovery,
all seven existing source-evidence and successful package artifacts were unexpired.
The original Linux build-step cancellation cause is not established by the
later manual cancellation annotation.

The third attempt passed Linux and Windows x86 builds and package checks, reused
the successful source/Windows x64/macOS jobs, and passed finalization. Its actual
Linux log records an eight-second DEB launch and removal, AppImage zsync
verification and four collected distribution assets. The Windows x86 log records
six-second installed NSIS and portable launches, removal and signed updater asset
collection. The tag was created only after these gates succeeded.

The first public NSIS attempt passed x86, but its x64 job and the separate MSI
job ended without any executed steps. Both job annotations explicitly report
that a hosted runner could not be acquired. These terminal failures were
archived before browser failed-job-only retries of the same two runs and source;
no replacement workflows or releases were dispatched.

### Public package qualification

All six checks were dispatched once at the published tag and exact source above.
Downloaded logs and artifacts support the completed rows; queued jobs do not
establish qualification.

| Check | Current evidence | Limit |
| --- | --- | --- |
| [Public assets 37372566840](https://github.com/Moresyl/metaclean/actions/runs/37372566840) | Passed all 19 checksum-listed downloads and five updater signature/tamper-rejection checks. | Updater signing is separate from OS signing. |
| [Windows NSIS 37372571375](https://github.com/Moresyl/metaclean/actions/runs/37372571375) | Both x64 and x86 passed 0.11.10 → 0.12.0 → 0.11.10, real application-triggered signed update/restart, synthetic preferences/history/fingerprints, truncated-installer rejection and uninstall. | x86 runs on a 64-bit hosted machine; arbitrary migrations are not qualified. |
| [Windows MSI 37372575524](https://github.com/Moresyl/metaclean/actions/runs/37372575524) | Passed installation, upgrade, exact executable repair, registration/launch verification, manual rollback and removal. Current ProductCode: `A625F6DE-DEC6-4DD7-A191-00460FA095A9`. | Hosted Windows x64, one version pair; arbitrary migration and interrupted installation are not qualified. |
| [Linux DEB 37372580978](https://github.com/Moresyl/metaclean/actions/runs/37372580978) | Downloaded TSV/log confirms install, upgrade, manual rollback and removal at 0.11.10 → 0.12.0 → 0.11.10. | Hosted Ubuntu/Xvfb samples. |
| [Both macOS DMGs 37372584842](https://github.com/Moresyl/metaclean/actions/runs/37372584842) | Both downloaded TSVs/logs confirm six-second launches, replacement, rollback and removal across the same three versions. | Intel x86_64 ran on an arm64 host through compatibility support; Apple signing/notarization remains unavailable. |
| [Public crash recovery 37372588890](https://github.com/Moresyl/metaclean/actions/runs/37372588890) | Actual public portable v0.12.0 preserved 64 source hashes and two committed output hashes, with zero other fixture files. Two restarts made no automatic writes; a generic notice appeared once and then cleared. | One synthetic partial-batch interruption, not physical power loss or all interruption points. |

The actual release and Pages `latest.json` downloads are byte-identical,
SHA-256 `45a050616ee6d34ba0824dd90a2a2c2f7a03eced77227f77a6163fa6ac18c51d`.
Both advertise v0.12.0 and five correctly versioned platform URLs/signatures.
The public crash artifact records `candidate=false`, the exact tagged source,
and executable SHA-256
`5dfcc6d6df67b9109a09bafd95408aeeddfb6164313b6d939669619e216f2c79`.

Post-publication documentation review found that the 688px article area still
used a two-column hero and four narrow cards on a wide desktop. Its layout now
responds to the article container, with two readable cards or a single mobile
column, and avoids inherited whole-card/button underlines. At 768px the navigation
also extended about 11px beyond the viewport; compact link spacing fixes it.
Twelve isolated local Chrome views (390/590/768/1024/1440/2048px, both themes)
passed actual bounds, card readability, link semantics and no-horizontal-overflow
checks. Desktop/mobile screenshots were visually inspected. Preview asset caching
was corrected before the complete rerun; documentation checks and build passed.
The same twelve checks passed against the deployed site after
[Pages 37376546767](https://github.com/Moresyl/metaclean/actions/runs/37376546767)
published the layout correction.
This is a documentation-site change after the immutable application release.

The documentation follow-up [CI 37374640832](https://github.com/Moresyl/metaclean/actions/runs/37374640832)
passed its main quality gate and Windows/macOS desktop cases, but its Linux audio
precheck found no native cache and timed out while Cargo was still compiling;
the ten audio comparisons and desktop cases had not run. The original log is
retained. The audio harness now compiles its native fixture target once with
the same 600-second budget as the image/PDF harnesses. Per-case native execution,
FFmpeg encoding/decoding deadlines and every source/privacy/PCM assertion retain
their original 120-second bounds and semantics. All ten Opus/Vorbis PCM cases
passed locally after this test-tooling change; syntax/documentation checks and
the documentation build also passed. This does not change the shipped cleaner.

The corrected [CI 37377345533](https://github.com/Moresyl/metaclean/actions/runs/37377345533)
passed all four jobs at `ac6229b29dcd4c727cae1ef024575d63fc83f052`.
Downloaded logs confirm all ten Linux Opus/Vorbis PCM comparisons, eighteen
desktop cases on each of Windows, macOS and Linux, and the macOS filesystem
extended-attribute preservation test. Its main gate passed 428 frontend tests,
247 Windows native tests (ten ignored), 93.58% frontend and 91.82% Rust line
coverage, production/documentation builds and the security/format/release gates.
[Pages 37377345674](https://github.com/Moresyl/metaclean/actions/runs/37377345674)
also passed at this source, and the deployed validation page was checked for
the precompilation correction. These post-release checks do not change the
immutable application tag or its publicly qualified packages.

The first [candidate CI 37355757810](https://github.com/Moresyl/metaclean/actions/runs/37355757810)
passed the main gate (428 frontend tests, 247 native tests, 91.82% Rust line
coverage) and all eighteen Windows/macOS desktop cases. Linux passed the
viewport regression and the sixteen existing desktop cases, but its theme
check sampled an intermediate animated fill (`rgb(90, 90, 90)`). The check now
waits for the exact target fill before inspecting the controls; transitions
remain enabled and all final-value assertions remain strict. The local Windows
rerun and both corrected cross-platform CI runs passed all eighteen cases.

The subsequent [capture revision CI 37357510617](https://github.com/Moresyl/metaclean/actions/runs/37357510617)
also sampled an intermediate fill on Linux. Its macOS theme-reload assertion
read a missing theme attribute during document replacement. The reload check
now waits for the exact persisted theme and verifies the stored choice both
before and after refresh. These test synchronization changes do not alter the
application's theme initialization or storage behavior. The combined local
Windows rerun and the final cross-platform gate passed all eighteen cases.

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

## Published v0.11.10 verification — 2026-10-01

[v0.11.10](https://github.com/Moresyl/metaclean/releases/tag/v0.11.10), immutable
revision `92556428df40371ba240f583d67e11bdc4fae801`, was published at
2026-10-01 03:42:48 UTC with twenty nonempty assets.

- [Candidate CI 36808900161](https://github.com/Moresyl/metaclean/actions/runs/36808900161)
  passed 428 frontend tests, 247 Windows native tests (ten ignored), and sixteen
  desktop cases on each of Windows, macOS and Linux. Rust line coverage was 91.82%.
- [Release 36810266098](https://github.com/Moresyl/metaclean/actions/runs/36810266098)
  passed source validation, five platform builds and applicable installation/launch
  checks. Linux validation passed 241 native tests (eleven ignored), sixteen desktop
  cases, ten FFmpeg PCM, six PDF, six HEIF/AVIF, sixteen JPEG and eight PNG cases.
  Rust line coverage was 91.36%; frontend line coverage was 93.58%. The npm audit
  was clean; seven previously classified Rust warnings remain.
- Tagged [images 36810266094](https://github.com/Moresyl/metaclean/actions/runs/36810266094)
  and [PDF 36810266077](https://github.com/Moresyl/metaclean/actions/runs/36810266077)
  independently passed the same image/document fidelity scenarios.
- [Public assets 36811830309](https://github.com/Moresyl/metaclean/actions/runs/36811830309)
  verified nineteen checksum-listed assets, five updater signatures and rejection
  of modified package bytes.
- [NSIS 36811833175](https://github.com/Moresyl/metaclean/actions/runs/36811833175)
  passed x64/x86 application-triggered signed updates and restart, synthetic
  preferences/history/fingerprint preservation, truncated-installer rejection
  and uninstall through 0.11.9 → 0.11.10 → 0.11.9.
  [MSI 36811836748](https://github.com/Moresyl/metaclean/actions/runs/36811836748),
  [DEB 36811839406](https://github.com/Moresyl/metaclean/actions/runs/36811839406)
  and [DMG 36811842316](https://github.com/Moresyl/metaclean/actions/runs/36811842316)
  passed their corresponding transitions and launch/removal checks. MSI repair
  passed for `{E1D8B940-2B14-4566-B859-E6551757AE59}`.
- [Crash recovery 36811845168](https://github.com/Moresyl/metaclean/actions/runs/36811845168)
  preserved sixty-four source hashes and verified two committed outputs. Its
  complete inventory contained no additional files. Two restarts confirmed no
  automatic resume and a notice consumed only once. This single interruption
  point does not establish orphan-free cleanup or physical power-loss safety.
- [Pages 36811779908](https://github.com/Moresyl/metaclean/actions/runs/36811779908)
  deployed the five-platform update feed. The actual public Pages and release
  manifests were byte-identical, SHA-256
  `7951EA8D5A197FB37EAD6A1C6EC35B54190388E0B478419893221D2B3EA992C8`.

### JPEG embedded previews and private APP0 payloads

v0.11.10 reports and removes JFIF RGB thumbnails, JFXX JPEG/palette/RGB previews,
application-private APP0 segments and trailing private bytes inside JFIF segments.
It retains exact JFIF version, resolution units, densities and pixel aspect ratio.
The [JFIF 1.02 specification](https://www.w3.org/Graphics/JPEG/jfif3.pdf) describes
these separate display and preview fields. Truncated headers or thumbnail pixels,
inconsistent thumbnail dimensions and residual APP0 privacy payloads are rejected.

The detection regression failed on the previous implementation. The independent
Pillow harness now seeds all these preview classes and private sentinel bytes
into sixteen JPEG copy/replace cases. Assertions check private-payload removal,
exact compressed scans, decoded main-image pixels, ICC, orientation, EXIF/JFIF
print density and source/backup integrity. These synthetic cases do not qualify
every encoder, application-specific extension or image-processing workflow.

## Published v0.11.9 verification — 2026-10-01

[v0.11.9](https://github.com/Moresyl/metaclean/releases/tag/v0.11.9), immutable
revision `e0113eb70ba6fd2049297464f66744a9f8b48ca7`, was published at
2026-09-30 18:52:40 UTC with twenty nonempty assets.

- [Candidate CI 36755147155](https://github.com/Moresyl/metaclean/actions/runs/36755147155)
  passed 428 frontend tests, 244 Windows native tests (ten ignored), and sixteen
  desktop cases on each of Windows, macOS and Linux. Rust line coverage was 91.77%.
- [Release 36757394934](https://github.com/Moresyl/metaclean/actions/runs/36757394934)
  passed source validation, five builds and applicable installation/launch checks.
  Linux source validation passed 238 native tests (eleven ignored), sixteen desktop
  cases, ten independent FFmpeg PCM cases, six PDF, six HEIF/AVIF, sixteen JPEG
  and eight PNG cases. Rust line coverage was 91.30%; frontend line coverage was
  93.58%. The npm audit was clean; seven previously classified Rust warnings remain.
- [Public assets 36761767639](https://github.com/Moresyl/metaclean/actions/runs/36761767639)
  verified nineteen checksum-listed assets, five updater signatures and rejection
  of modified package bytes.
- [NSIS 36762462081](https://github.com/Moresyl/metaclean/actions/runs/36762462081)
  passed x64/x86 application-triggered signed updates and restart, synthetic
  preferences/history/fingerprint preservation, truncated-installer rejection
  and uninstall through 0.11.8 → 0.11.9 → 0.11.8.
  [MSI 36761777603](https://github.com/Moresyl/metaclean/actions/runs/36761777603),
  [DEB 36761783676](https://github.com/Moresyl/metaclean/actions/runs/36761783676)
  and [DMG 36761788601](https://github.com/Moresyl/metaclean/actions/runs/36761788601)
  passed the corresponding transitions and launch/removal checks. MSI repair
  passed for `{54AAB4E4-6831-4625-8DE1-0C38C04EB6AB}`.
- [Crash recovery 36761793491](https://github.com/Moresyl/metaclean/actions/runs/36761793491)
  preserved sixty-four sources and verified one committed output. Its complete
  inventory contained no additional files. Two restarts confirmed no automatic
  resume and a notice consumed only once. This single interruption point does
  not establish orphan-free cleanup or physical power-loss safety.
- [Pages 36761714180](https://github.com/Moresyl/metaclean/actions/runs/36761714180)
  deployed the five-platform update feed. Public Pages and release manifests were
  byte-identical, SHA-256
  `A4461ABAA4A18D086F104378CD891E94F26547740774CEC030FAF3DC5FAD624B`.

The [first NSIS run 36761772615](https://github.com/Moresyl/metaclean/actions/runs/36761772615)
failed while initializing synthetic storage on the previous v0.11.8 application,
before attempting an upgrade. A diagnostic-only harness change added the failed
phase and WebView exception details; the full rerun above and a
[second complete run 36762793328](https://github.com/Moresyl/metaclean/actions/runs/36762793328)
passed both architectures without a product change or weakened checks. The initial
exception's cause remains unconfirmed.

### PNG modification-time cleanup

v0.11.9 reports, removes and rejects residual PNG `tIME` chunks.
The [PNG specification](https://www.w3.org/TR/png-3/#11tIME) defines this optional
chunk as the image's last-modification time, distinct from filesystem timestamps.
The new detection regression failed on v0.11.8's implementation before the fix.
After the change, 244 native tests passed (ten ignored), strict Clippy and
formatting passed, and eight independent Pillow cases passed locally.

The fixtures cover RGBA, indexed-color transparency, 16-bit grayscale and a
two-frame APNG, each in copy and replacement modes. Independent CRC/chunk
inspection confirms that only `tIME` disappears. Decoded pixels, all retained
chunks, color profiles, physical resolution, animation timing/controls and the
configured filesystem modification time remain identical. Source bytes or
replacement backups also match. These synthetic fixtures do not qualify all
PNG encoders or establish support for arbitrary unknown chunks.

The independent checks are included in image and release source gates.
[Image run 36753853542](https://github.com/Moresyl/metaclean/actions/runs/36753853542)
passed six HEIF/AVIF, sixteen JPEG and eight PNG cases;
[PDF run 36753853573](https://github.com/Moresyl/metaclean/actions/runs/36753853573)
passed six page/text/form/source-integrity cases.

The initial candidate CI stopped at newly disclosed npm dependency advisories.
The desktop test-toolchain overrides now use `ip-address` 10.7.1 and
`brace-expansion` 1.1.21 / 2.1.7. A fresh local npm audit reports zero known
vulnerabilities; forty release automation tests and the supply-chain check pass.
The qualified candidate and public release results are recorded above.

## Published v0.11.8 verification — 2026-09-29

[v0.11.8](https://github.com/Moresyl/metaclean/releases/tag/v0.11.8), immutable
revision `4ad0ee63a9bae22ea3c2dd5fcaff039b22c0c58d`, was published at
2026-09-29 05:57:19 UTC with twenty nonempty assets.

- [Release 36527199482](https://github.com/Moresyl/metaclean/actions/runs/36527199482)
  passed source validation, five platform builds and applicable installation and
  launch checks. Linux validation passed 428 frontend tests, 237 native tests
  (ten ignored), sixteen desktop cases, ten independent FFmpeg PCM cases,
  six PDF, six HEIF/AVIF and sixteen JPEG cases. Rust line coverage was 91.35%;
  frontend line coverage was 93.58%. The npm audit found no known vulnerabilities;
  seven previously classified Rust warnings remain.
- [Public assets 36528727885](https://github.com/Moresyl/metaclean/actions/runs/36528727885)
  verified nineteen checksum-listed assets, five updater signatures and rejection
  of modified package bytes.
- [NSIS 36528731019](https://github.com/Moresyl/metaclean/actions/runs/36528731019)
  passed x64/x86 application-triggered signed updates and restart, synthetic
  preferences/history/fingerprint preservation, truncated-installer rejection
  with the old executable intact, and uninstall through 0.11.7 → 0.11.8 → 0.11.7.
  [MSI 36528733990](https://github.com/Moresyl/metaclean/actions/runs/36528733990),
  [DEB 36528736592](https://github.com/Moresyl/metaclean/actions/runs/36528736592)
  and [DMG 36528740683](https://github.com/Moresyl/metaclean/actions/runs/36528740683)
  passed the corresponding version transitions and launch/removal checks.
  MSI repair passed for `{F05371A7-098E-4F43-A4C9-29A0F33FF74C}`.
- [Crash recovery 36528743327](https://github.com/Moresyl/metaclean/actions/runs/36528743327)
  preserved sixty-four source hashes and verified one committed output. Forced
  termination left one additional 8,388,605-byte temporary file whose hash matched
  the expected cleaned output, not the source. Two restarts preserved the snapshot,
  did not resume cleanup and showed the interruption notice only once. This is
  direct evidence that forced termination can leave a temporary file.
- [Pages 36528633642](https://github.com/Moresyl/metaclean/actions/runs/36528633642)
  deployed an update feed byte-identical to the release feed, version 0.11.8 with
  five platforms, SHA-256
  `7D80ADB8ADCBA105AA2996F8F62F1C8DC4C6FB38535D7483D545618A0C07AE18`.

[Public desktop measurements 36528753753](https://github.com/Moresyl/metaclean/actions/runs/36528753753)
passed three fresh runs per fixture size. All source, output and audit hashes
matched; all twelve peak inventories contained seven application/WebView processes.
Both crash and memory evidence identify public executable SHA-256
`56A7AF5BB09FD8555D50FFBB4A2EBAED15F14FE83A60CF6750287AB148D8DEB1`.

| Synthetic UTF-8 input | Scan | Cleanup | Aggregate working set | Aggregate private memory |
|---|---:|---:|---:|---:|
| 64 MiB | 1.03–1.08 s | 1.17–1.36 s | 504.95–506.64 MiB | 272.02–273.76 MiB |
| 256 MiB | 3.07–3.77 s | 4.46–4.78 s | 889.04–892.42 MiB | 654.48–659.94 MiB |

These are sampled application-plus-WebView totals on disposable hosted machines,
not memory limits or a controlled speed comparison. Shared working-set pages may
be counted twice and sampling may miss peaks. Windows x86 ran on 64-bit Windows;
Intel DMG ran on arm64 macOS. Synthetic migration and forced-process-termination
tests do not qualify arbitrary user state, physical power loss, interrupted
installation or Apple Gatekeeper approval. The user's installed application was
not changed.

## UTF-8 buffer reuse candidate measurements — 2026-09-29

Revision `5616ba7893a91ccdfe114332fc1031250f726ad7` borrows validated
UTF-8 input and consumes the cleaned string for UTF-8 output. Three isolated
optimized native processes per implementation each scanned and cleaned 64 MiB
and 256 MiB synthetic text fixtures. Every source, output and audit fingerprint
check passed. Observed peak working sets were 1,030.87–1,031.34 MiB before and
518.86–519.32 MiB after; sampled private memory was 1,028.21–1,028.25 MiB
before and 515.20–515.25 MiB after. The requested sampling wait was 5 ms.

These numbers include native test fixture and verification work, exclude the
desktop UI/WebView, and are not an application memory requirement or upper bound.
Concurrent compilation also prevents a controlled throughput comparison.
The borrowed-input regression failed on the original implementation. All 243
native tests then passed (nine ignored), along with strict Clippy and formatting.
Exact-byte tests cover empty and multibyte text, UTF-8 BOM insertion and UTF-16
endianness. Sixteen local Windows desktop cases, the production frontend build
and the documentation build also passed.

[Hosted candidate measurements 36525360046](https://github.com/Moresyl/metaclean/actions/runs/36525360046)
passed three fresh runs per size, including source, output and audit fingerprint
checks. All twelve peak process inventories contained exactly seven MetaClean
and WebView processes, with creation-time ancestry checks:

| Synthetic UTF-8 input | Aggregate working set | Aggregate private memory |
|---|---:|---:|
| 64 MiB | 504.23–507.87 MiB | 270.99–273.05 MiB |
| 256 MiB | 887.80–889.71 MiB | 655.16–657.00 MiB |

These candidate executables still reported version 0.11.7; `application.json`
records `candidate: true`, the revision above and each executable hash. They
are distinct from the published v0.11.7 baseline below. Measurements include
startup through cleanup, use a requested 200 ms wait plus process enumeration,
can miss peaks and can double-count shared working-set pages. Hosted machines
vary, so this is neither a controlled timing comparison nor an application
memory upper bound or proof that whole-application memory was halved.

The final v0.11.8 revision `4ad0ee63a9bae22ea3c2dd5fcaff039b22c0c58d`
passed [CI 36525823068](https://github.com/Moresyl/metaclean/actions/runs/36525823068):
428 frontend tests, 243 native tests, 91.82% Rust line coverage with the documented
exclusions, and sixteen desktop cases on each of Windows, Linux and macOS.
Tagged independent [PDF checks](https://github.com/Moresyl/metaclean/actions/runs/36527199495)
and [image checks](https://github.com/Moresyl/metaclean/actions/runs/36527199446)
passed six PDF, six HEIF/AVIF and sixteen JPEG cases.

## Published v0.11.7 verification — 2026-09-29

The [stable release](https://github.com/Moresyl/metaclean/releases/tag/v0.11.7)
at `8efad9c51c6129c25d722d1c7eb62dbe875393b0` was published on
2026-09-29 at 04:37:26 UTC with twenty nonempty assets.

- [Candidate CI 36519310581](https://github.com/Moresyl/metaclean/actions/runs/36519310581)
  passed 428 frontend tests, 240 Windows native tests (nine ignored) and sixteen
  desktop cases on each of Windows, Linux and macOS. Its Windows run recorded
  91.77% Rust line coverage with the documented exclusions. The rebuilt local
  Windows desktop suite also passed all sixteen cases.
- [Release 36520841603](https://github.com/Moresyl/metaclean/actions/runs/36520841603)
  passed source revalidation, all five platform builds, applicable installation
  and launch checks, and finalization. Linux source validation passed 428 frontend
  tests, 234 native tests (ten ignored), sixteen desktop cases, ten FFmpeg PCM
  cases, six PDF, six HEIF/AVIF and sixteen JPEG cases. Linux Rust line coverage
  was 91.30%. Frontend coverage was 89.96% statements, 85.54% branches, 92.07%
  functions and 93.58% lines. The npm audit reported no known vulnerabilities;
  seven previously classified Rust warnings remain.
- The separate tagged [image workflow 36520841553](https://github.com/Moresyl/metaclean/actions/runs/36520841553)
  and [PDF workflow 36520841510](https://github.com/Moresyl/metaclean/actions/runs/36520841510)
  also passed. Downloaded image evidence includes sixteen matching JPEG raw-pixel
  hash pairs and six HEIF/AVIF result records; all six Poppler PDF cases passed.
- [Public assets 36522541974](https://github.com/Moresyl/metaclean/actions/runs/36522541974)
  passed verification of the nineteen checksum-listed assets, all five updater
  signatures and rejection of modified package bytes.
- [Windows NSIS 36522545117](https://github.com/Moresyl/metaclean/actions/runs/36522545117)
  passed x64/x86 application-triggered updates, automatic restart and synthetic
  settings/history/fingerprint preservation through 0.11.6 → 0.11.7 → 0.11.6.
  Truncated installers were rejected with the previous executable hash and
  launch intact, and final removal passed.
- [MSI 36522548342](https://github.com/Moresyl/metaclean/actions/runs/36522548342),
  [DEB 36522551545](https://github.com/Moresyl/metaclean/actions/runs/36522551545)
  and [both DMGs 36522554872](https://github.com/Moresyl/metaclean/actions/runs/36522554872)
  passed the same upgrade/rollback pair, applicable launch checks and removal.
  MSI repair passed for product code `{A3F32EC3-8C41-48BA-A3F7-C9A881F9C59F}`.
- [Crash recovery 36522558812](https://github.com/Moresyl/metaclean/actions/runs/36522558812)
  preserved all sixty-four source hashes and verified one committed output.
  The complete file inventory, including hidden files, contained no other files
  in this run. Two restarts preserved the snapshot, did not resume cleanup and
  showed the interruption notice only once. Public executable SHA-256:
  `D4AAC722073F65BB9BC2D5C381CDE295852036307C793F5BF0EB345585E98227`.
- After [Pages 36522454945](https://github.com/Moresyl/metaclean/actions/runs/36522454945),
  the release and Pages update manifests were byte-identical for version 0.11.7
  and all five platforms, SHA-256
  `893D7939F63F1A9C71C60E1B8A8DEC95BE5C83C8E03E15BBBA9B181E542A040F`.

These use disposable hosted machines and synthetic state. Windows x86 runs on
64-bit Windows and Intel DMG on an arm64 Mac. They do not qualify native older
CPUs/32-bit Windows, arbitrary data migration, physical power loss, interrupted
installation or Apple Gatekeeper approval. A crash run without extra files does
not guarantee absence of orphaned temporary files at every interruption point.
The user's local installed application was not changed. Separate process-tree
measurements and isolated source-guard results are recorded below.

## Published desktop memory with process identity checks — v0.11.7

[Run 36523943242](https://github.com/Moresyl/metaclean/actions/runs/36523943242)
passed six fresh runs of the checksummed public Windows x64 portable package,
three each for 64 MiB and 256 MiB synthetic UTF-8 text. Real desktop scan and
cleanup controls were used. Complete source/output hashes and stored audit
fingerprints matched, with exactly the expected source and output files.

The sampler validates the root's creation time, rejects parent relationships
where the child predates the current parent instance, and reads memory counters
from the same CIM snapshot as process identities. It exports names, IDs,
creation times and counters for each observed working-set/private-byte peak.
Regression tests cover stale parent IDs, reused root IDs, missing/duplicate
identities, timestamp precision and invalid counters; live CIM checks passed
on every runner. All six peak inventories contained seven processes and only
`MetaClean.exe` and `msedgewebview2.exe` names.

| Input | UI scan time | UI cleanup time | Maximum sampled sum of working sets | Maximum sampled sum of private bytes |
| --- | --- | --- | --- | --- |
| 64 MiB | 1.04–1.28 s | 1.27–1.29 s | 501.81–507.93 MiB | 332.84–337.12 MiB |
| 256 MiB | 2.72–3.81 s | 4.44–4.89 s | 953.24–1,145.31 MiB | 911.56–916.15 MiB |

Runs used Windows Server 2025 hosts with AMD EPYC 7763 or Intel Xeon 6973P-C
processors. Sampling includes startup and the WebView, excludes the controller,
and requests 200 ms waits between process enumeration. Shared resident pages
may be counted more than once, enumeration adds variable delay and short peaks
can be missed. These are workload-specific observations, not a RAM requirement,
memory upper bound or causal before/after performance comparison. They do not
support interpreting the isolated source-guard reduction as a halving of total
application memory. Other formats, platforms and slow storage remain unqualified.

The earlier [run 36523255513](https://github.com/Moresyl/metaclean/actions/runs/36523255513)
passed all file-integrity checks but reported 142 processes and 3,679.77 MiB
in one 64 MiB case, versus seven processes in its other cases. That sampler
used PID-only ancestry and did not export contributing processes, so the outlier's
ownership cannot be established. It is not accepted as application-memory
evidence. The old algorithm could include an older unrelated process through a
reused parent PID; the new regression fixture proves this mechanism, but cannot
retroactively establish which processes caused that particular outlier.

## Bounded source revalidation — v0.11.7

The source guard now reads through a 64 KiB buffer and compares every byte,
including an explicit end-of-file check. It retains validated opening, source
size limits and path-link checks, then refreshes handle metadata before comparing
the metadata snapshot. This removes the second source-sized allocation from
revalidation; it does not make all cleaners streaming or bound total app memory.

Local native tests passed 240 cases (nine explicit/environment-dependent cases
ignored), including new chunk-boundary, truncated/appended data, short-read,
interruption, I/O-error and same-size mutation cases. Existing permission,
timestamp, reparse-path and guarded-write tests still pass. Strict all-target,
all-feature Clippy and formatting passed before the version-only candidate bump.

The ignored `safe_io::tests::benchmark_source_revalidation` runs five exact-byte
checks against a generated file. `METACLEAN_GUARD_MIB` accepts 64 or 256;
`METACLEAN_GUARD_BASELINE=true` reproduces the previous whole-buffer guard inside
test code. Twelve isolated optimized Windows processes (three per size/mode)
were measured using process peak working set and requested 5 ms private-byte
sampling, alternating old/new modes. Each process includes fixture allocation
and excludes cleaning and the WebView; both modes use the same executable.

| Source size | Previous guard peak working set | Bounded guard peak working set |
| --- | --- | --- |
| 64 MiB | 132.66–133.11 MiB | 68.73–68.77 MiB |
| 256 MiB | 516.66–516.67 MiB | 260.75 MiB |

All 428 frontend tests also pass (93.58% lines, 85.49% branches). Independent
JPEG sixteen-case, HEIF/AVIF six-case and MuPDF six-case fidelity checks pass with
the candidate version, including source/backup hashes and both output modes.

Every measurement's exact-byte and metadata assertions passed. These observations
are specific to this synthetic revalidation workload and machine, not a guarantee
of application-wide peak memory or throughput. Hosted candidate and publication
results are recorded above.

## Published v0.11.6 verification — 2026-09-29

The [stable release](https://github.com/Moresyl/metaclean/releases/tag/v0.11.6)
at `5d6da93a5355048c2ab3f404f4022f46f70d635b` was published on
2026-09-29 at 03:21:07 UTC with 20 nonempty assets.

[Release 36514991683](https://github.com/Moresyl/metaclean/actions/runs/36514991683)
passed source revalidation, all five platform builds, applicable package and
launch checks, and finalization. Linux source validation passed 428 frontend
tests, 231 native tests (nine ignored), sixteen desktop cases, ten FFmpeg PCM
cases, six PDF cases, six HEIF/AVIF cases and sixteen JPEG fidelity cases.
Frontend coverage was 89.96% statements, 85.54% branches, 92.07% functions and
93.58% lines. Linux Rust line coverage was 91.46% with the documented exclusions.
The npm audit reported no known vulnerabilities; seven previously classified
Rust warnings remain.

- [Public assets 36516811314](https://github.com/Moresyl/metaclean/actions/runs/36516811314)
  passed checksum verification for nineteen listed assets, all five updater
  signatures and rejection of modified package bytes.
- [Windows NSIS 36516815192](https://github.com/Moresyl/metaclean/actions/runs/36516815192)
  passed x64/x86 application-triggered signed updates, restart and preservation
  of synthetic settings/history/fingerprints across 0.11.5 → 0.11.6 → 0.11.5.
  Truncated installers were rejected with previous executable hashes and launch
  intact; removal also passed.
- [MSI 36516818549](https://github.com/Moresyl/metaclean/actions/runs/36516818549),
  [DEB 36516821596](https://github.com/Moresyl/metaclean/actions/runs/36516821596)
  and [both DMGs 36516826353](https://github.com/Moresyl/metaclean/actions/runs/36516826353)
  passed the same version transition, applicable launch checks and removal.
  MSI repair passed for product code `{6B8A0841-6B2F-40E6-909C-DD64A6A9B52C}`.
- [Crash recovery 36516829493](https://github.com/Moresyl/metaclean/actions/runs/36516829493)
  preserved all sixty-four source hashes and verified one committed output.
  Two restarts left the file snapshot unchanged; cleanup did not automatically
  resume and the interruption notice cleared after its first display. The
  snapshot also contained one additional file. This runner records its count,
  but does not export its name or contents, so this result does not establish
  absence or cleanup of orphaned temporary files. Public executable SHA-256:
  `D991FF63E6ACD7FB2A3432FF52757ED67E8398546658A62AEBB47F0574A2C033`.
- The crash verifier now exports a per-file inventory with name, size, SHA-256,
  source/output/other classification and exact source/clean-output hash matches;
  hidden files are included in both inventory and restart comparisons.
  Four subsequent public-binary runs
  ([36517727117](https://github.com/Moresyl/metaclean/actions/runs/36517727117),
  [36517907228](https://github.com/Moresyl/metaclean/actions/runs/36517907228),
  [36517911359](https://github.com/Moresyl/metaclean/actions/runs/36517911359),
  [36517914627](https://github.com/Moresyl/metaclean/actions/runs/36517914627))
  all preserved sixty-four sources, verified respectively one, two, two and one
  committed outputs, and found no additional files. Both restart checks passed
  in every run. These timing-dependent observations do not identify the earlier
  extra file or establish that forced termination always leaves no temporary
  files. The named temporary-file implementation relies on normal process
  cleanup; abrupt termination can bypass that cleanup.
- After [Pages 36516748487](https://github.com/Moresyl/metaclean/actions/runs/36516748487),
  the public Pages and release update manifests were byte-identical, reporting
  0.11.6 and five platforms, SHA-256
  `BBFDA758480FB7958B430729BEF6E0553161E1F697A9CA589A796B9CE0A41E0E`.

These are disposable hosted-machine and synthetic-state checks. Windows x86
runs on 64-bit Windows and Intel DMG on an arm64 Mac. They do not qualify native
older-CPU/32-bit-OS compatibility, arbitrary user-data migration, physical power
loss, interrupted installation or Apple Gatekeeper approval. The user's local
installation was not changed. Previous memory measurements were not repeated.

## Published v0.11.5 verification — 2026-09-29

The [stable release](https://github.com/Moresyl/metaclean/releases/tag/v0.11.5)
at `6d937b9fcc2940b3621a43935dfc8a1073802647` was published on
2026-09-29 at 01:22:41 UTC with 20 nonempty assets.

- [Final candidate CI 36504579824](https://github.com/Moresyl/metaclean/actions/runs/36504579824)
  passed 424 frontend tests, 233 Windows native tests (6 ignored), the quality
  gates and 15 desktop E2E cases on each of Windows, Linux and macOS. Windows
  Rust line coverage was 92.07% with the documented exclusions. Local rebuilt
  Windows E2E and a separate run after the navigation-test fix each passed 15 cases.
- The earlier candidate failed one Linux E2E case when a navigation helper
  indexed an empty button array after refresh. The helper now waits for the
  actual visible button and clicks it; behavioral assertions and timeouts remain
  intact. The final three-platform run above verifies the revised helper.
- [Release 36505903839](https://github.com/Moresyl/metaclean/actions/runs/36505903839)
  passed source revalidation, all five builds, package/launch checks and
  finalization. Linux source validation passed 424 frontend, 227 native
  (7 ignored), 15 desktop E2E, ten FFmpeg PCM cases and all six independent
  PDF fidelity cases. Rust line coverage was 91.59%; seven previously classified
  Rust warnings remain. The official npm audit reported no known vulnerabilities.
- [Public verification 36507680233](https://github.com/Moresyl/metaclean/actions/runs/36507680233)
  checked 19 checksum-listed assets and all five updater signatures against the tagged
  public key, including rejection of modified bytes. Following successful Pages
  deployment 36507610470, release and Pages update feeds both reported 0.11.5
  and were byte-identical across all five platforms.
- [Public crash regression 36507697390](https://github.com/Moresyl/metaclean/actions/runs/36507697390)
  verified 64 unchanged sources, one committed output with the expected hash,
  no extra files, a one-time restart notice and no automatic resumption. The
  public x64 executable SHA-256 was
  `cf61eb3a7df34f29ab365f40e9877c6c16c701eaeb1a57dcfc99528913b25a18`.
- The 0.11.4 → 0.11.5 → 0.11.4 pair passed
  [MSI repair/transitions 36507687631](https://github.com/Moresyl/metaclean/actions/runs/36507687631),
  [DEB transitions 36507690761](https://github.com/Moresyl/metaclean/actions/runs/36507690761)
  and [both DMG architectures 36507694143](https://github.com/Moresyl/metaclean/actions/runs/36507694143).
  Version checks, applicable launch checks, repair and removal passed.
- [Windows NSIS 36507899300](https://github.com/Moresyl/metaclean/actions/runs/36507899300)
  passed x64/x86 application-triggered signed updates, automatic restart,
  preservation of synthetic settings/history/fingerprints, manual downgrade and
  removal. Truncated installers were rejected while the old executable hash,
  registration and launch remained intact.

These checks use disposable hosted machines and synthetic state. Windows x86
runs on 64-bit Windows and Intel DMG runs on an arm64 Mac; they do not establish
native older-CPU/32-bit-OS compatibility,
arbitrary user-data migration, physical power-loss recovery, interrupted
installation or Apple Gatekeeper approval. Earlier memory measurements were
not repeated for this PDF patch.

## Independent JPEG print/display qualification — v0.11.6

`scripts/verify-jpeg-fidelity.py` generates four original JPEG fixtures and
checks copy/replace with orientation retention both enabled and disabled:
sixteen cases. Independent Pillow decoding verifies exact source EXIF rational
density values and units, JFIF density, raw pixels, expected displayed pixels,
ICC bytes, permitted EXIF fields and private-author removal. Native SHA-256
results and unchanged source/backup bytes are also checked. All sixteen cases
pass locally with Pillow 12.2.0. Native tests cover both TIFF byte orders,
malformed/partial/duplicate/conflicting density and orientation-only rescans.

Local candidate checks passed 428 frontend tests (93.58% lines, 85.49% branches),
237 native tests with eight environment-dependent tests ignored, strict Clippy
and all sixteen Windows desktop cases. The added desktop scenario scans an
original generated JPEG containing only retained display metadata, then turns
off orientation preservation and creates a safe copy through the real UI.
The independent JPEG sixteen-case and HEIF six-case suites were repeated after
the shared test-helper changes; all passed. The six PDF cases with embedded
JPEG display metadata also pass MuPDF.

[Final candidate CI 36513754246](https://github.com/Moresyl/metaclean/actions/runs/36513754246)
passed 428 frontend tests, 237 Windows native tests (eight ignored) and sixteen
desktop cases on each of Windows, macOS and Linux. The 2026-09-29 Windows run
recorded 91.93% Rust line coverage with the documented exclusions.
[Independent image checks 36513754218](https://github.com/Moresyl/metaclean/actions/runs/36513754218)
passed all sixteen JPEG and six HEIF/AVIF cases;
[PDF checks 36513754127](https://github.com/Moresyl/metaclean/actions/runs/36513754127)
passed all six cases using Poppler. Publication and public-package qualification
remain separate gates.

Before this change, a real generated EXIF-only density sample lost tags
282/283/296 despite retaining identical decoded pixels. A second sample with
orientation also showed that a rebuilt orientation segment was reported as
private EXIF. These regressions are now covered separately from pixel fidelity.
This qualifies the stated synthetic cases, not arbitrary images, print drivers
or application-specific interpretation of conflicting JFIF/EXIF density.

## Independent HEIF/AVIF qualification — 2026-09-29

`scripts/verify-heif-fidelity.py` generates synthetic HEIC RGB, two-image HEIC
and transparent AVIF fixtures with ICC profiles, rotated display orientation,
EXIF author/description fields and XMP. In both copy and replace modes, external
decoders compare frame counts, RGBA pixels, displayed pixels/dimensions, alpha
ranges and exact ICC profile bytes. The checks require distinct HEIC frames and
varying AVIF alpha, then verify private EXIF/XMP fields and marker bytes are gone.
Native SHA-256 results and independent source/backup byte comparisons also pass.

All six cases pass locally using Pillow 12.2.0, pillow-heif 1.3.0 and libheif
1.21.2; codec versions are saved with each run. A shared ignored native test
helper now serves these cases and the existing six PDF cases, which also passed
after the refactor. No production cleaner change was required by these samples.
The [independent Linux workflow 36509053726](https://github.com/Moresyl/metaclean/actions/runs/36509053726)
also passed all six cases. These checks now run before future release packaging;
changes to the shared engine trigger both PDF and HEIF fidelity workflows.
This matrix does not qualify HDR, gain maps, depth images, timed animations,
every encoder, arbitrary RAW files or all HEIF/AVIF reader applications.

## Independent PDF fidelity qualification — 2026-09-29

`scripts/verify-pdf-fidelity.py` generates its own ReportLab fixtures: two-page
vector/text with rotation, a two-page interactive form with three fields, and
an ASCII85-wrapped JPEG with EXIF plus transparent vector content. Copy and
replace modes each verify source/backup bytes and native source/output hashes.
Independent pypdf checks compare page geometry, text, field names, values and
widget state. Pillow checks decoded JPEG pixels and private EXIF removal while
verifying retained density, orientation and ICC bytes in the current fixtures.

Local MuPDF 1.27.2.3 rendering at 144 dpi passes all six cases with identical
before/after RGB pixels. The original implementation failed the form case:
all three field names disappeared and the rendered page changed. Distinguishing
widget field names from annotation authors fixes that regression. The image
case initially failed closed on its ordinary ASCII85/DCT filter chain; bounded
ASCII85 unwrapping now supports that chain without JPEG pixel re-encoding.
Malformed encodings, unknown chains and non-default composite parameters are
rejected. Dedicated unit tests cover decode limits and integer-overflow input.

The hosted [PDF workflow 36502482296](https://github.com/Moresyl/metaclean/actions/runs/36502482296)
passed all six cases with Poppler on Linux. The release source gate now runs
the same checks before packaging. These synthetic samples do not establish arbitrary PDF, XFA,
digital-signature, accessibility or complex form compatibility.

## Published v0.11.4 verification — 2026-09-29

The [stable release](https://github.com/Moresyl/metaclean/releases/tag/v0.11.4)
at `5748de3e7eeb8a876a934d0bd7b928361bb33c5d` was published on
2026-09-28 at 23:21:08 UTC with 20 nonempty assets.

- [Candidate CI 36494120243](https://github.com/Moresyl/metaclean/actions/runs/36494120243)
  passed 424 frontend tests, 230 Windows native tests (5 ignored), the quality
  gates and 15 desktop E2E cases on each of Windows, Linux and macOS. Windows
  Rust line coverage was 92.25% using the documented CI exclusions. A separate
  local rebuilt Windows desktop run also passed all 15 cases.
- [Release 36495605281](https://github.com/Moresyl/metaclean/actions/runs/36495605281)
  passed source revalidation, all five builds, applicable installer/portable/app
  launch checks and finalization. Linux source revalidation passed 424 frontend,
  224 native (6 ignored), 15 desktop tests and ten independent FFmpeg PCM checks;
  Rust line coverage was 91.76%. Seven known Rust warnings remain as classified below.
- [Public verification 36497748414](https://github.com/Moresyl/metaclean/actions/runs/36497748414)
  checked 19 checksum-listed assets, all five updater signatures against the
  tagged public key, and rejection of modified package bytes. After successful
  Pages deployment 36497537117, direct downloads of the release and Pages update
  feeds both reported 0.11.4 and were byte-identical across all five platforms.
- [Public crash regression 36497768170](https://github.com/Moresyl/metaclean/actions/runs/36497768170)
  verified 64 unchanged sources, one committed output with the expected hash,
  no extra fixture files, a one-time restart notice and no automatic resumption.
  The public x64 executable SHA-256 was
  `c8349d7a5f9724f2c06cefb85d1bc15bb4e1d3259b0612c949af2576f9644fb6`.
- [NSIS 36497752191](https://github.com/Moresyl/metaclean/actions/runs/36497752191)
  passed x64/x86 application-triggered signed updates from 0.11.3 to 0.11.4,
  automatic restart, synthetic settings/history/hash preservation, downgrade
  and removal. Truncated installers were rejected while the old executable
  hash, registration and launch remained intact.
- The 0.11.3 → 0.11.4 → 0.11.3 pair also passed
  [MSI repair/transitions 36497756113](https://github.com/Moresyl/metaclean/actions/runs/36497756113),
  [DEB transitions 36497759817](https://github.com/Moresyl/metaclean/actions/runs/36497759817)
  and [both DMG architectures 36497763493](https://github.com/Moresyl/metaclean/actions/runs/36497763493).
  MSI repair restored the executable after deliberate damage. DEB live launch
  windows and DMG versions, identity and six-second launches were verified.

These checks use disposable hosted machines and synthetic state. Windows x86
runs on 64-bit Windows and Intel DMG runs on an arm64 Mac; they do not prove
native older-CPU/32-bit-OS compatibility, arbitrary user-data migration, physical
power-loss recovery, interrupted installation or Apple Gatekeeper approval.
The 0.11.3 memory measurements below were not repeated for 0.11.4.

## Post-release development toolchain audit — 2026-09-29

[Documentation-commit CI 36498203678](https://github.com/Moresyl/metaclean/actions/runs/36498203678)
failed the official npm audit after v0.11.4 publication. The updated
[GHSA-3wwx-pv8p-q78v advisory](https://github.com/advisories/GHSA-3wwx-pv8p-q78v)
identified `undici` 6.28.0 and 7.29.0 in the WebdriverIO/Cheerio and jsdom test
dependency chains. Earlier successful audit runs remain historical observations,
not guarantees against subsequently updated advisory data.

The workspace now overrides those two versions to the published fixes 6.28.1
and 7.29.1. The official npm audit reports no known vulnerabilities after a
frozen-lockfile installation. `pnpm why --prod undici` returns no production
dependency path. This is development-toolchain maintenance; it does not replace
the published v0.11.4 binaries or move their tag.

## Native documentation captures — 2026-09-29

Refreshed the 15 published PNG captures and two workflow GIFs using the rebuilt
v0.11.4 Windows desktop at source `291b80e`, with the E2E driver enabled.
Both English and Chinese capture scenarios passed real intake, scan, search,
cleanup and settings flows using disposable synthetic files. Captures assert
1180 × 720 dimensions and no horizontal overflow; source text still contains
the synthetic markers while cleaned copies do not. The capture restores prior
application storage and removes only its own temporary fixtures.

The screenshots were visually reviewed, including light/dark appearance,
localized controls and the current version label. The two five-frame animations
are generated from these unmodified captures and use the existing finite-loop
timing. These are development-build UI captures, not a claim of new public-binary
performance or additional external-application compatibility.

## Rust dependency warning triage (0.11.4)

Reviewed on 2026-09-29 using the current lockfile, official crates.io metadata,
the official RustSec database and `cargo tree --target all --invert PACKAGE`.
`cargo audit` reports zero vulnerability entries and seven informational warnings
after the single-package update below. Informational does not mean harmless.

| Dependency / advisory | Classification | Evidence and required follow-up |
|---|---|---|
| `chacha20` 0.10.1, yanked | Upgradable; updated to 0.10.2 in v0.11.4 | `metaclean → lopdf 0.44.0 → rand 0.10.2 → chacha20`. Official registry marks 0.10.1 yanked and 0.10.2 available. The upstream changelog fixes an SSE4.1 intrinsic in the SSE2 RNG/legacy backend. Only this package and checksum changed; no claim of a newly discovered application exploit. |
| `glib` 0.18.5, [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) | Requires upstream-compatible dependency migration | Linux Tauri/Wry/GTK/WebKit chains require the 0.18 generation. The advisory fixes `VariantStrIter` in >=0.20, a different compatible-version range; adding a newer parallel GLib cannot repair the existing copy. Keep the warning visible and re-evaluate with framework upgrades. Absence of a direct application call is not proof of unreachability. |
| `proc-macro-error` 1.0.4, [RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html) | Allowed but tracked maintenance warning | Linux `glib-macros` and `gtk3-macros` build dependencies. No patched release is listed; follow the framework's macro migration. This is a build-time maintenance concern, not evidence of a runtime exploit. |
| `unic-char-property` 0.9.0, [RUSTSEC-2025-0081](https://rustsec.org/advisories/RUSTSEC-2025-0081.html) | Allowed but tracked maintenance warning | Tauri utilities → `urlpattern 0.3.0` → `unic-ucd-ident`. Requires the upstream Unicode dependency migration; no patched release is listed. |
| `unic-char-range` 0.9.0, [RUSTSEC-2025-0075](https://rustsec.org/advisories/RUSTSEC-2025-0075.html) | Allowed but tracked maintenance warning | Same `urlpattern` Unicode chain; no patched release is listed. |
| `unic-common` 0.9.0, [RUSTSEC-2025-0080](https://rustsec.org/advisories/RUSTSEC-2025-0080.html) | Allowed but tracked maintenance warning | Same chain through `unic-ucd-version`; no patched release is listed. |
| `unic-ucd-ident` 0.9.0, [RUSTSEC-2025-0100](https://rustsec.org/advisories/RUSTSEC-2025-0100.html) | Allowed but tracked maintenance warning | `urlpattern` dependency; no patched release is listed. |
| `unic-ucd-version` 0.9.0, [RUSTSEC-2025-0098](https://rustsec.org/advisories/RUSTSEC-2025-0098.html) | Allowed but tracked maintenance warning | Same Unicode chain; no patched release is listed. |

The existing CI continues to run the official audit without ignored advisory IDs.
Review this table when dependencies or advisories change and before each release.
The registry metadata and upstream patch are available at
[crates.io](https://crates.io/crates/chacha20/0.10.2) and
[RustCrypto change 580](https://github.com/RustCrypto/stream-ciphers/pull/580).

## Isolated storage failure qualification

- [Hosted Linux run 36493274324](https://github.com/Moresyl/metaclean/actions/runs/36493274324)
  passed on test commit `d4f007e`, without changing production cleanup code.
  A disposable 16 MiB tmpfs was filled until a write probe returned `ENOSPC`
  (errno 28), then emptied and remounted read-only; the same probe returned
  `EROFS` (errno 30) while the source file retained writable permission bits.
- Scanning remained successful. Both copy and replace cleanup modes failed with
  an error, without claiming an output, backup or integrity result. Source bytes
  remained identical and directory snapshots showed no temporary files, partial
  outputs or backup fragments. The uploaded evidence includes mount details,
  native test output and source SHA-256.
- This qualifies two actual filesystem failure paths using a synthetic text
  fixture. It does not simulate physical media failures, slow disks, power loss,
  or exhaustion occurring after a replacement backup has already been committed.

## Automated quality gates

- v0.11.2 [candidate CI](https://github.com/Moresyl/metaclean/actions/runs/36477273684)
  passed all quality gates and Windows/macOS/Linux desktop E2E. Frontend: 418 tests,
  statements 89.62%, branches 85.09%, functions 91.92%, lines 93.40%. Windows native:
  225 passed, five explicitly gated tests ignored; Rust lines 92.27% with the CI
  exclusion expression. A separate local Windows desktop run also passed 15 cases.
- v0.11.2 [release pipeline](https://github.com/Moresyl/metaclean/actions/runs/36479381902)
  passed Linux source revalidation, all five platform builds and applicable package
  smoke checks before final publication. Linux: 218 native tests passed, five ignored,
  92.23% Rust line coverage, 15 desktop cases and ten independent FFmpeg PCM checks.
  Platform-specific tests account for the Windows/Linux test-count difference.
  Cargo audit completed with the eight previously documented upstream warnings.

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

[Run 36473320724](https://github.com/Moresyl/metaclean/actions/runs/36473320724)
passed with both `verify_storage=true` and `in_app_update=true`. On x64 and x86,
the test clicked the real 0.11.0 application's **Install update** button only
after its dialog offered exactly 0.11.1. The normal frontend/native updater
used the public signed feed, downloaded and installed the update, exited the old
process and automatically restarted the new executable. The test verified the
new version, live restarted window and preserved synthetic storage before any
manual relaunch. Subsequent manual downgrade and final uninstall also passed.
Logs explicitly distinguish this application-triggered update from the earlier
installer-only runs. No update endpoint or package signature check was mocked.

`scripts/verify-windows-upgrade.ps1` refuses non-hosted environments and existing
MetaClean installations/processes. The local preexisting 0.9.0 MSI installation
was not modified. This verifies x64/x86 NSIS installer transitions and launchability;
the application-triggered signed-update path is additionally covered by run
36473320724. Arbitrary historical user-data migration and interrupted installation
recovery remain unverified. Separate
MSI, Linux and macOS package-transition evidence follows below.

## Published Windows MSI upgrade — 2026-09-29

[Run 36475350834](https://github.com/Moresyl/metaclean/actions/runs/36475350834)
passed on a disposable Windows runner using the public x64 MSI packages and
published SHA-256 manifests. Installation of 0.11.0, upgrade to 0.11.1 and manual
downgrade to 0.11.0 each produced exactly one MSI registration with the expected
version, matching executable version and live MetaClean window after six seconds.
Final uninstall removed both the application executable and registration.
Downloaded JSON records the expected distinct product codes and their return to
the original value on downgrade; verbose installer logs accompany the artifact.

[Run 36475617820](https://github.com/Moresyl/metaclean/actions/runs/36475617820)
repeated that sequence and added repair after upgrade. It overwrote only the
test installation's executable with four invalid bytes, then ran the public
0.11.1 MSI with `/fa`. Repair restored the executable's exact original SHA-256,
preserved the sole expected product registration and launched a live window for
six seconds. The downloaded current-version entry records `repairVerified: true`;
subsequent downgrade and removal passed. This demonstrates recovery from that
executable-file corruption, not interrupted installation or filesystem failure.

The test refuses non-hosted environments and preexisting installations/processes.
It does not cover MSI/NSIS cross-installer migration, user-data preservation or
power loss during installation. The local preexisting 0.9.0 installation was not
modified.

## Published Linux DEB upgrade — 2026-09-29

[Run 36473955463](https://github.com/Moresyl/metaclean/actions/runs/36473955463)
passed on Ubuntu 22.04 amd64: public DEB checksum and package identity checks,
0.11.0 installation, in-place upgrade to 0.11.1, explicit package-manager
downgrade to 0.11.0 and removal. Each version matched `dpkg-query` and stayed
alive for the eight-second Xvfb smoke window. Final removal left neither the
executable nor an installed package registration. The `linux-upgrade-evidence`
artifact records the three observed versions and timeout statuses.

The first run stopped before installation because the new test used an incorrect
package name; it was corrected to the published `meta-clean` identity. This
verification covers DEB transitions and process liveness, not WebView data
preservation, AppImage self-update, RPM transitions or interrupted installation.

## Published macOS DMG replacement — 2026-09-29

[Run 36474805222](https://github.com/Moresyl/metaclean/actions/runs/36474805222)
passed independently for the public aarch64 and x64 DMGs. Both jobs checked
release status and published SHA-256 values, copied each application into an
owned temporary directory, then replaced 0.11.0 with 0.11.1 and manually rolled
back to 0.11.0. Each step checked the installed bundle version, identifier
`com.moresl.metaclean`, exact executable architecture and six-second process
liveness. Final removal deleted the temporary application; mounted images were
detached. Both downloaded evidence artifacts record all three matching versions.

The runner image was macOS 26 ARM64 for both jobs. The x64 package therefore
passed through the host's Intel compatibility environment, not on native Intel
hardware. This is manual bundle replacement evidence, not application-triggered
update, WebView data preservation, interrupted replacement recovery, Gatekeeper
acceptance or Apple signing/notarization verification. No existing `/Applications`
installation was replaced.

## Large-text native qualification — 2026-09-29

The explicit ignored test `engine::tests::benchmark_large_text_budget_boundary`
creates 64 MiB and exactly 256 MiB UTF-8 files, each containing one removable
zero-width character. It runs the real scan and copy-clean paths, streams an
independent source hash comparison, checks every output byte and verifies both
audit hashes. A 256 MiB + 1 byte input must fail scanning and cleaning without
creating any output or changing its length. Fixtures live only in a temporary
directory. This benchmark is included in `pnpm test:benchmark`.

On the same Windows 11 / i7-12700 machine as the earlier native memory study,
three fresh release-test processes per implementation produced these observations:

| Observation | Published 0.11.1 text algorithm | Streaming context and span-copy implementation |
| --- | --- | --- |
| Peak working set observed, entire process | 1,887,068,160–1,887,178,752 bytes | 1,080,926,208–1,080,942,592 bytes |
| Sampled peak private memory | 2,289,094,656–2,289,115,136 bytes | 1,078,112,256–1,078,124,544 bytes |
| 64 MiB scan | 672.23–689.05 ms | 486.51–498.69 ms |
| 64 MiB copy-clean | 878.47–890.62 ms | 709.99–742.34 ms |
| 256 MiB scan | 4,387.39–4,840.48 ms | 2,529.23–2,606.20 ms |
| 256 MiB copy-clean | 6,107.56–6,268.48 ms | 4,696.38–5,053.39 ms |

The working-set observation fell by approximately 43% for this fixture sequence.
The monitor requested five-millisecond sampling and used the OS peak-working-set
counter; private bytes are sampled, and the unobserved final interval means these
are not strict upper bounds. Measurements include fixture generation, hashing
and verification, but exclude compilation, the monitor and desktop WebView. The
baseline and final measurements ran without concurrent local test/build tasks.
They do not qualify slow storage, cold caches, concurrent large scans, arbitrary
Unicode mixtures, every file format or complete application memory usage.

A separate deterministic differential harness compared old and new outputs and
finding fields for 108,921 three-character/random cases plus empty, single and
two-character inputs. Permanent regressions cover original-neighbor semantics,
nested directional embeddings/overrides and malformed flag-tag sequences. The
optimization does not alter the 256 MiB input limit or Unicode preservation policy.
The optimization is included in v0.11.2; v0.11.1 is the measured baseline.

## Published v0.11.2 verification — 2026-09-29

The [stable release](https://github.com/Moresyl/metaclean/releases/tag/v0.11.2)
was published at 2026-09-28 20:48:36 UTC from
`ae197feac7c90cd8a5b106657265abbe1fc071fe`, with 20 nonempty assets and neither
draft nor prerelease status. Independent
[run 36481920205](https://github.com/Moresyl/metaclean/actions/runs/36481920205)
downloaded the public assets, verified all 19 SHA-256-listed files, validated
all five updater signatures against the tagged public key, and rejected modified
package bytes. The release and Pages `latest.json` endpoints both returned 0.11.2
with exactly matching download URLs and signatures for all five platforms; the
[feed deployment](https://github.com/Moresyl/metaclean/actions/runs/36481821010)
also completed successfully.

The subsequent [installed-update run 36482289761](https://github.com/Moresyl/metaclean/actions/runs/36482289761)
passed independently for Windows x64 and x86 NSIS packages. The installed 0.11.1
application clicked its real update button for 0.11.2, downloaded the public signed
update and automatically restarted. Both jobs verified exact synthetic locale,
theme, output-mode and history/fingerprint values in the restarted WebView, then
manually downgraded to 0.11.1 with those values retained and uninstalled cleanly.
Before upgrade, rejection of a truncated 0.11.2 installer preserved the old
executable hash, registration and launchability. Downloaded artifacts record the
three observed versions and `storageVerified: true` throughout; logs independently
confirm application-triggered update and restart on both architectures. The x86
package ran on 64-bit Windows. This qualifies this NSIS version pair and synthetic
data, not arbitrary historical migrations, power-loss recovery or Linux/macOS
application-triggered updates to 0.11.2.

Additional public-package checks qualified the 0.11.1 → 0.11.2 → 0.11.1 pair:

- [MSI run 36482694133](https://github.com/Moresyl/metaclean/actions/runs/36482694133)
  verified sole product registration, executable versions, six-second windows,
  upgrade/downgrade and removal. Repair of the deliberately corrupted 0.11.2
  executable restored its exact original hash, registration and launchability.
- [DEB run 36482701851](https://github.com/Moresyl/metaclean/actions/runs/36482701851)
  verified package-manager versions, eight-second process liveness for each
  transition and clean removal on Ubuntu 22.04 amd64.
- [DMG run 36482709218](https://github.com/Moresyl/metaclean/actions/runs/36482709218)
  verified manual temporary-bundle replacement and rollback for ARM and Intel
  packages, checking bundle identity, versions, architectures, six-second process
  liveness and removal. Both hosts were ARM64; Intel used compatibility support.

Downloaded artifacts and logs confirmed all three observed versions in each job.
These additional runs do not assert MSI/DEB/DMG user-data preservation, Linux/macOS
self-update, native Intel Mac execution, Gatekeeper acceptance or interrupted
installation recovery.

## Released cleanup interruption regression — 2026-09-29

The new hosted-only `verify-crash-recovery.yml` workflow tested the public v0.11.2
x64 portable package using 64 synthetic 8 MiB text files. It invoked real native
drag-and-drop intake, clicked the real scan/confirm controls, observed an active
count-only recovery marker and at least one committed output, then forcibly
terminated the owned application process tree before the batch finished.

[Run 36483534933](https://github.com/Moresyl/metaclean/actions/runs/36483534933)
and the diagnostic [run 36483828672](https://github.com/Moresyl/metaclean/actions/runs/36483828672)
both failed the restart-notice assertion. Before restart, all source hashes and
the complete committed-output hashes passed. The second run's downloaded
`recover.json` confirmed an empty queue, retained English locale and no recovery
notice; `before-crash.json` showed the marker existed during cleanup. This
reproduces a missing-notice defect under forced termination. The WebView-only
marker is not sufficient evidence of durable crash recovery. A native durable
recovery record and an equivalent end-to-end passing regression were required;
no such fix is included in v0.11.2. These failed runs do not qualify power-loss or
installer-interruption recovery.

## Native interruption recovery candidate — 2026-09-29

The v0.11.3 candidate at `659dabc7243ca72368fcc722bea55dbacbd3fb50` passed
[run 36486936517](https://github.com/Moresyl/metaclean/actions/runs/36486936517).
It built the normal release executable without desktop test plugins and repeated
the same real 64-file cleanup interruption. All 64 source hashes and both committed
output hashes matched the independent expected bytes. No extra fixture files
remained. The first restart showed the native interruption notice despite a null
WebView marker; its empty queue and disabled action required explicit re-import.
The second restart did not repeat the notice. Neither restart changed any fixture.
The evidence identifies candidate executable SHA-256
`f3d76c7f122b558db3ab65ed0b468bafe28f6efb0a29875be4fa2f2ed9d628b7`.

An earlier candidate attempt, run 36485641334, failed to establish its debug
connection before any cleanup. The diagnostic rerun explicitly supplied the
WebView debug argument and allowed a longer startup window; it does not establish
which startup condition caused the earlier connection failure.

The native fix writes and flushes a path-free record before modifying files,
locks it for the batch lifetime, and removes it on normal completion. Startup
consumes only abandoned records and never resumes operations. Its
[CI run 36485613074](https://github.com/Moresyl/metaclean/actions/runs/36485613074)
passed 424 frontend tests, 230 Windows native tests (5 ignored), and 15 desktop
tests on each of Windows, Linux and macOS. Rust line coverage was 92.25% with the
documented CI exclusions; `recovery.rs` reached 90.98%. npm audit reported no known
vulnerabilities after the development-toolchain `ip-address` 10.5.1 pin; the eight
previously documented upstream Rust warnings remain. Local strict Clippy and the
rebuilt Windows desktop suite also passed.

This section records candidate evidence; public-package checks follow below. The forced
termination scenario is Windows x64 safe-copy cleanup; it does not qualify
power loss, interrupted installation or all operating systems and output modes.

## Published v0.11.3 verification — 2026-09-29

[Release 36488240740](https://github.com/Moresyl/metaclean/actions/runs/36488240740)
passed source validation, all five platform builds, applicable package smoke
checks and finalization. The [stable release](https://github.com/Moresyl/metaclean/releases/tag/v0.11.3)
at `f8b95987d4ffce164e5ca26d3bd4acc73822437a` contains 20 nonempty assets.
Source validation passed 424 frontend tests, 224 Linux native tests (5 ignored),
15 Linux desktop tests and ten independent FFmpeg PCM comparisons. Rust line
coverage was 92.27% with the documented exclusions; the recovery module reached
94.96% on Linux. The eight known upstream Rust audit warnings remain.

[Independent verification 36490447072](https://github.com/Moresyl/metaclean/actions/runs/36490447072)
downloaded the public packages, checked all 19 checksummed files, verified all
five updater signatures against the tagged key and rejected modified bytes.
The release and Pages update manifests both returned 0.11.3 and matched exactly
across all five platforms after deployment 36490377849.

[Public crash regression 36490451100](https://github.com/Moresyl/metaclean/actions/runs/36490451100)
passed the same 64-file scenario using the downloaded x64 portable package.
All sources and the one committed output retained their expected hashes; there
were no additional fixture files. The first restart displayed the interruption
notice, the second cleared it, and neither restart resumed file operations.
The tested public executable SHA-256 was
`ab8af37d734d2bd405aeace48c7a82be5ac431d60cc94f50c4e0c1f29cc9ed3f`.

[Windows upgrade 36490455936](https://github.com/Moresyl/metaclean/actions/runs/36490455936)
passed x64 and x86 NSIS application-triggered signed updates from 0.11.2 to
0.11.3, automatic restart, synthetic settings/history/hash preservation, manual
downgrade to 0.11.2 and uninstall. Truncated-installer rejection preserved the
old executable hash, registration and launch. The x86 package ran on a 64-bit
Windows host; this does not qualify a native 32-bit OS or arbitrary user data.

Additional public-package checks passed the 0.11.2 → 0.11.3 → 0.11.2 pair:

- [MSI 36490896570](https://github.com/Moresyl/metaclean/actions/runs/36490896570)
  verified registration, executable versions, launch, repair, downgrade and
  removal. Repair restored the exact executable hash after deliberate damage.
- [DEB 36490900879](https://github.com/Moresyl/metaclean/actions/runs/36490900879)
  verified installation, package versions, live launch windows, downgrade and
  removal on the hosted Ubuntu x64 runner.
- [DMG 36490904420](https://github.com/Moresyl/metaclean/actions/runs/36490904420)
  verified copied-app replacement, versions, bundle identity, six-second launches,
  rollback and removal for arm64 and x86_64 packages on arm64 hosts.

These MSI/DEB/DMG transitions do not establish arbitrary-data migration, in-app
updates for those formats, native Intel Mac qualification or Gatekeeper approval.

Post-tag test-only commit `313dc70` adds an observed native drag-enter response
before its single synthetic drop event. This addresses a plausible registration
race exposed by an earlier macOS queue-intake timeout, without retrying drops or
weakening output checks. [CI 36488873692](https://github.com/Moresyl/metaclean/actions/runs/36488873692)
passed all three 15-test desktop suites. It does not change the released product.

## Published desktop memory observation — 2026-09-29

Historical evidence: this run used the older PID-only ancestry sampler. Its
process ownership cannot be revalidated from the saved records. Use the v0.11.7
identity-checked observations above for current process-tree evidence, and do
not use this older table as a qualified before/after memory comparison.

[Run 36491998506](https://github.com/Moresyl/metaclean/actions/runs/36491998506)
completed six fresh Windows x64 application runs against the checksummed public
v0.11.3 portable package: three each with a 64 MiB and 256 MiB synthetic text
file containing one zero-width character. Real native drag/drop, scan and
confirmation controls drove cleanup. Every source hash, complete expected
output hash and stored audit fingerprint matched; no extra fixture files remained.
The shared UI driver's existing crash scenario also passed again in
[run 36492003924](https://github.com/Moresyl/metaclean/actions/runs/36492003924).

The sampler observed the owned application and its descendants, including the
WebView (seven processes), excluding the fixture generator and Node controller.
It enumerated the process tree between requested 200 ms waits; enumeration adds
overhead, so this is not a fixed-frequency trace. Runs used Windows Server 2025
Datacenter hosts with different AMD EPYC and Intel Xeon CPUs. The table gives
the minimum and maximum across the three observations for each input size.

| Input | UI scan time | UI cleanup time | Maximum sampled sum of working sets | Maximum sampled sum of private bytes |
| --- | --- | --- | --- | --- |
| 64 MiB | 0.52–1.04 s | 1.08–1.56 s | 503.79–571.55 MiB | 332.92–336.55 MiB |
| 256 MiB | 2.55–3.71 s | 4.68–4.73 s | 887.26–1,243.54 MiB | 911.42–1,168.73 MiB |

Timing includes UI polling; memory sampling covers startup through completed
cleanup. Working-set sums may count shared pages more than once, and private
bytes measure committed private memory rather than resident pages. Samples can
miss short peaks. These figures are workload-specific observations, not a memory
upper bound, a minimum-RAM recommendation or a before/after speed comparison.
They add full-process-tree evidence to the earlier native-only measurements;
slow disks, other operating systems, complex documents and mixed Unicode
workloads still need separate qualification. No product binary changed.

## Remaining external release gates

1. Test representative documents in genuine Microsoft Word and newer WPS builds, including layout, complex objects and supported OpenDocument interoperability. The limited WPS 2019 OOXML semantic round trip above and prior LibreOffice 26.2.5 samples do not close that broader qualification.
2. Provide Apple Developer signing/notarization credentials and verify both DMGs with Gatekeeper.
