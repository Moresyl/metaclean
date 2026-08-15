import { Copy, FileWarning, RotateCcw } from "lucide-react";
import { useEffect, useState } from "react";
import { useI18n } from "../lib/i18n";
import type { CleanMode, ContextMenuStatus } from "../types";

export default function SettingsPage({ mode, onModeChange }: { mode: CleanMode; onModeChange: (mode: CleanMode) => void }) {
  const [contextMenu, setContextMenu] = useState<ContextMenuStatus>();
  const [busy, setBusy] = useState(false);
  const { locale, setLocale, text } = useI18n();
  useEffect(() => { void import("@tauri-apps/api/core").then(({ invoke }) => invoke<ContextMenuStatus>("get_context_menu_status")).then(setContextMenu).catch(() => undefined); }, []);
  async function toggleContextMenu() {
    if (!contextMenu?.available) return;
    setBusy(true);
    try { const { invoke } = await import("@tauri-apps/api/core"); setContextMenu(await invoke<ContextMenuStatus>("set_context_menu_enabled", { enabled: !contextMenu.enabled })); }
    finally { setBusy(false); }
  }
  return (
    <section className="secondary-page">
      <div className="page-heading"><div><h1>{text("设置", "Settings")}</h1><p>{text("设置会自动保存在本机。", "Settings are saved locally.")}</p></div></div>
      <div className="settings-card"><div className="setting-row"><div><h2>{text("界面语言", "Language")}</h2><p>{text("切换后立即生效。", "Changes apply immediately.")}</p></div><div className="locale-switch"><button className={locale === "zh" ? "selected" : ""} type="button" onClick={() => setLocale("zh")}>中文</button><button className={locale === "en" ? "selected" : ""} type="button" onClick={() => setLocale("en")}>English</button></div></div></div>
      <div className="settings-card"><h2>{text("默认输出方式", "Default output mode")}</h2><p>{text("每次添加新文件时使用的默认策略。", "The default strategy for newly added files.")}</p><div className="settings-choices"><button className={mode === "copy" ? "selected" : ""} type="button" onClick={() => onModeChange("copy")}><Copy size={18}/><span><strong>{text("保存为安全副本", "Save a safe copy")}</strong><small>{text("保留原文件，生成 .cleaned 副本", "Keep the original and create a .cleaned copy")}</small></span></button><button className={mode === "replace" ? "selected" : ""} type="button" onClick={() => onModeChange("replace")}><FileWarning size={18}/><span><strong>{text("替换并备份", "Replace with backup")}</strong><small>{text("先创建 .bak，再原子替换原文件", "Create .bak, then atomically replace the original")}</small></span></button></div></div>
      <div className="settings-card"><div className="setting-row"><div><h2>{text("Windows 右键菜单", "Windows context menu")}</h2><p>{contextMenu ? text(contextMenu.detail, contextMenu.available ? (contextMenu.enabled ? "Enabled for supported file types. On Windows 11, use Show more options." : "Enable the File Explorer command for supported types. On Windows 11, it appears under Show more options.") : "Context-menu integration is available on Windows only.") : text("正在检测资源管理器集成…", "Checking File Explorer integration…")}</p></div><button type="button" disabled={!contextMenu?.available || busy} onClick={() => void toggleContextMenu()}>{busy ? text("处理中…", "Working…") : contextMenu?.enabled ? text("停用", "Disable") : text("启用", "Enable")}</button></div></div>
      <div className="settings-card"><h2>{text("安全保证", "Safety guarantees")}</h2><div className="safety-grid"><span><RotateCcw size={16}/>{text("替换模式始终备份", "Replacement always creates a backup")}</span><span>{text("单文件最大 256 MiB", "256 MiB maximum per file")}</span><span>{text("拒绝写入符号链接", "Refuses symlink writes")}</span><span>{text("输出采用临时文件原子替换", "Atomic temporary-file output")}</span></div></div>
    </section>
  );
}
