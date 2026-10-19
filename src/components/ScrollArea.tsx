import { useLayoutEffect, useRef, useState, type ReactNode } from "react";

/** Edge cues disappear when the corresponding end of the content is visible. */
export default function ScrollArea({ children, label, className = "" }: {
  children: ReactNode; label: string; className?: string;
}) {
  const viewport = useRef<HTMLDivElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const [edges, setEdges] = useState({ top: false, bottom: false });
  const measure = () => {
    const element = viewport.current;
    if (!element) return;
    const maximum = Math.max(0, element.scrollHeight - element.clientHeight);
    const position = Math.max(0, Math.min(maximum, element.scrollTop));
    const next = { top: position > 1, bottom: maximum - position > 1 };
    setEdges(previous => previous.top === next.top && previous.bottom === next.bottom ? previous : next);
  };
  useLayoutEffect(() => {
    measure();
    const observer = typeof ResizeObserver === "undefined" ? undefined : new ResizeObserver(measure);
    if (viewport.current) observer?.observe(viewport.current);
    if (content.current) observer?.observe(content.current);
    window.addEventListener("resize", measure);
    return () => { observer?.disconnect(); window.removeEventListener("resize", measure); };
  }, [children]);
  return <div className="relative min-h-0">
    <div ref={viewport} role="region" aria-label={label} tabIndex={0}
      className={`overflow-y-auto overscroll-contain ${className}`} onScroll={measure}>
      <div ref={content}>{children}</div>
    </div>
    {edges.top ? <div aria-hidden="true" data-scroll-edge="top" className="pointer-events-none absolute inset-x-0 top-0 h-3 bg-gradient-to-b from-surface to-transparent forced-colors:hidden" /> : null}
    {edges.bottom ? <div aria-hidden="true" data-scroll-edge="bottom" className="pointer-events-none absolute inset-x-0 bottom-0 h-3 bg-gradient-to-t from-surface to-transparent forced-colors:hidden" /> : null}
  </div>;
}
