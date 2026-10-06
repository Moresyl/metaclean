import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";

interface SegmentOption<Value extends string> {
  value: Value;
  label: string;
  icon?: ReactNode;
  disabled?: boolean;
}

interface SegmentedControlProps<Value extends string> {
  label: string;
  options: SegmentOption<Value>[];
  value: Value;
  onChange: (value: Value) => void;
  className?: string;
}

/** A single-choice track with native buttons and one keyboard entry point. */
export default function SegmentedControl<Value extends string>({
  label, options, value, onChange, className = "",
}: SegmentedControlProps<Value>) {
  const track = useRef<HTMLDivElement>(null);
  const thumb = useRef<HTMLSpanElement>(null);
  const [focusedValue, setFocusedValue] = useState(value);
  const entry = options.find(option => option.value === focusedValue && !option.disabled)
    ?? options.find(option => option.value === value && !option.disabled)
    ?? options.find(option => !option.disabled);

  useEffect(() => { setFocusedValue(value); }, [value]);

  useLayoutEffect(() => {
    const container = track.current;
    const indicator = thumb.current;
    if (!container || !indicator) return;
    const measure = () => {
      const selected = container.querySelector<HTMLButtonElement>("button[aria-pressed='true']");
      indicator.hidden = !selected;
      if (!selected) return;
      indicator.style.width = `${selected.offsetWidth}px`;
      indicator.style.transform = `translateX(${selected.offsetLeft}px)`;
      if (container.scrollWidth <= container.clientWidth) return;
      const bounds = container.getBoundingClientRect();
      const item = selected.getBoundingClientRect();
      const margin = bounds.width * 0.15;
      if (item.left < bounds.left + margin || item.right > bounds.right - margin) {
        container.scrollBy({
          left: item.left + item.width / 2 - bounds.left - bounds.width / 2,
          behavior: indicator.dataset.ready && !window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "smooth" : "instant",
        });
      }
    };
    measure();
    const frame = requestAnimationFrame(() => { indicator.dataset.ready = "true"; });
    const observer = typeof ResizeObserver === "undefined" ? undefined : new ResizeObserver(measure);
    observer?.observe(container);
    container.querySelectorAll("button").forEach(button => observer?.observe(button));
    return () => { cancelAnimationFrame(frame); observer?.disconnect(); };
  }, [value, options]);

  function moveFocus(event: KeyboardEvent<HTMLDivElement>) {
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
    const container = track.current;
    if (!container || !(event.target instanceof HTMLButtonElement)) return;
    const buttons = [...container.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
    const index = buttons.indexOf(event.target);
    if (index < 0) return;
    const rtl = getComputedStyle(container).direction === "rtl";
    let step = 0;
    if (event.key === "ArrowRight") step = rtl ? -1 : 1;
    if (event.key === "ArrowLeft") step = rtl ? 1 : -1;
    if (event.key === "ArrowDown") step = 1;
    if (event.key === "ArrowUp") step = -1;
    const next = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1 : step ? index + step : -1;
    if (!step && event.key !== "Home" && event.key !== "End") return;
    event.preventDefault();
    buttons[next]?.focus();
  }

  return (
    <div ref={track} role="group" aria-label={label} className={`segmented-control ${className}`} onKeyDown={moveFocus}>
      <span ref={thumb} className="segment-thumb" aria-hidden="true" hidden />
      {options.map(option => (
        <button
          key={option.value}
          type="button"
          className="segment-option"
          aria-pressed={option.value === value}
          disabled={option.disabled}
          tabIndex={entry?.value === option.value ? 0 : -1}
          onFocus={() => setFocusedValue(option.value)}
          onClick={() => { if (option.value !== value) onChange(option.value); }}
        >
          <span className="segment-content">
            {option.icon ? <span aria-hidden="true">{option.icon}</span> : null}
            <span>{option.label}</span>
          </span>
        </button>
      ))}
    </div>
  );
}
