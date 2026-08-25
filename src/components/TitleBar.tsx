import { Search } from "lucide-react";
import ContextMenu, { useContextMenu, type MenuEntry } from "./ContextMenu";
import { closeWindow, isWindowDragTarget, minimizeWindow, startWindowDragging } from "../lib/window";
import { commandKeyLabel } from "../lib/keys";
import { useI18n } from "../lib/i18n";

/**
 * The window's own title bar.
 *
 * Windows draws one for free, but the free one is a strip of system chrome
 * bolted above the app: it cannot carry a control, it never follows the dark
 * theme reliably, and the seam between it and the navigation pane is the first
 * thing that gives a desktop app away as a web page in a frame. So the window
 * ships undecorated and this stands in its place, at the metrics the system
 * uses — a 36px bar with 46px caption buttons, the close button turning the
 * same red.
 *
 * The identity lives in the navigation pane below, which frees the middle of
 * the bar for the one control worth putting there: the command entry.
 */

/** Segoe Fluent's caption glyphs, at the half-pixel offsets that keep the
 *  1px strokes from landing between device pixels. */
const MINIMIZE = "M0 5.5h10";
const CLOSE = "M0.7 0.7 9.3 9.3M9.3 0.7 0.7 9.3";

function CaptionGlyph({ path }: { path: string }) {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true">
      <path d={path} stroke="currentColor" strokeWidth="1" />
    </svg>
  );
}

/* Arrow rather than hand: both Windows and macOS leave the system cursor alone
   over their own window controls, and so does this. */
const CAPTION =
  "grid h-full w-[46px] cursor-default place-items-center text-muted " +
  "transition-colors duration-100";

interface TitleBarProps {
  closeToTray: boolean;
  onOpenCommands: () => void;
}

export default function TitleBar({ closeToTray, onOpenCommands }: TitleBarProps) {
  const { text } = useI18n();
  const menu = useContextMenu();
  const minimize = text("最小化", "Minimize");
  const close = text("关闭", "Close");
  const closeAction = closeToTray
    ? text("关闭到托盘", "Close to tray")
    : text("关闭并退出", "Close and exit");
  const commands = text("命令", "Commands");
  const entries: MenuEntry[] = [
    { id: "minimize", label: minimize, run: () => void minimizeWindow() },
    "separator",
    { id: "close", label: closeAction, accelerator: "Alt+F4", danger: true, run: () => void closeWindow() },
  ];

  return (
    <header
      className="titlebar chrome relative z-30 flex h-9 items-stretch border-b border-line select-none"
      data-tauri-drag-region
      onContextMenu={menu.open}
      onMouseDown={(event) => {
        const target = event.target;
        if (event.button === 0
          && target instanceof Element
          && !target.hasAttribute("data-tauri-drag-region")
          && isWindowDragTarget(target)) {
          void startWindowDragging();
        }
      }}
    >
      <div className="titlebar-brand flex flex-1 items-center gap-2 pl-2.5" data-tauri-drag-region>
        <span
          className="grid size-[17px] shrink-0 place-items-center rounded-[5px] bg-brand text-xs leading-none font-bold text-on-brand"
          aria-hidden="true"
        >
          M
        </span>
        <span className="text-base font-medium">MetaClean</span>
        <small className="hidden truncate text-xs text-muted sm:block">
          {text("隐私净化工作台", "Privacy workspace")}
        </small>
      </div>

      {/* Centred on the window rather than after the name, because it is the one
          control here and a search field that slides as the title grows reads as
          a toolbar item instead of as the window's own entry point. */}
      <button
        // `muted` at rest and the window's own ink on hover. This is a button
        // that is meant to be found — it is the only way into the palette that
        // does not require already knowing the shortcut — and a button set in
        // the ink reserved for placeholders is a button that reads as disabled.
        className="absolute top-1/2 left-1/2 flex h-[24px] w-[min(300px,38vw)] -translate-x-1/2 -translate-y-1/2 items-center gap-2 rounded-control border border-line bg-canvas-deep pr-1.5 pl-2 text-sm text-muted transition-colors duration-100 hover:border-line-strong hover:text-text"
        type="button"
        aria-label={commands}
        data-tip={`${commands} · ${commandKeyLabel()}K`}
        onClick={onOpenCommands}
      >
        <Search size={14} strokeWidth={2} aria-hidden="true" className="shrink-0" />
        <span className="flex-1 truncate text-left">{text("搜索命令…", "Search commands…")}</span>
        <kbd className="kbd">{commandKeyLabel()}K</kbd>
      </button>

      <div className="flex items-stretch">
        <button
          className={`${CAPTION} hover:bg-[color-mix(in_oklab,var(--color-text)_10%,transparent)] hover:text-text`}
          type="button"
          aria-label={minimize}
          data-tip={minimize}
          onClick={() => void minimizeWindow()}
        >
          <CaptionGlyph path={MINIMIZE} />
        </button>
        {/* The system's own close red, because this is the one button in the
            window whose colour a user already knows by heart. */}
        <button
          className={`${CAPTION} hover:bg-[#c42b1c] hover:text-white`}
          type="button"
          aria-label={close}
          data-tip={closeAction}
          onClick={() => void closeWindow()}
        >
          <CaptionGlyph path={CLOSE} />
        </button>
      </div>

      {menu.anchor ? <ContextMenu entries={entries} anchor={menu.anchor} label="MetaClean" onClose={menu.close} /> : null}
    </header>
  );
}
