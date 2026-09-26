import { CircleHelp, FileCheck2, History, Settings, ShieldCheck } from "lucide-react";
import type { Page } from "../types";
import { commandKeyLabel } from "../lib/keys";
import { useI18n } from "../lib/i18n";

const navigation: Array<{ page: Page; label: string; icon: typeof FileCheck2; key: string }> = [
  { page: "clean", label: "文件净化", icon: FileCheck2, key: "1" },
  { page: "history", label: "处理记录", icon: History, key: "2" },
  { page: "privacy", label: "隐私说明", icon: ShieldCheck, key: "3" },
  { page: "settings", label: "设置", icon: Settings, key: "4" },
  { page: "about", label: "关于", icon: CircleHelp, key: "5" },
];

/**
 * A 264px workspace sidebar that can collapse to a 64px icon rail. Both states
 * keep the same navigation order and accessible names, so collapsing changes
 * density without changing the keyboard or screen-reader model.
 */
export default function Sidebar({ page, collapsed, onNavigate }: { page: Page; collapsed: boolean; onNavigate: (page: Page) => void }) {
  const { text } = useI18n();
  const labels: Record<Page, string> = {
    clean: text("文件净化", "Clean files"),
    history: text("处理记录", "History"),
    privacy: text("隐私说明", "Privacy"),
    settings: text("设置", "Settings"),
    about: text("关于", "About"),
  };

  const renderItem = ({ page: target, label, icon: Icon, key }: (typeof navigation)[number]) => {
    const active = page === target;
    const name = labels[target] ?? label;
    return (
      <button
        className={[
          "group relative flex h-10 w-full items-center rounded-control text-left",
          "transition-colors duration-150 ease-[var(--ease-out-soft)]",
          collapsed ? "justify-center px-0" : "gap-2 px-2.5",
          active
            ? "bg-surface-2/70 text-text"
            : "text-muted hover:bg-surface/70 hover:text-text",
        ].join(" ")}
        key={target}
        type="button"
        aria-current={active ? "page" : undefined}
        data-tip={`${name} · ${commandKeyLabel()}${key}`}
        onClick={() => onNavigate(target)}
      >
        <Icon className="shrink-0" size={16} strokeWidth={active ? 2.1 : 1.8} aria-hidden="true" />
        <span className={collapsed ? "sr-only" : "min-w-0 flex-1 truncate text-base font-medium"}>{name}</span>
        {!collapsed ? (
          <kbd className="kbd opacity-0 transition-opacity duration-150 group-hover:opacity-100 group-focus-visible:opacity-100" aria-hidden="true">
            {commandKeyLabel()}{key}
          </kbd>
        ) : null}
      </button>
    );
  };

  return (
    <aside className="sidebar chrome relative flex min-w-0 flex-col">
      <nav
        className="flex min-h-0 flex-1 flex-col gap-1 px-3 py-5"
        aria-label={text("主导航", "Main navigation")}
      >
        {!collapsed ? <p className="caption mb-2 px-2.5">{text("工作区", "Workspace")}</p> : null}
        {navigation.slice(0, 3).map(renderItem)}
        <div className="flex-1" />
        {!collapsed ? <p className="caption mb-1 border-t border-line px-2 pt-3">{text("应用", "Application")}</p> : <div className="mx-2 my-2 border-t border-line" />}
        {navigation.slice(3).map(renderItem)}
      </nav>
    </aside>
  );
}
