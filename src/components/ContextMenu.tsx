import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

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
  const optionId = useId();
  const [position, setPosition] = useState<MenuAnchor>(anchor);
  const [active, setActive] = useState(-1);
  const commands = entries.filter(isCommand);
  const enabledIndexes = commands.map((command, index) => (command.disabled ? -1 : index)).filter((index) => index >= 0);
  const activeCommand = commands[active]?.disabled ? undefined : commands[active];

  useLayoutEffect(() => {
    const previous = document.activeElement;
    const menu = surface.current;
    return () => {
      const focused = document.activeElement;
      if (focused !== document.body && focused !== menu && !menu?.contains(focused)) return;
      if (previous instanceof HTMLElement && previous !== document.body && document.contains(previous)
        && !previous.matches(":disabled,[aria-disabled='true']")) {
        previous.focus({ preventScroll: true });
      } else {
        document.querySelector<HTMLElement>("main[tabindex='-1']")?.focus({ preventScroll: true });
      }
    };
  }, []);

  // Use layout dimensions before paint: animation transforms must not shrink
  // the measured flyout, and localized labels determine its actual width.
  useLayoutEffect(() => {
    const element = surface.current;
    if (!element) return;
    const layout = getComputedStyle(element);
    const width = parseFloat(layout.width) || element.offsetWidth;
    const height = parseFloat(layout.height) || element.offsetHeight;
    const rightToLeft = document.documentElement.dir === "rtl";
    let x = rightToLeft ? anchor.x - width : anchor.x;
    let y = anchor.y;
    if (x + width > window.innerWidth - MARGIN) x = anchor.x - width;
    x = Math.max(MARGIN, Math.min(x, window.innerWidth - width - MARGIN));
    // Flip above the pointer instead of clipping, which is what a real menu
    // near the bottom of the screen does.
    if (y + height > window.innerHeight - MARGIN) y = anchor.y - height;
    y = Math.max(MARGIN, Math.min(y, window.innerHeight - height - MARGIN));
    setPosition({ x, y });
    element.focus({ preventScroll: true });
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
      if (!enabledIndexes.length) return -1;
      const at = enabledIndexes.indexOf(current);
      return enabledIndexes[(((at < 0 ? (delta > 0 ? -1 : 0) : at) + delta) % enabledIndexes.length + enabledIndexes.length) % enabledIndexes.length];
    });
  }, [enabledIndexes]);

  const choose = (command: MenuCommand) => {
    if (command.disabled) return;
    onClose();
    command.run();
  };

  return createPortal(
    <div
      className="menu-layer fixed inset-0 z-50"
      onPointerDown={onClose}
      onContextMenu={(event) => { event.preventDefault(); onClose(); }}
    >
      <div
        // No scrim behind it. A right-click menu is a continuation of the thing
        // that was clicked, not a mode the window enters, and dimming the whole
        // app for four commands says otherwise.
        className="context-menu absolute grid outline-none"
        role="menu"
        aria-label={label}
        aria-activedescendant={activeCommand ? `${optionId}-${active}` : undefined}
        ref={surface}
        tabIndex={-1}
        style={{ left: `${position.x}px`, top: `${position.y}px` }}
        onPointerDown={(event) => event.stopPropagation()}
        onKeyDown={(event) => {
          if (event.key === "Escape") { event.preventDefault(); onClose(); return; }
          if (event.key === "Tab") { event.preventDefault(); onClose(); return; }
          if (event.key === "ArrowDown") { event.preventDefault(); step(1); return; }
          if (event.key === "ArrowUp") { event.preventDefault(); step(-1); return; }
          if (event.key === "Home" || event.key === "End") {
            event.preventDefault();
            setActive((event.key === "Home" ? enabledIndexes[0] : enabledIndexes.at(-1)) ?? -1);
            return;
          }
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            const command = commands[active];
            if (command) choose(command);
          }
        }}
      >
        {entries.map((entry, index) => {
          if (entry === "separator") return <hr className="menu-separator" key={`separator-${index}`} />;
          const selected = commands.indexOf(entry) === active;
          return (
            <button
              key={entry.id}
              type="button"
              role="menuitem"
              id={`${optionId}-${commands.indexOf(entry)}`}
              disabled={entry.disabled}
              tabIndex={-1}
              data-danger={entry.danger || undefined}
              className={`menu-command${selected ? " active" : ""}`}
              onPointerEnter={() => { if (!entry.disabled) setActive(commands.indexOf(entry)); }}
              onClick={() => choose(entry)}
            >
              <span
                className="menu-command-icon grid size-[15px] shrink-0 place-items-center"
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
    </div>, document.body
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
