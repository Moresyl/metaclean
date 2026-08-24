import { Search } from "lucide-react";
import ContextMenu, { useContextMenu, type MenuEntry } from "./ContextMenu";
import { closeWindow, minimizeWindow } from "../lib/window";
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

export default function TitleBar({ onOpenCommands }: { onOpenCommands: () => void }) {
  const { text } = useI18n();
  const menu = useContextMenu();
  const minimize = text("最小化", "Minimize");
  const close = text("关闭", "Close");
  const commands = text("命令", "Commands");
  const entries: MenuEntry[] = [
    { id: "minimize", label: minimize, run: () => void minimizeWindow() },
    "separator",
    { id: "close", label: close, accelerator: "Alt+F4", danger: true, run: () => void closeWindow() },
  ];

  return (
    <header className="titlebar" data-tauri-drag-region onContextMenu={menu.open}>
      <div className="titlebar-brand" data-tauri-drag-region>
        <span className="brand-mark" aria-hidden="true">M</span>
        <span>MetaClean</span>
        <small>{text("隐私净化工作台", "Privacy workspace")}</small>
      </div>
      <button
        className="command-center"
        type="button"
        aria-label={commands}
        data-tip={`${commands} · ${commandKeyLabel()}K`}
        onClick={onOpenCommands}
      >
        <Search size={13} strokeWidth={2} aria-hidden="true" />
        <span>{text("搜索命令…", "Search commands…")}</span>
        <kbd>{commandKeyLabel()}K</kbd>
      </button>
      <div className="caption-buttons">
        <button className="caption-button" type="button" aria-label={minimize} data-tip={minimize} onClick={() => void minimizeWindow()}>
          <CaptionGlyph path={MINIMIZE} />
        </button>
        <button className="caption-button close" type="button" aria-label={close} data-tip={close} onClick={() => void closeWindow()}>
          <CaptionGlyph path={CLOSE} />
        </button>
      </div>
      {menu.anchor ? <ContextMenu entries={entries} anchor={menu.anchor} label="MetaClean" onClose={menu.close} /> : null}
    </header>
  );
}
