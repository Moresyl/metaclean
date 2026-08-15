import { FileCheck2, History, Settings, ShieldCheck } from "lucide-react";
import type { Page } from "../types";

const navigation: Array<{ page: Page; label: string; icon: typeof FileCheck2 }> = [
  { page: "clean", label: "文件净化", icon: FileCheck2 },
  { page: "history", label: "处理记录", icon: History },
  { page: "privacy", label: "隐私说明", icon: ShieldCheck },
  { page: "settings", label: "设置", icon: Settings },
];

export default function Sidebar({ page, onNavigate }: { page: Page; onNavigate: (page: Page) => void }) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark">M</div>
        <div><strong>MetaClean</strong><span>文件隐私净化器</span></div>
      </div>
      <nav aria-label="主导航">
        {navigation.map(({ page: target, label, icon: Icon }) => (
          <button className={`nav-item ${page === target ? "active" : ""}`} key={target} type="button" onClick={() => onNavigate(target)}>
            <Icon size={17} strokeWidth={1.8} />{label}
          </button>
        ))}
      </nav>
      <div className="local-note"><ShieldCheck size={15} /><span><strong>纯本地处理</strong>文件不会离开设备</span></div>
    </aside>
  );
}
