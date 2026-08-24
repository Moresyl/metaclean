import { useEffect, useLayoutEffect, useRef, useState } from "react";

/**
 * The one tooltip the window ever shows.
 *
 * Rather than wrapping every control, this listens at the document and reads
 * `data-tip` off whatever the pointer or the focus ring lands on. Any element
 * anywhere in the tree gets a tooltip by carrying the attribute, and there is
 * never more than one on screen — which is the part a per-control component
 * gets wrong the moment the pointer crosses two of them quickly.
 *
 * It is positioned in script rather than CSS so it can flip above a control near
 * the bottom edge instead of being clipped by the window.
 */

/** What Windows waits before showing a tooltip the pointer is resting on. */
const HOVER_DELAY = 420;
/** Keyboard focus is deliberate, so its tooltip arrives almost at once. */
const FOCUS_DELAY = 90;
const GAP = 6;
const MARGIN = 8;

interface Tip {
  text: string;
  rect: DOMRect;
  viaKeyboard: boolean;
}

function tipFor(target: EventTarget | null): HTMLElement | null {
  return target instanceof Element ? target.closest<HTMLElement>("[data-tip]") : null;
}

export default function TooltipHost() {
  const [tip, setTip] = useState<Tip | null>(null);
  const [placement, setPlacement] = useState({ x: 0, y: 0, above: false });
  const surface = useRef<HTMLDivElement>(null);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => {
    const cancel = () => {
      window.clearTimeout(timer.current);
      setTip(null);
    };
    const schedule = (host: HTMLElement, viaKeyboard: boolean) => {
      const text = host.dataset.tip;
      if (!text) return;
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(
        () => setTip({ text, rect: host.getBoundingClientRect(), viaKeyboard }),
        viaKeyboard ? FOCUS_DELAY : HOVER_DELAY,
      );
    };

    const over = (event: PointerEvent) => {
      const host = tipFor(event.target);
      if (!host) { cancel(); return; }
      schedule(host, false);
    };
    // A tooltip that stayed up while its control was being used would cover the
    // thing that just changed, so any press takes it down.
    const focusIn = (event: FocusEvent) => {
      const host = tipFor(event.target);
      if (host && host.matches(":focus-visible")) schedule(host, true);
      else cancel();
    };

    document.addEventListener("pointerover", over);
    document.addEventListener("pointerdown", cancel);
    document.addEventListener("focusin", focusIn);
    document.addEventListener("focusout", cancel);
    window.addEventListener("blur", cancel);
    window.addEventListener("wheel", cancel, { passive: true });
    return () => {
      window.clearTimeout(timer.current);
      document.removeEventListener("pointerover", over);
      document.removeEventListener("pointerdown", cancel);
      document.removeEventListener("focusin", focusIn);
      document.removeEventListener("focusout", cancel);
      window.removeEventListener("blur", cancel);
      window.removeEventListener("wheel", cancel);
    };
  }, []);

  useLayoutEffect(() => {
    const element = surface.current;
    if (!tip || !element) return;
    const { width, height } = element.getBoundingClientRect();
    const below = tip.rect.bottom + GAP;
    const above = below + height > window.innerHeight - MARGIN;
    setPlacement({
      x: Math.min(
        Math.max(MARGIN, tip.rect.left + tip.rect.width / 2 - width / 2),
        window.innerWidth - width - MARGIN,
      ),
      y: above ? Math.max(MARGIN, tip.rect.top - GAP - height) : below,
      above,
    });
  }, [tip]);

  if (!tip) return null;
  return (
    <div
      className={`tooltip ${placement.above ? "above" : ""} ${tip.viaKeyboard ? "immediate" : ""}`}
      ref={surface}
      role="tooltip"
      style={{ left: `${placement.x}px`, top: `${placement.y}px` }}
    >
      {tip.text}
    </div>
  );
}
