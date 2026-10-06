# MetaClean desktop design system

## Design basis

- The updated desktop design library supplied by the user was reviewed on 2026-10-06.
- Current theme declarations, selector scopes and control rules take precedence over older summaries.
- Source locations, comparison material and research evidence stay in the ignored local research directory. Product code uses independent components and semantic tokens.

## Design direction

The whole application shares the supplied desktop design language: quiet neutral
surfaces, compact controls, short feedback, consistent typography and a persistent
workspace sidebar. File cleaning, history, privacy, settings, support and overlays
consume the same tokens. Their content remains specific to a local file utility.

## MetaClean tokens

| Role | Current rule |
| --- | --- |
| Canvas | Dark `#181818` or light `#ffffff` content beside `#000000` / `#f9f9f9` workspace chrome |
| Surface | Neutral `#212121` / `#ffffff` panels and `#303030` / `#ededed` raised controls |
| Brand and status | Neutral primary actions and checked controls; blue focus; green, orange and red for semantic status |
| Text | Dark `#dfdfdf`, light `#1a1c1f`; secondary ink resolves from 70% foreground opacity |
| Geometry | 12px panels, 6px shared control corners, 8px native-select base corners, a 24px intake surface, 1px hairlines and shared 28/32/36px pill actions |
| Type | System UI font stack, 14px body at 430 weight with 1.5 line height, 13px small text and 500/600 emphasis; platform sans-serif and monospace fallbacks |
| Checkbox | 18px square, 4px base corner, theme-specific neutral borders and disabled combinations, stable selected hover and a 2px focus outline |
| Actions | 400-weight labels with 1em line height; 13/14/13px type for 28/32/36px heights, 18px icons, default arrow cursor and separate surface feedback; icon-only actions retain 24/28px squares |
| Selects | 32px height, 12px gutters, 500-weight 13px labels with 24px line height; transparent surface, 1px inset border, an 8×12px direction indicator and a 2px inset focus ring |
| Select pickers | Supporting scalar controls use a trigger-width panel, 4px gutter, opaque canvas surface, 16px base ordinary corner, 5px/8px rows, 13px/430 labels and a leading 16px checkmark |
| Queue search | 32px height, 8px base ordinary corner, 12px gutters, 400-weight 13px text with 19.5px line height, 8px adornment gap and a soft theme surface |
| Segmented choices | 32px track, 2px padding/gap, 8px base ordinary corner, 28px options, 12px gutters and 600-weight 13px labels; a neutral raised selected thumb |
| Context menus | 180px minimum width, 6px viewport margin, 4px gutter, opaque canvas surface, 16px panel/12px row base corners, 13px/430 text, 5px/8px row padding, 6px gaps, arrow cursor and 50% disabled opacity |
| Tooltips | Ordinary 300px width cap bounded by the viewport, 12px/16px gutters, scaled 8px round corners, 14px/400 text at 1.45 line height, themed elevated surfaces and shadows; 150ms hover delay, 5px anchor gap and 15px collision margins |
| Motion | 150ms control feedback; reduced-motion mode disables transitions and animations, showing entrance surfaces immediately |

Supported renderers apply a 1.25 corner scale with `superellipse(1.5)` to
shared panel/control shapes. Other renderers use the unscaled radius. The
checkbox scales its 4px base corner to 5px in supported renderers and keeps
ordinary rounded geometry. Its border, disabled fill and focus ring use
dedicated tokens rather than text-opacity tokens. UI and monospace stacks use
the supplied desktop platform defaults. Generic fallbacks remain platform
dependent; a declared stack alone does not prove which font renders a Chinese
glyph on a particular machine.

Actual Windows native WebView2 observations on 2026-10-07 cover the existing
Chinese/English page heading and two sidebar controls. Rendered-glyph records
identify Noto Sans SC for Chinese text and Segoe UI family faces for Latin text
and shortcuts. These are six localized product-node observations in an isolated
test build, separate from the earlier Chrome measurements; they do not identify
the original application's fonts or qualify every node and operating system.

Actions follow the current component's default pill configuration. Their surfaces
carry theme-specific normal, hover, pressed and disabled colors; labels and icons
remain stable. Outlined and ghost actions place the focus ring 1px inside the
edge, while primary and danger actions place it 2px outside. Danger uses its own
red focus ring. Pressed actions do not scale the content. Native button semantics,
form behavior, refs, names and expanded/disabled states remain intact.

Native selects use their independently verified ordinary-corner default rather
than inheriting the action pill configuration. Their 8px base corner scales to
10px where the shared corner-scaling rule is supported while keeping ordinary
rounded geometry. Disabled borders and ink remain distinct from hover, invalid
controls use semantic red, and the indicator follows the logical trailing edge.
Options, keyboard selection, required validity and form data belong to the real
native select; its decorative indicator adds no separate focus target.

Where `appearance: base-select` and `::picker(select)` are supported, scalar
selects use a styled native top-layer picker. Its geometry, option feedback,
selected weight and checkmark position follow the supplied select rules. The
surface reuses the separately verified opaque desktop-menu rule: a translucent
surface in the native picker allowed underlying text to show through during
visual review. This is an independent composition of supplied desktop rules,
not a claim that every original select call site has the same surface.
The panel follows the trigger width, scrolls within the viewport and uses
300ms entering and 200ms exiting curves. Native keyboard navigation, typeahead,
disabled-option skipping, focus and form commits remain browser behavior.
Forced-color mode uses system canvas/highlight colors; reduced motion disables
the transitions. Unsupported renderers, multiple selects and listboxes retain
their existing native presentation and require separate visual qualification.

Queue search uses the supplied input's soft variant. Focus draws a 1px inset
border at 20% foreground opacity; invalid input uses semantic red. Disabled
input dims the complete control to 50% and suppresses focus/error borders.
The native input retains its search behavior and has no separate focus outline;
forced-color mode restores a system-colored container boundary and focus ring.

Settings categories and theme choices share one segmented control. The selected
label uses the main ink; inactive labels use 65% of the same ink in sRGB and
receive emphasis on hover or keyboard focus. Disabled options retain 50% opacity.
The selected
thumb follows the actual option bounds and remeasures when the track or labels
resize. Selection remains visible in an overflowing track, including RTL.
Native buttons expose their pressed state; a single keyboard entry point,
non-wrapping arrows, Home/End and disabled-option skipping preserve navigation.
Focus movement alone does not change the preference. Thumb movement uses a
300ms entering curve; reduced-motion and forced-color rules remain active.

The current desktop startup selects its desktop window scope. Earlier fixtures
used a different scope, which changed small-text sizing, body weight, action
cursors and shared menu variables. Current rules use the verified desktop
declarations; native selects and small/large actions resolve their shared control
type size to 13px in that scope. The file context-menu trigger and shared wrapper
have now been traced: the source desktop branch prefers a system menu, while its
web fallback uses a 180px minimum width and 6px collision padding. Our web menu
adopts that fallback geometry and shared desktop pointer/disabled states; native
system-menu rendering remains platform-specific.

Context menus use one keyboard entry point and unique active-descendant IDs.
Arrows skip disabled items, Home/End reach enabled boundaries and Tab/Escape
close the flyout. Closing returns focus without scrolling when focus is still
inside the menu; an independently focused control keeps its focus. Theme and
forced-color rules preserve the active command and destructive-action meaning.

## Implemented patterns

- The empty cleaning workspace has one centered file-entry area. The queue
  appears only after importing files, so an empty queue does not duplicate the
  initial instructions. Settings use divided rows instead of nested cards.
- Cleanup preferences sit behind one vertical divider, without an outer card.
  Shared 28px intake/page headings and 500/600 weights keep
  the hierarchy consistent across scripts. File-type glyphs explain
  intake scope. Queue search and filters expose their batch-action scope.
- `Sidebar` is a 264px persistent workspace panel that collapses to a 64px icon
  rail. It keeps five destinations, an explicit `aria-current` state,
  `Ctrl/Cmd+1…5` navigation and `Ctrl/Cmd+B` collapse control.
- At compact viewport widths the workspace uses a 64px navigation rail with
  visually hidden accessible labels. Cleanup options move below file intake,
  settings categories scroll horizontally, and modal content stays within the
  viewport with its own scroll area. These rules also support desktop zoom.
- Cleaning layout follows the content container's width, including sidebar and
  zoom constraints. Stacked intake keeps its content height; queued files occupy
  a bounded 260–360px panel with wrapped toolbar groups and an independently
  scrolling list. Options follow the complete intake/queue region.
- Confirmation, update and context-menu overlays render at the document root.
  Animated page containers cannot redefine their fixed-position bounds or clip
  their actions. Context menus measure unscaled layout bounds, keep a 6px
  viewport margin and focus without scrolling the workspace.
- The command panel uses a 520px width cap within 92% of the viewport. Its
  list is capped at 440px and shrinks with available height; the panel reserves
  16px vertical margins even in short viewports. Selection stays visible when
  the list or window resizes, with Home/End navigation and focus return intact.
  Mouse movement changes selection; rows entering beneath a stationary pointer
  during scrolling leave the keyboard selection intact.
- `TitleBar` aligns its identity area with the sidebar, reserves its center for
  command search and keeps native caption actions fixed to the right edge.
- `CleanOptions` owns the one commit action. The queue toolbar remains secondary
  and groups sorting, export and clear actions without competing with cleanup.
- `AboutPage` is the support footer pattern: runtime facts, bounded diagnostics,
  update status, issue/feature/release links and source/license links in one
  predictable surface.
- Empty and status states use a reserved region and `aria-live` announcements;
  long queues use `content-visibility: auto` to avoid painting rows outside the
  scrollport.
- Long cleanup batches report count-only progress in the local status strip;
  progress never exposes a file path or content value.
- Page-level chunks are loaded on demand. The first paint keeps the cleaning
  workflow small while History, Privacy, Settings and About remain independently
  cacheable.
- Styles scan the application source directory explicitly. Development watching
  excludes local research, coverage, native builds and generated documentation, so these
  artifacts cannot trigger unrelated page reloads or expand style scanning.
- Windows NSIS and WiX bundles reuse the same neutral light/dark surfaces,
  product mark and compact typography hierarchy. The native installers keep
  platform-standard controls while their welcome, progress, completion and
  uninstall surfaces remain visibly part of the same product. NSIS follows a
  Simplified Chinese Windows locale with an English fallback.
- The VitePress documentation uses the same neutral palette, 12px panel
  radius, semantic-only status colour and zero-tracking typography. It presents
  a current 1180 x 720 application capture without decorative gradients or
  blurred chrome.

Tooltips mount at the document root and measure their unscaled layout dimensions
before positioning. Escape dismisses visible and pending tips without consuming
the key; pointer exit, resize and detached controls cancel stale descriptions.
Each visible tip adds its own accessible description ID to the control and removes
only that ID on dismissal, preserving descriptions owned by other components.
Keyboard focus stays on the control. Reduced motion and forced system colors
remain available.

## Native documentation captures

Run `pnpm test:e2e:build`, then `pnpm docs:capture` to capture English and Chinese
workflows at 1180 × 720. The opt-in capture uses synthetic files in a new temporary
directory, drives native intake and cleanup, checks that the original Unicode
traces survive while cleaned copies omit them, restores saved preferences and
removes its temporary files. It never processes user documents. With Pillow
installed, `python scripts/build-doc-gifs.py` generates finite-loop GIFs from
those unmodified captures. Each README uses only its matching language.

## Guardrails

Do not add third-party imagery, raw metadata previews, decorative gradients over
the file workflow, generic `transition: all`, unlabeled icon buttons or button
controls without accessible names. Any new format or destructive write must clear the
native bounded-parser, candidate re-inspection and atomic-write tests described
in `SUPPORT_POLICY.md`.

## Rerun inputs

```text
pnpm test:coverage
pnpm test:e2e
pnpm docs:capture
pnpm test:docs
pnpm docs:build
```
