import { FileCheck2, History, Settings, ShieldCheck } from "lucide-react";
import type { Page } from "../types";
import { useI18n } from "../lib/i18n";
import { useUpdate } from "../contexts/UpdateContext";

const navigation: Array<{ page: Page; label: string; icon: typeof FileCheck2 }> = [
  { page: "clean", label: "文件净化", icon: FileCheck2 },
  { page: "history", label: "处理记录", icon: History },
  { page: "privacy", label: "隐私说明", icon: ShieldCheck },
  { page: "settings", label: "设置", icon: Settings },
];

export default function Sidebar({ page, onNavigate }: { page: Page; onNavigate: (page: Page) => void }) {
  const { text } = useI18n();
  const update = useUpdate();
  const labels: Record<Page, string> = { clean: text("文件净化", "Clean files"), history: text("处理记录", "History"), privacy: text("隐私说明", "Privacy"), settings: text("设置", "Settings") };
  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark">M</div>
        <div><strong>MetaClean</strong><span>{text("文件隐私净化器", "File privacy cleaner")}</span></div>
      </div>
      <nav aria-label={text("主导航", "Main navigation")}>
        {navigation.map(({ page: target, label, icon: Icon }) => (
          <button className={`nav-item ${page === target ? "active" : ""}`} key={target} type="button" onClick={() => onNavigate(target)}>
            <Icon size={17} strokeWidth={1.8} />{labels[target] ?? label}
          </button>
        ))}
      </nav>
      <footer className="sidebar-footer">
        <div className="local-note"><ShieldCheck size={15} /><span><strong>{text("纯本地处理", "Local only")}</strong>{text("文件不会离开设备", "Files never leave this device")}</span></div>
        <span className="app-version">MetaClean v{update.currentVersion ?? "…"}</span>
      </footer>
    </aside>
  );
}
