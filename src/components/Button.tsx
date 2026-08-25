import type { ButtonHTMLAttributes, ReactNode } from "react";

/**
 * The one button in the window.
 *
 * Four variants and two heights, and nothing outside this file decides what a
 * button looks like. That is the whole point: a window with six shades of
 * "secondary" reads as six programs, and the drift starts the first time a
 * screen needs a button one pixel shorter than the last one did.
 */
export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";

/** `md` is the default. `sm` is for a button inside a card's own header, `lg`
 *  for the one full-width action that commits a screen's worth of decisions. */
export type ButtonSize = "sm" | "md" | "lg";

const BASE =
  "inline-flex shrink-0 items-center justify-center gap-1.5 rounded-control font-medium " +
  "whitespace-nowrap transition duration-100 ease-[var(--ease-out-soft)] select-none";

/* What a button that cannot be pressed looks like — and it is not that button
   at 40% opacity.
 *
 * Fading a filled accent leaves the accent: mint at 0.4 over a near-black
 * ground is a muddy green slab carrying ink that was chosen to sit on mint,
 * which reads as a rendering fault rather than as a disabled control. So a
 * disabled fill gives up being a fill and becomes the flat grey every desktop
 * toolkit uses for the state. A ghost button goes the other way and gives up
 * nothing, because it had no box to lose — growing one when it stops working
 * would draw the eye to the one control on screen that cannot be used. */
const OFF: Record<ButtonVariant, string> = {
  primary: "disabled:bg-surface-2 disabled:text-faint",
  secondary: "disabled:bg-transparent disabled:border-line disabled:text-faint",
  ghost: "disabled:text-faint",
  danger: "disabled:bg-surface-2 disabled:text-faint",
};

/* 28px, which is what `.field` measures — a toolbar where the dropdown is two
   pixels taller than the button beside it is the specific kind of wrong nobody
   can name and everybody sees. */
const SIZES: Record<ButtonSize, string> = {
  sm: "h-[24px] px-2 text-sm",
  md: "h-[28px] px-3 text-base",
  lg: "h-[32px] px-4 text-md",
};

const VARIANTS: Record<ButtonVariant, string> = {
  // Filled with the accent, which is why there is at most one of these on
  // screen at a time: two would each be asking to be the answer.
  primary: "bg-brand text-on-brand enabled:hover:brightness-[1.08] enabled:active:brightness-95",
  secondary:
    "border border-line-strong bg-surface-2 text-text enabled:hover:border-faint " +
    "enabled:hover:bg-surface-2 enabled:hover:brightness-[1.06] enabled:active:brightness-95",
  // No box until it is pointed at — for the icon rows a card wears in its
  // header, where six outlined buttons would out-shout the list below them.
  ghost: "px-2 text-muted enabled:hover:bg-surface-2 enabled:hover:text-text",
  danger: "bg-danger text-on-danger enabled:hover:brightness-[1.08] enabled:active:brightness-95",
};

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  children?: ReactNode;
}

export default function Button({
  variant = "secondary",
  size = "md",
  className = "",
  type = "button",
  children,
  ...rest
}: ButtonProps) {
  return (
    <button type={type} className={`${BASE} ${SIZES[size]} ${VARIANTS[variant]} ${OFF[variant]} ${className}`} {...rest}>
      {children}
    </button>
  );
}

/** A square button carrying nothing but an icon. Always labelled by the caller. */
export function IconButton({
  variant = "ghost",
  size = "md",
  className = "",
  type = "button",
  children,
  ...rest
}: ButtonProps) {
  const box = size === "sm" ? "size-[24px]" : "size-[28px]";
  return (
    <button
      type={type}
      className={`${BASE} ${box} p-0 ${VARIANTS[variant]} ${OFF[variant]} ${className}`}
      {...rest}
    >
      {children}
    </button>
  );
}
