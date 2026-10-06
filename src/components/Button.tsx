import { Children, forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";

/**
 * Shared native actions with four variants and three heights. Surface and
 * content rules live in the common stylesheet; callers supply behavior and labels.
 */
export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";

/** `md` is the default. `sm` is for a button inside a card's own header, `lg`
 *  for the one full-width action that commits a screen's worth of decisions. */
export type ButtonSize = "sm" | "md" | "lg";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  children?: ReactNode;
}

function ActionContent({ children }: { children: ReactNode }) {
  return <span className="action-content">{Children.map(children, child =>
    typeof child === "string" || typeof child === "number" ? <span>{child}</span> : child,
  )}</span>;
}

const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button({
  variant = "secondary",
  size = "md",
  className = "",
  type = "button",
  children,
  ...rest
}, ref) {
  return (
    <button ref={ref} type={type} className={`action ${className}`} data-variant={variant} data-size={size} {...rest}>
      <ActionContent>{children}</ActionContent>
    </button>
  );
});

export default Button;

/** A square button carrying nothing but an icon. Always labelled by the caller. */
export function IconButton({
  variant = "ghost",
  size = "md",
  className = "",
  type = "button",
  children,
  ...rest
}: ButtonProps) {
  return (
    <button
      type={type}
      className={`action action-icon ${className}`}
      data-variant={variant}
      data-size={size}
      {...rest}
    >
      <ActionContent>{children}</ActionContent>
    </button>
  );
}
