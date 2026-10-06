import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

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

/** A brief pause keeps incidental pointer movement from opening a tip. */
const HOVER_DELAY = 150;
/** Keyboard focus is deliberate, so its tooltip arrives almost at once. */
const FOCUS_DELAY = 90;
const GAP = 5;
const MARGIN = 15;

interface Tip {
  host: HTMLElement;
  text: string;
  rect: DOMRect;
  viaKeyboard: boolean;
}

function tipFor(target: EventTarget | null): HTMLElement | null {
  return target instanceof Element ? target.closest<HTMLElement>("[data-tip]") : null;
}

export default function TooltipHost() {
  const descriptionId = useId();
  const [tip, setTip] = useState<Tip | null>(null);
  const [placement, setPlacement] = useState({ x: 0, y: 0, above: false });
  const surface = useRef<HTMLDivElement>(null);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => {
    const cancel = () => {
      window.clearTimeout(timer.current);
      timer.current = undefined;
      setTip(null);
    };
    const schedule = (host: HTMLElement, viaKeyboard: boolean) => {
      cancel();
      const text = host.dataset.tip;
      if (!text) return;
      timer.current = window.setTimeout(
        () => {
          timer.current = undefined;
          if (document.contains(host) && host.dataset.tip === text) {
            setTip({ host, text, rect: host.getBoundingClientRect(), viaKeyboard });
          }
        },
        viaKeyboard ? FOCUS_DELAY : HOVER_DELAY,
      );
    };

    const over = (event: PointerEvent) => {
      const host = tipFor(event.target);
      if (!host) { cancel(); return; }
      if (event.relatedTarget instanceof Node && host.contains(event.relatedTarget)) return;
      schedule(host, false);
    };
    const out = (event: PointerEvent) => {
      const host = tipFor(event.target);
      if (host && (!(event.relatedTarget instanceof Node) || !host.contains(event.relatedTarget))) cancel();
    };
    const keyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") cancel();
    };
    // A tooltip that stayed up while its control was being used would cover the
    // thing that just changed, so any press takes it down.
    const focusIn = (event: FocusEvent) => {
      const host = tipFor(event.target);
      if (host && host.matches(":focus-visible")) schedule(host, true);
      else cancel();
    };

    document.addEventListener("pointerover", over);
    document.addEventListener("pointerout", out);
    document.addEventListener("pointerdown", cancel);
    document.addEventListener("keydown", keyDown);
    document.addEventListener("focusin", focusIn);
    document.addEventListener("focusout", cancel);
    window.addEventListener("blur", cancel);
    window.addEventListener("resize", cancel);
    window.addEventListener("wheel", cancel, { passive: true });
    return () => {
      window.clearTimeout(timer.current);
      document.removeEventListener("pointerover", over);
      document.removeEventListener("pointerout", out);
      document.removeEventListener("pointerdown", cancel);
      document.removeEventListener("keydown", keyDown);
      document.removeEventListener("focusin", focusIn);
      document.removeEventListener("focusout", cancel);
      window.removeEventListener("blur", cancel);
      window.removeEventListener("resize", cancel);
      window.removeEventListener("wheel", cancel);
    };
  }, []);

  useLayoutEffect(() => {
    if (!tip) return;
    const descriptions = tip.host.getAttribute("aria-describedby")?.split(/\s+/).filter(Boolean) ?? [];
    tip.host.setAttribute("aria-describedby", [...new Set([...descriptions, descriptionId])].join(" "));
    const observer = new MutationObserver(() => {
      if (!document.contains(tip.host)) setTip(null);
    });
    observer.observe(document.body, { childList: true, subtree: true });
    return () => {
      observer.disconnect();
      const current = tip.host.getAttribute("aria-describedby")?.split(/\s+/).filter(Boolean) ?? [];
      if (!current.includes(descriptionId)) return;
      const retained = current.filter(id => id !== descriptionId);
      if (retained.length) tip.host.setAttribute("aria-describedby", retained.join(" "));
      else tip.host.removeAttribute("aria-describedby");
    };
  }, [tip, descriptionId]);

  useLayoutEffect(() => {
    const element = surface.current;
    if (!tip || !element) return;
    const style = getComputedStyle(element);
    const width = parseFloat(style.width) || element.offsetWidth;
    const height = parseFloat(style.height) || element.offsetHeight;
    const below = tip.rect.bottom + GAP;
    const above = below + height > window.innerHeight - MARGIN;
    setPlacement({
      x: Math.min(
        Math.max(MARGIN, tip.rect.left + tip.rect.width / 2 - width / 2),
        window.innerWidth - width - MARGIN,
      ),
      y: Math.max(MARGIN, Math.min(above ? tip.rect.top - GAP - height : below,
        window.innerHeight - height - MARGIN)),
      above,
    });
  }, [tip]);

  if (!tip) return null;
  return createPortal(
    <div
      className={[
        "tooltip pointer-events-none fixed z-[60]",
        // It arrives already in place when the keyboard asked for it: the delay
        // was the animation in that case, and doubling it reads as lag.
        tip.viaKeyboard ? "animate-fade" : "animate-pop",
      ].join(" ")}
      ref={surface}
      id={descriptionId}
      role="tooltip"
      style={{ left: `${placement.x}px`, top: `${placement.y}px` }}
    >
      {tip.text}
    </div>, document.body
  );
}
