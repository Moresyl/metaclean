/**
 * The light on a panel surface.
 *
 * One static wash of the accent and a film of noise, both at the edge of
 * visibility. Neither is decoration for its own sake: a 64px rail and a
 * half-window drop target are large flat fills, and a large flat fill in a
 * webview reads as painted cardboard next to the real chrome around it. This is
 * the cheapest thing that stops it — pure paint, no blur filter, no layout, so
 * it costs one composited layer and nothing per frame.
 *
 * The noise is an inline SVG turbulence rather than an asset because it has to
 * survive `img-src 'self' … data:` without adding a file to the bundle.
 */

const NOISE =
  "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E\")";

export default function Ambient({ className = "" }: { className?: string }) {
  return (
    <div aria-hidden="true" className={`pointer-events-none absolute inset-0 overflow-hidden ${className}`}>
      <div
        className="absolute -top-40 -left-24 size-[26rem] rounded-full opacity-[0.12]"
        style={{ background: "radial-gradient(circle, var(--color-brand) 0%, transparent 68%)" }}
      />
      <div className="absolute inset-0 opacity-[0.02] mix-blend-overlay" style={{ backgroundImage: NOISE }} />
    </div>
  );
}
