import { forwardRef, type SelectHTMLAttributes } from "react";

/**
 * A native dropdown with a shared shell and decorative direction indicator.
 * Keep options, keyboard interaction and form behavior on the real select.
 */
const Select = forwardRef<HTMLSelectElement, SelectHTMLAttributes<HTMLSelectElement>>(function Select({
  className = "",
  children,
  ...rest
}, ref) {
  return (
    <div className="select-control relative inline-flex shrink-0">
      <select ref={ref} className={`field w-full ${className}`} {...rest}>
        {children}
      </select>
      <svg
        viewBox="0 0 8 12"
        aria-hidden="true"
        focusable="false"
        className="select-indicator pointer-events-none absolute top-1/2 -translate-y-1/2"
      >
        <path d="M1 4 4 1 7 4M1 8 4 11 7 8" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    </div>
  );
});

export default Select;
