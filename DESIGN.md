# MetaClean design reference

## Source

- Inspiration list: `C:\Users\18468\Desktop\wcb.txt`
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
| Canvas | Near-black `#0b0f14` with a light-mode counterpart; no gradient behind core content |
| Surface | `#121820` cards and chrome, `#1a212b` raised controls |
| Accent | Mint `#46d9a2` for the one primary action and success state |
| Text | `#e8edef` primary, `#a3aeb6` secondary, `#78848d` placeholder/disabled |
| Geometry | 8px panels, 5px controls, 1px hairlines, 28–32px control heights |
| Type | Segoe UI Variable with CJK fallbacks; display face only for headings |
| Motion | Short rise/pop transitions; reduced-motion mode collapses them to 1ms |

## Implemented patterns

- `Sidebar` is the persistent navigation pattern: five primary destinations,
  an explicit `aria-current` state and keyboard accelerators `Ctrl/Cmd+1…5`.
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
