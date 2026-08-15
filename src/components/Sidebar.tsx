import { FileCheck2, History, Settings, ShieldCheck } from "lucide-react";

const navigation = [
  { label: "文件净化", icon: FileCheck2, active: true },
  { label: "处理记录", icon: History, active: false },
  { label: "隐私说明", icon: ShieldCheck, active: false },
  { label: "设置", icon: Settings, active: false },
];

export default function Sidebar() {
  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark">M</div>
        <div><strong>MetaClean</strong><span>文件隐私净化器</span></div>
      </div>
      <nav aria-label="主导航">
        {navigation.map(({ label, icon: Icon, active }) => (
          <button className={`nav-item ${active ? "active" : ""}`} key={label} type="button">
            <Icon size={17} strokeWidth={1.8} />{label}
          </button>
        ))}
      </nav>
      <div className="local-note"><ShieldCheck size={15} /><span><strong>纯本地处理</strong>文件不会离开设备</span></div>
    </aside>
  );
}
