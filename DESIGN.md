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
| Geometry | 12px panels, 6px controls, a 24px intake surface, 1px hairlines, shared 28/32/36px buttons and 32px fields |
| Type | System UI font stack, 14px body with 1.5 line height, 500/600 weights; Microsoft YaHei UI, PingFang SC and Noto Sans SC fallbacks |
| Checkbox | 18px square, 4px corner, neutral checked and indeterminate states, 2px focus outline, distinct disabled combinations |
| Motion | 150ms control feedback; reduced-motion mode collapses animations and transitions to 0.001ms |

Supported renderers apply a 1.25 corner scale with `superellipse(1.5)` to
shared panel/control shapes. Other renderers use the unscaled radius. The
checkbox keeps its explicit 4px corner in both cases. Font fallbacks are
implementation choices; a declared stack alone does not prove which font
renders a Chinese glyph on a particular machine.

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
- Confirmation and update overlays render at the document root. Animated page
  containers cannot redefine their fixed-position bounds or clip their actions.
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
