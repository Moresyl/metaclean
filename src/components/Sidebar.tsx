import { FileCheck2, History, Settings, ShieldCheck } from "lucide-react";
import type { Page } from "../types";
import Ambient from "./Ambient";
import { commandKeyLabel } from "../lib/keys";
import { useI18n } from "../lib/i18n";

const navigation: Array<{ page: Page; label: string; icon: typeof FileCheck2; key: string }> = [
  { page: "clean", label: "文件净化", icon: FileCheck2, key: "1" },
  { page: "history", label: "处理记录", icon: History, key: "2" },
  { page: "privacy", label: "隐私说明", icon: ShieldCheck, key: "3" },
  { page: "settings", label: "设置", icon: Settings, key: "4" },
];

/**
 * The navigation rail.
 *
 * Each item is a fixed 60px square centred in the rail, not a button stretched
 * to the rail's width — which is the difference between four tiles that line up
 * and four boxes that merely happen to be the same size. The size falls out of
 * the label: a CJK glyph is exactly one em wide, so 文件净化 at 12px measures
 * 48px and nothing about it is negotiable. 60px is that plus 6px of air on each
 * side — enough that nothing truncates and nothing crowds the edge.
 *
 * The rail's top padding matches the page header's beside it, so the first item
 * and the page title start on the same line.
 */
export default function Sidebar({ page, onNavigate }: { page: Page; onNavigate: (page: Page) => void }) {
  const { text } = useI18n();
  const labels: Record<Page, string> = {
    clean: text("文件净化", "Clean files"),
    history: text("处理记录", "History"),
    privacy: text("隐私说明", "Privacy"),
    settings: text("设置", "Settings"),
  };

  return (
    <aside className="sidebar chrome relative flex flex-col border-r border-line">
      <Ambient />
      <nav
        className="relative z-10 flex flex-col items-center gap-1 px-1.5 pt-4"
        aria-label={text("主导航", "Main navigation")}
      >
        {navigation.map(({ page: target, label, icon: Icon, key }) => {
          const active = page === target;
          const name = labels[target] ?? label;
          return (
            <button
              className={[
                "relative flex size-[60px] flex-col items-center justify-center gap-1.5 rounded-panel",
                "transition-colors duration-100",
                // `muted`, not `faint`: these four are the window's primary
                // navigation, and a label somebody is meant to aim at should
                // not be dimmer than the metadata in the status strip.
                active ? "bg-surface-2 text-text" : "text-muted hover:bg-surface-2/60 hover:text-text",
              ].join(" ")}
              key={target}
              type="button"
              aria-current={active ? "page" : undefined}
              data-tip={`${name} · ${commandKeyLabel()}${key}`}
              onClick={() => onNavigate(target)}
            >
              {/* Flush against the window's own edge — `-left-1.5` is exactly
                  the nav's `px-1.5`, cancelled — so it reads as "you are here
                  in this rail" rather than as a stray border on one tile. Half
                  way between the two, which is where it sat, it reads as
                  neither. It grows rather than appears: switching pages should
                  be one movement, not two things swapping. */}
              <span
                className={[
                  "absolute top-1/2 -left-1.5 w-[2.5px] -translate-y-1/2 rounded-r-full bg-brand",
                  "transition-[height] duration-200 ease-[var(--ease-out-soft)]",
                  active ? "h-[18px]" : "h-0",
                ].join(" ")}
                aria-hidden="true"
              />
              <Icon size={17} strokeWidth={active ? 2.2 : 1.8} aria-hidden="true" />
              <span className="text-xs leading-none font-medium">{name}</span>
            </button>
          );
        })}
      </nav>
    </aside>
  );
}
