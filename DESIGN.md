# MetaClean design reference

## Source

- Inspiration list: the local `wcb.txt` supplied during the design review; its absolute workstation path is intentionally not published.
- Captures: [Navbar Gallery](https://navbar.gallery) and [CTA.gallery](https://cta.gallery), captured 2026-09-12 with Firecrawl.
- Local capture artifacts are kept in the ignored `.firecrawl/` directory for review; third-party logos, images and copy are not shipped with MetaClean.

## Design direction

MetaClean is a desktop utility, so the reference sites are used for interaction
principles rather than visual copying:

- Navbar Gallery: a compact, persistent navigation rail with a clear active
  state and a discoverable collection structure.
- CTA.gallery: a single dominant action, restrained secondary actions and
  category-driven grouping that lets users scan before committing.
- The remaining links in `wcb.txt` are a pattern library for hero hierarchy,
  footer grouping, bento-style information cards, empty states and responsive
  rhythm; they are references only, not source assets.

## MetaClean tokens

| Role | Current rule |
| --- | --- |
| Canvas | Dark `#202020` or light `#ffffff` content on a quieter `#161616` / `#f0f0f0` workspace frame |
| Surface | Neutral `#2b2b2b` / `#ffffff` cards and `#363636` / `#f0f0f0` raised controls |
| Brand and status | Neutral brand ink for selection; green, orange and red only for semantic status |
| Text | `#e8e8e8` / `#292929` primary with distinct secondary and disabled levels |
| Geometry | 8px panels, 5px controls, 1px hairlines, 28–32px control heights |
| Type | 14px Segoe UI Variable base with Microsoft YaHei UI, PingFang SC and Noto Sans SC fallbacks |
| Motion | 100–200ms colour, opacity and geometry transitions; reduced-motion mode collapses them to 1ms |

## Implemented patterns

- `Sidebar` is a 264px persistent workspace panel that collapses to a 64px icon
  rail. It keeps five destinations, an explicit `aria-current` state,
  `Ctrl/Cmd+1…5` navigation and `Ctrl/Cmd+B` collapse control.
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
- Windows NSIS and WiX bundles reuse the same neutral light/dark surfaces,
  product mark and compact typography hierarchy. The native installers keep
  platform-standard controls while their welcome, progress, completion and
  uninstall surfaces remain visibly part of the same product. NSIS follows a
  Simplified Chinese Windows locale with an English fallback.
- The VitePress documentation uses the same neutral palette, 8px maximum panel
  radius, semantic-only status colour and zero-tracking typography. It presents
  a current 1180 x 720 application capture without decorative gradients or
  blurred chrome.

## Guardrails

Do not add third-party imagery, raw metadata previews, decorative gradients over
the file workflow, generic `transition: all`, unlabeled icon buttons or button
elements for navigation. Any new format or destructive write must clear the
native bounded-parser, candidate re-inspection and atomic-write tests described
in `SUPPORT_POLICY.md`.

## Rerun inputs

```text
workflow: firecrawl-website-design-clone
source_urls: https://navbar.gallery, https://cta.gallery
target_stack: Tauri 2 + React 19 + Vite + Tailwind CSS 4
output: DESIGN.md
```
