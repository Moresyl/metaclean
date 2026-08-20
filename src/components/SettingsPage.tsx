import {
  CheckCircle2,
  Copy,
  Download,
  ExternalLink,
  FileWarning,
  Monitor,
  Moon,
  Palette,
  RefreshCw,
  RotateCcw,
  ShieldCheck,
  SlidersHorizontal,
  Sun,
  Wrench,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useI18n } from "../lib/i18n";
import type { CleanMode, ContextMenuStatus } from "../types";
import { useUpdate } from "../contexts/UpdateContext";
import { useTheme } from "../contexts/ThemeContext";
import { LOCALES, type Locale } from "../lib/locales";

interface SettingsPageProps {
  mode: CleanMode;
  onModeChange: (mode: CleanMode) => void;
  preserveTimestamps: boolean;
  onPreserveTimestampsChange: (value: boolean) => void;
  preserveOrientation: boolean;
  onPreserveOrientationChange: (value: boolean) => void;
  preserveColorProfile: boolean;
  onPreserveColorProfileChange: (value: boolean) => void;
  removeExtendedAttributes: boolean;
  onRemoveExtendedAttributesChange: (value: boolean) => void;
}

type SettingsSection = "appearance" | "cleaning" | "system" | "safety";

export default function SettingsPage({
  mode,
  onModeChange,
  preserveTimestamps,
  onPreserveTimestampsChange,
  preserveOrientation,
  onPreserveOrientationChange,
  preserveColorProfile,
  onPreserveColorProfileChange,
  removeExtendedAttributes,
  onRemoveExtendedAttributesChange,
}: SettingsPageProps) {
  const [section, setSection] = useState<SettingsSection>("appearance");
  const [contextMenu, setContextMenu] = useState<ContextMenuStatus>();
  const [busy, setBusy] = useState(false);
  const { locale, setLocale, text } = useI18n();
  const update = useUpdate();
  const theme = useTheme();

  useEffect(() => {
    void import("@tauri-apps/api/core")
      .then(({ invoke }) => invoke<ContextMenuStatus>("get_context_menu_status"))
      .then(setContextMenu)
      .catch(() => undefined);
  }, []);

  async function toggleContextMenu() {
    if (!contextMenu?.available) return;
    setBusy(true);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      setContextMenu(await invoke<ContextMenuStatus>("set_context_menu_enabled", { enabled: !contextMenu.enabled }));
    } finally {
      setBusy(false);
    }
  }

  const updateBusy = update.status === "checking" || update.status === "updating";
  const updatePercent = update.progress?.total && update.progress.total > 0
    ? Math.min(100, Math.round((update.progress.downloaded / update.progress.total) * 100))
    : undefined;

  return (
    <section className="settings-page">
      <nav className="settings-nav" aria-label={text("设置分类", "Settings categories")}>
        <button className={section === "appearance" ? "active" : ""} type="button" onClick={() => setSection("appearance")}><Palette size={17}/><span>{text("外观与语言", "Appearance")}</span></button>
        <button className={section === "cleaning" ? "active" : ""} type="button" onClick={() => setSection("cleaning")}><SlidersHorizontal size={17}/><span>{text("清理偏好", "Cleaning")}</span></button>
        <button className={section === "system" ? "active" : ""} type="button" onClick={() => setSection("system")}><Wrench size={17}/><span>{text("系统与更新", "System & updates")}</span></button>
        <button className={section === "safety" ? "active" : ""} type="button" onClick={() => setSection("safety")}><ShieldCheck size={17}/><span>{text("安全保障", "Safety")}</span></button>
      </nav>

      <div className="settings-panel">
        {section === "appearance" ? <>
          <header className="settings-panel-heading"><div><h2>{text("外观与语言", "Appearance and language")}</h2><p>{text("调整 MetaClean 在此设备上的显示方式。", "Choose how MetaClean looks on this device.")}</p></div></header>
          <div className="settings-list">
            <div className="setting-item"><div><h3>{text("界面语言", "Language")}</h3><p>{text("切换后立即生效。", "Changes apply immediately.")}</p></div><div className="locale-switch"><select aria-label={text("界面语言", "Language")} value={locale} onChange={(event) => setLocale(event.target.value as Locale)}>{LOCALES.map((option) => <option key={option.code} value={option.code}>{option.nativeName}</option>)}</select></div></div>
            <div className="setting-item setting-item-stacked"><div><h3>{text("界面主题", "Theme")}</h3><p>{text("跟随系统，或固定使用浅色/深色主题。", "Follow the system or use a fixed light or dark theme.")}</p></div><div className="theme-choices"><button className={theme.mode === "system" ? "selected" : ""} type="button" onClick={() => theme.setMode("system")}><Monitor size={17}/>{text("跟随系统", "System")}</button><button className={theme.mode === "light" ? "selected" : ""} type="button" onClick={() => theme.setMode("light")}><Sun size={17}/>{text("浅色", "Light")}</button><button className={theme.mode === "dark" ? "selected" : ""} type="button" onClick={() => theme.setMode("dark")}><Moon size={17}/>{text("深色", "Dark")}</button></div></div>
          </div>
        </> : null}

        {section === "cleaning" ? <>
          <header className="settings-panel-heading"><div><h2>{text("清理偏好", "Cleaning preferences")}</h2><p>{text("设置默认输出方式和需要保留的文件属性。", "Set the default output and retained file properties.")}</p></div></header>
          <div className="settings-list">
            <div className="setting-item setting-item-stacked"><div><h3>{text("默认输出方式", "Default output mode")}</h3><p>{text("每次添加新文件时使用的默认策略。", "The default strategy for newly added files.")}</p></div><div className="settings-choices"><button className={mode === "copy" ? "selected" : ""} type="button" onClick={() => onModeChange("copy")}><Copy size={18}/><span><strong>{text("保存为安全副本", "Save a safe copy")}</strong><small>{text("保留原文件，生成 .cleaned 副本", "Keep the original and create a .cleaned copy")}</small></span></button><button className={mode === "replace" ? "selected" : ""} type="button" onClick={() => onModeChange("replace")}><FileWarning size={18}/><span><strong>{text("替换并备份", "Replace with backup")}</strong><small>{text("先创建 .bak，再原子替换原文件", "Create .bak, then atomically replace the original")}</small></span></button></div></div>
            <div className="setting-item setting-item-stacked"><div><h3>{text("保真选项", "Fidelity options")}</h3><p>{text("隐私元数据始终移除；这里只控制显示和文件管理所需的信息。", "Private metadata is always removed; these options only retain display and file-management data.")}</p></div><div className="fidelity-options"><label><input type="checkbox" checked={preserveOrientation} onChange={(event) => onPreserveOrientationChange(event.target.checked)}/><span><strong>{text("保留 JPEG 显示方向", "Preserve JPEG display orientation")}</strong><small>{text("重建最小方向信息，其余 EXIF/GPS 仍全部移除。", "Rebuild minimal orientation data; all other EXIF/GPS data is removed.")}</small></span></label><label><input type="checkbox" checked={preserveColorProfile} onChange={(event) => onPreserveColorProfileChange(event.target.checked)}/><span><strong>{text("图片", "Images")} · ICC / sRGB</strong></span></label><label><input type="checkbox" checked={removeExtendedAttributes} onChange={(event) => onRemoveExtendedAttributesChange(event.target.checked)}/><span><strong>− macOS · xattr</strong><small>{text("清理报告中列出的来源扩展属性。", "Remove provenance attributes listed in the scan report.")}</small></span></label><label><input type="checkbox" checked={preserveTimestamps} onChange={(event) => onPreserveTimestampsChange(event.target.checked)}/><span><strong>{text("保留访问和修改时间", "Preserve access and modification times")}</strong><small>{text("关闭后，输出文件使用清理时的时间。", "When disabled, output files use the cleanup time.")}</small></span></label></div></div>
          </div>
        </> : null}

        {section === "system" ? <>
          <header className="settings-panel-heading"><div><h2>{text("系统与更新", "System and updates")}</h2><p>{text("管理系统集成并检查 MetaClean 新版本。", "Manage system integration and MetaClean updates.")}</p></div></header>
          <div className="settings-list">
            <div className="setting-item"><div><h3>{text("Windows 右键菜单", "Windows context menu")}</h3><p>{contextMenu ? text(contextMenu.detail, contextMenu.available ? (contextMenu.enabled ? "Enabled for supported file types. On Windows 11, use Show more options." : "Enable the File Explorer command for supported types. On Windows 11, it appears under Show more options.") : "Context-menu integration is available on Windows only.") : text("正在检测资源管理器集成…", "Checking File Explorer integration…")}</p></div><button className="setting-button" type="button" disabled={!contextMenu?.available || busy} onClick={() => void toggleContextMenu()}>{busy ? text("处理中…", "Working…") : contextMenu?.enabled ? text("停用", "Disable") : text("启用", "Enable")}</button></div>
            <div className="setting-item setting-item-stacked update-settings"><div className="setting-row"><div><h3>{text("版本更新", "Software updates")}</h3><p>{update.status === "checking" ? text("正在检查 GitHub 正式版本…", "Checking the latest stable GitHub release…") : update.status === "updating" ? `${text("正在下载并安装更新…", "Downloading and installing…")}${updatePercent === undefined ? "" : ` ${updatePercent}%`}` : update.status === "available" ? text(`当前 v${update.currentVersion}，可更新到 v${update.info?.availableVersion}。`, `Version ${update.currentVersion} is installed; ${update.info?.availableVersion} is available.`) : update.status === "current" ? text(`当前 v${update.currentVersion}，已经是最新版。`, `Version ${update.currentVersion} is up to date.`) : update.status === "error" ? text(`检查失败：${update.error}`, `Update check failed: ${update.error}`) : text("仅连接 GitHub Releases，不发送文件或使用情况。", "Checks GitHub Releases only; no files or usage data are sent.")}</p></div><div className="settings-actions"><button type="button" disabled={updateBusy} onClick={() => void update.checkUpdate()}><RefreshCw size={15}/>{text("检查更新", "Check now")}</button>{update.info ? <button className="primary" type="button" disabled={updateBusy} onClick={() => void update.installUpdate()}>{update.runtime.selfUpdateSupported ? <Download size={15}/> : <ExternalLink size={15}/>} {update.runtime.selfUpdateSupported ? text("安装更新", "Install update") : text("前往 GitHub", "Open GitHub")}</button> : null}</div></div>
              {update.status === "updating" ? <progress className="update-progress" aria-label={text("更新进度", "Update progress")} max={100} value={updatePercent}/>: null}
              {update.info && update.status !== "current" ? <div className="update-notes"><strong>{text("更新内容", "What's new")} · {update.info.name}</strong>{update.info.notes ? <p>{update.info.notes}</p> : <p>{text("请前往 GitHub 查看完整更新说明。", "Open GitHub for the full release notes.")}</p>}</div> : null}
            </div>
            <label className="setting-item setting-toggle"><span><strong>{text("启动后自动检查", "Check automatically after launch")}</strong><small>{text("关闭后将保持离线，直到你手动检查。", "When disabled, MetaClean stays offline until you check manually.")}</small></span><input type="checkbox" checked={update.autoCheckEnabled} onChange={(event) => update.setAutoCheckEnabled(event.target.checked)}/></label>
          </div>
        </> : null}

        {section === "safety" ? <>
          <header className="settings-panel-heading"><div><h2>{text("安全保障", "Safety guarantees")}</h2><p>{text("这些规则由清理引擎强制执行，不依赖界面设置。", "The cleaning engine enforces these rules independently of UI settings.")}</p></div></header>
          <div className="settings-list safety-list">
            <div className="safety-item"><RotateCcw size={18}/><div><h3>{text("替换前始终备份", "Always back up before replacement")}</h3><p>{text("替换模式会先创建 .bak 文件。", "Replace mode creates a .bak file first.")}</p></div><CheckCircle2 size={18}/></div>
            <div className="safety-item"><ShieldCheck size={18}/><div><h3>{text("拒绝危险写入", "Reject unsafe writes")}</h3><p>{text("拒绝写入符号链接，并限制单文件最大 256 MiB。", "Symlink writes are refused and files are limited to 256 MiB.")}</p></div><CheckCircle2 size={18}/></div>
            <div className="safety-item"><Wrench size={18}/><div><h3>{text("原子生成输出", "Atomic output")}</h3><p>{text("先写入临时文件，验证后再替换目标。", "Output is written to a temporary file and verified before replacement.")}</p></div><CheckCircle2 size={18}/></div>
          </div>
        </> : null}
      </div>
    </section>
  );
}
