import { useCallback, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";

/**
 * The flyout Windows draws for a right-click, rebuilt because turning the
 * system decorations off took the real one away with them.
 *
 * A surface that does nothing when right-clicked is one of the small absences
 * that make an app read as a web page in a frame, so every surface with
 * commands of its own answers with this.
 */
export interface MenuCommand {
  id: string;
  label: string;
  icon?: ReactNode;
  accelerator?: string;
  danger?: boolean;
  disabled?: boolean;
  run: () => void;
}

export type MenuEntry = MenuCommand | "separator";

export interface MenuAnchor {
  x: number;
  y: number;
}

/** Distance the flyout keeps from the window edge when it has to be nudged. */
const MARGIN = 8;

const ITEM = [
  "flex w-full items-center gap-2.5 rounded-[4px] px-2 py-[5px] text-left text-base",
  "transition-colors duration-75 disabled:pointer-events-none disabled:opacity-40",
].join(" ");

function isCommand(entry: MenuEntry): entry is MenuCommand {
  return entry !== "separator";
}

export default function ContextMenu({
  entries,
  anchor,
  label,
  onClose,
}: {
  entries: MenuEntry[];
  anchor: MenuAnchor;
  label: string;
  onClose: () => void;
}) {
  const surface = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState<MenuAnchor>(anchor);
  const [active, setActive] = useState(-1);
  const commands = entries.filter(isCommand);

  // Measured after the first paint rather than guessed: the flyout is as wide
  // as its longest label, which depends on the locale.
  useLayoutEffect(() => {
    const element = surface.current;
    if (!element) return;
    const { width, height } = element.getBoundingClientRect();
    const rightToLeft = document.documentElement.dir === "rtl";
    let x = rightToLeft ? anchor.x - width : anchor.x;
    let y = anchor.y;
    if (x + width > window.innerWidth - MARGIN) x = anchor.x - width;
    if (x < MARGIN) x = MARGIN;
    // Flip above the pointer instead of clipping, which is what a real menu
    // near the bottom of the screen does.
    if (y + height > window.innerHeight - MARGIN) y = Math.max(MARGIN, anchor.y - height);
    setPosition({ x, y });
    element.focus();
  }, [anchor.x, anchor.y]);

  useEffect(() => {
    const dismiss = () => onClose();
    window.addEventListener("resize", dismiss);
    window.addEventListener("blur", dismiss);
    window.addEventListener("wheel", dismiss, { passive: true });
    return () => {
      window.removeEventListener("resize", dismiss);
      window.removeEventListener("blur", dismiss);
      window.removeEventListener("wheel", dismiss);
    };
  }, [onClose]);

  const step = useCallback((delta: number) => {
    setActive((current) => {
      const enabled = commands.map((command, index) => (command.disabled ? -1 : index)).filter((index) => index >= 0);
      if (!enabled.length) return current;
      const at = enabled.indexOf(current);
      return enabled[(((at < 0 ? (delta > 0 ? -1 : 0) : at) + delta) % enabled.length + enabled.length) % enabled.length];
    });
  }, [commands]);

  const choose = (command: MenuCommand) => {
    if (command.disabled) return;
    onClose();
    command.run();
  };

  return (
    <div
      className="menu-layer fixed inset-0 z-50"
      onPointerDown={onClose}
      onContextMenu={(event) => { event.preventDefault(); onClose(); }}
    >
      <div
        // No scrim behind it. A right-click menu is a continuation of the thing
        // that was clicked, not a mode the window enters, and dimming the whole
        // app for four commands says otherwise.
        className="animate-pop absolute grid min-w-[190px] gap-px rounded-panel border border-line-strong bg-surface p-1 shadow-lift outline-none"
        role="menu"
        aria-label={label}
        ref={surface}
        tabIndex={-1}
        style={{ left: `${position.x}px`, top: `${position.y}px` }}
        onPointerDown={(event) => event.stopPropagation()}
        onKeyDown={(event) => {
          if (event.key === "Escape") { event.preventDefault(); onClose(); return; }
          if (event.key === "ArrowDown") { event.preventDefault(); step(1); return; }
          if (event.key === "ArrowUp") { event.preventDefault(); step(-1); return; }
          if (event.key === "Enter" || event.key === " ") {
            const command = commands[active];
            if (command) { event.preventDefault(); choose(command); }
          }
        }}
      >
        {entries.map((entry, index) => {
          if (entry === "separator") return <hr className="my-1 h-px border-0 bg-line" key={`separator-${index}`} />;
          const selected = commands.indexOf(entry) === active;
          return (
            <button
              key={entry.id}
              type="button"
              role="menuitem"
              disabled={entry.disabled}
              // `active` stays a bare token: the pointer and the arrow keys share
              // one highlight, so it cannot be left to :hover.
              //
              // The same mint ground the palette gives its cursor, because these
              // are the same object in two places — a list of commands with one
              // of them under the pointer — and a window that highlights the one
              // in grey and the other in mint is a window assembled from parts.
              // Destructive entries keep their own red ground: it is the one case
              // where the highlight has to say something the accent cannot.
              className={[
                ITEM,
                entry.danger ? "text-danger" : "text-text",
                selected ? `active ${entry.danger ? "bg-danger/14" : "bg-brand/12"}` : "",
              ].join(" ")}
              onPointerEnter={() => setActive(commands.indexOf(entry))}
              onClick={() => choose(entry)}
            >
              <span
                className={`grid size-[15px] shrink-0 place-items-center ${entry.danger ? "text-danger" : selected ? "text-brand" : "text-muted"}`}
                aria-hidden="true"
              >
                {entry.icon}
              </span>
              <span className="min-w-0 flex-1 truncate">{entry.label}</span>
              {entry.accelerator ? (
                <span className="shrink-0 pl-3 text-xs text-muted tabular-nums">{entry.accelerator}</span>
              ) : null}
            </button>
          );
        })}
      </div>
    </div>
  );
}

/**
 * Opens one flyout at a time for a surface, and hands back the element to
 * render. Keeping the anchor in state — rather than a portal ref — is what lets
 * a second right-click move the open menu instead of stacking another one.
 */
export function useContextMenu() {
  const [anchor, setAnchor] = useState<MenuAnchor | null>(null);
  const close = useCallback(() => setAnchor(null), []);
  const open = useCallback((event: {
    clientX: number;
    clientY: number;
    currentTarget: EventTarget | null;
    preventDefault: () => void;
  }) => {
    event.preventDefault();
    // Shift+F10 and the Menu key report no coordinates. Windows drops the
    // flyout on the focused control in that case, so do the same.
    if (event.clientX === 0 && event.clientY === 0 && event.currentTarget instanceof Element) {
      const rect = event.currentTarget.getBoundingClientRect();
      setAnchor({ x: rect.left + 12, y: rect.bottom - 4 });
      return;
    }
    setAnchor({ x: event.clientX, y: event.clientY });
  }, []);
  return { anchor, open, close };
}
