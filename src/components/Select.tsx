import { ChevronDown } from "lucide-react";
import type { SelectHTMLAttributes } from "react";

/**
 * A dropdown, wearing the same shell as every button beside it.
 *
 * The webview's own arrow is drawn at the system's size in the system's grey
 * and cannot be reached, so it is turned off and a lucide chevron sits over the
 * gutter `.field` reserves for it — which is what makes the arrow here the same
 * weight and the same ink as every other icon in the window.
 */
export default function Select({
  className = "",
  children,
  ...rest
}: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <div className="relative inline-flex shrink-0">
      <select className={`field w-full ${className}`} {...rest}>
        {children}
      </select>
      <ChevronDown
        size={14}
        strokeWidth={2}
        aria-hidden="true"
        // `muted`, not `faint`. This chevron is the entire reason anybody knows
        // the control opens — `.field` gives a select and a text box the same
        // shell — so it is the one glyph on the control that must not be the
        // dimmest thing on it.
        className="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-muted"
      />
    </div>
  );
}
