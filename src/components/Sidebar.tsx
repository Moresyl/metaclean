import { FileCheck2, History, Settings, ShieldCheck } from "lucide-react";
import type { Page } from "../types";
import { commandKeyLabel } from "../lib/keys";
import { useI18n } from "../lib/i18n";

const navigation: Array<{ page: Page; label: string; icon: typeof FileCheck2; key: string }> = [
  { page: "clean", label: "文件净化", icon: FileCheck2, key: "1" },
  { page: "history", label: "处理记录", icon: History, key: "2" },
  { page: "privacy", label: "隐私说明", icon: ShieldCheck, key: "3" },
  { page: "settings", label: "设置", icon: Settings, key: "4" },
];

export default function Sidebar({ page, onNavigate }: { page: Page; onNavigate: (page: Page) => void }) {
  const { text } = useI18n();
  const labels: Record<Page, string> = { clean: text("文件净化", "Clean files"), history: text("处理记录", "History"), privacy: text("隐私说明", "Privacy"), settings: text("设置", "Settings") };
  return (
    <aside className="sidebar">
      <nav aria-label={text("主导航", "Main navigation")}>
        {navigation.map(({ page: target, label, icon: Icon, key }) => (
          <button
            className={`nav-item ${page === target ? "active" : ""}`}
            key={target}
            type="button"
            data-tip={`${labels[target] ?? label} · ${commandKeyLabel()}${key}`}
            onClick={() => onNavigate(target)}
          >
            <Icon size={18} strokeWidth={1.8} /><span>{labels[target] ?? label}</span>
          </button>
        ))}
      </nav>
    </aside>
  );
}
