import {
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
import { useEffect, useState, type ReactNode } from "react";
import Button from "./Button";
import Select from "./Select";
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
  closeToTray: boolean;
  onCloseToTrayChange: (value: boolean) => void;
}

type SettingsSection = "appearance" | "cleaning" | "system" | "safety";

/** One row of the settings list: a card with its own hairline. */
const ITEM = "grid gap-2.5 rounded-panel border border-line bg-surface p-3.5 shadow-panel";
/** The same card, but the whole of it is the switch's hit target — and the
 *  control on the right of it is now actually a switch rather than a checkbox
 *  wearing this comment. See `.switch` in `styles.css` for why the distance
 *  between the label and the control decides which of the two is correct. */
const TOGGLE = "flex cursor-pointer items-center gap-3 rounded-panel border border-line bg-surface p-3.5 shadow-panel transition-colors duration-100 hover:border-line-strong";

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
  closeToTray,
  onCloseToTrayChange,
}: SettingsPageProps) {
  const [section, setSection] = useState<SettingsSection>("appearance");
  const [contextMenu, setContextMenu] = useState<ContextMenuStatus>();
  const [contextMenuError, setContextMenuError] = useState<string>();
  const [busy, setBusy] = useState(false);
  const { locale, setLocale, text } = useI18n();
  const update = useUpdate();
  const theme = useTheme();

  useEffect(() => {
    void import("@tauri-apps/api/core")
      .then(({ invoke }) => invoke<ContextMenuStatus>("get_context_menu_status"))
      .then(setContextMenu)
      .catch((error) => setContextMenuError(text(
        `无法读取右键菜单状态：${String(error)}`,
        `Could not read the context-menu status: ${String(error)}`,
      )));
  }, [text]);

  async function toggleContextMenu() {
    if (!contextMenu?.available) return;
    setBusy(true);
    setContextMenuError(undefined);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      setContextMenu(await invoke<ContextMenuStatus>("set_context_menu_enabled", { enabled: !contextMenu.enabled }));
    } catch (error) {
      setContextMenuError(text(
        `更新右键菜单失败：${String(error)}`,
        `Could not update the context menu: ${String(error)}`,
      ));
    } finally {
      setBusy(false);
    }
  }

  const updateBusy = update.status === "checking" || update.status === "updating";
  const updatePercent = update.progress?.total && update.progress.total > 0
    ? Math.min(100, Math.round((update.progress.downloaded / update.progress.total) * 100))
    : undefined;

  const sections: Array<[SettingsSection, ReactNode, string]> = [
    ["appearance", <Palette size={14} strokeWidth={2} />, text("外观与语言", "Appearance")],
    ["cleaning", <SlidersHorizontal size={14} strokeWidth={2} />, text("清理偏好", "Cleaning")],
    ["system", <Wrench size={14} strokeWidth={2} />, text("系统与更新", "System & updates")],
    ["safety", <ShieldCheck size={14} strokeWidth={2} />, text("安全保障", "Safety")],
  ];

  return (
    /* 680px, where the file pages get 900. A settings row is a label on the
       left and one control on the right, so the width it wants is whatever puts
       those two within a glance of each other — at 900 the language dropdown
       sat half a metre from the word 界面语言 with nothing in between, which is
       a table of contents, not a form. */
    <section className="grid h-full max-w-[680px] grid-rows-[auto_minmax(0,1fr)] gap-3.5">
      {/* Segments in a recessed track, not a second rail down the side. Four
          categories is a switch, and a switch belongs on one line above the
          thing it switches — a 176px column held four short labels and 500px of
          nothing, which is what made the page read as half-built. The selected
          segment is raised out of the track on the card colour, so which one is
          on can be read without reading the labels. */}
      {/* `justify-self`, not `self`: inside a grid `self-start` aligns on the
          block axis, which this row was already doing, and left the track
          stretched the full 900px with four segments huddled at one end of
          it — a switch drawn as if it were a table. */}
      <nav
        className="settings-nav flex shrink-0 items-center gap-1 justify-self-start rounded-panel border border-line bg-canvas-deep p-1"
        aria-label={text("设置分类", "Settings categories")}
      >
        {sections.map(([id, icon, label]) => {
          const selected = section === id;
          return (
            <button
              key={id}
              type="button"
              aria-pressed={selected}
              onClick={() => setSection(id)}
              className={[
                "flex h-[28px] items-center gap-1.5 rounded-control px-2.5 text-base whitespace-nowrap",
                "transition-colors duration-100",
                selected
                  ? "cursor-default bg-surface font-medium text-text shadow-panel"
                  : "text-muted hover:bg-surface/55 hover:text-text",
              ].join(" ")}
            >
              {/* The glyph tracks the label it sits beside — `muted` at rest,
                  mint when this is the open section — instead of dropping a
                  step below it. A tab strip where four icons are dimmer than
                  the four words attached to them reads as four disabled tabs
                  with one live one, which is not what a segmented control is. */}
              <span className={`shrink-0 ${selected ? "text-brand" : "text-inherit"}`} aria-hidden="true">{icon}</span>
              {label}
            </button>
          );
        })}
      </nav>

      <div className="settings-list grid min-h-0 auto-rows-max gap-2.5 overflow-y-auto pr-0.5 pb-1">
        {section === "appearance" ? <>
          <PanelHead
            title={text("外观与语言", "Appearance and language")}
            detail={text("调整 MetaClean 在此设备上的显示方式。", "Choose how MetaClean looks on this device.")}
          />
          <div className={ITEM}>
            <div className="flex items-center gap-3">
              <Head title={text("界面语言", "Language")} detail={text("切换后立即生效。", "Changes apply immediately.")} />
              <div className="locale-switch">
                <Select
                  aria-label={text("界面语言", "Language")}
                  className="max-w-[160px]"
                  value={locale}
                  onChange={(event) => setLocale(event.target.value as Locale)}
                >
                  {LOCALES.map((option) => <option key={option.code} value={option.code}>{option.nativeName}</option>)}
                </Select>
              </div>
            </div>
          </div>
          <div className={ITEM}>
            <Head
              title={text("界面主题", "Theme")}
              detail={text("跟随系统，或固定使用浅色/深色主题。", "Follow the system or use a fixed light or dark theme.")}
            />
            {/* Capped, because three choices do not get wider just because the
                window did. Stretched across the card each tile was 300px of
                empty ground around one 17px glyph, which reads as three panels
                rather than as one control with three positions. */}
            <div className="theme-choices grid max-w-[340px] grid-cols-3 gap-2">
              <Tile selected={theme.mode === "system"} onClick={() => theme.setMode("system")}>
                <Monitor size={17} strokeWidth={1.8} />{text("跟随系统", "System")}
              </Tile>
              <Tile selected={theme.mode === "light"} onClick={() => theme.setMode("light")}>
                <Sun size={17} strokeWidth={1.8} />{text("浅色", "Light")}
              </Tile>
              <Tile selected={theme.mode === "dark"} onClick={() => theme.setMode("dark")}>
                <Moon size={17} strokeWidth={1.8} />{text("深色", "Dark")}
              </Tile>
            </div>
          </div>
        </> : null}

        {section === "cleaning" ? <>
          <PanelHead
            title={text("清理偏好", "Cleaning preferences")}
            detail={text("设置默认输出方式和需要保留的文件属性。", "Set the default output and retained file properties.")}
          />
          <div className={ITEM}>
            <Head
              title={text("默认输出方式", "Default output mode")}
              detail={text("每次添加新文件时使用的默认策略。", "The default strategy for newly added files.")}
            />
            <div className="grid gap-2 sm:grid-cols-2">
              <Choice
                selected={mode === "copy"}
                icon={<Copy size={17} strokeWidth={1.8} />}
                title={text("保存为安全副本", "Save a safe copy")}
                detail={text("保留原文件，生成 .cleaned 副本", "Keep the original and create a .cleaned copy")}
                onClick={() => onModeChange("copy")}
              />
              <Choice
                selected={mode === "replace"}
                icon={<FileWarning size={17} strokeWidth={1.8} />}
                title={text("替换并备份", "Replace with backup")}
                detail={text("先创建 .bak，再原子替换原文件", "Create .bak, then atomically replace the original")}
                onClick={() => onModeChange("replace")}
              />
            </div>
          </div>
          <div className={ITEM}>
            <Head
              title={text("保真选项", "Fidelity options")}
              detail={text("隐私元数据始终移除；这里只控制显示和文件管理所需的信息。", "Private metadata is always removed; these options only retain display and file-management data.")}
            />
            <div className="fidelity-options grid gap-0.5">
              <Check checked={preserveOrientation} onChange={onPreserveOrientationChange}
                title={text("保留 JPEG 显示方向", "Preserve JPEG display orientation")}
                detail={text("重建最小方向信息，其余 EXIF/GPS 仍全部移除。", "Rebuild minimal orientation data; all other EXIF/GPS data is removed.")} />
              <Check checked={preserveColorProfile} onChange={onPreserveColorProfileChange}
                title={`${text("图片", "Images")} · ICC / sRGB`} />
              <Check checked={removeExtendedAttributes} onChange={onRemoveExtendedAttributesChange}
                title="− macOS · xattr"
                detail={text("清理报告中列出的来源扩展属性。", "Remove provenance attributes listed in the scan report.")} />
              <Check checked={preserveTimestamps} onChange={onPreserveTimestampsChange}
                title={text("保留访问和修改时间", "Preserve access and modification times")}
                detail={text("关闭后，输出文件使用清理时的时间。", "When disabled, output files use the cleanup time.")} />
            </div>
          </div>
        </> : null}

        {section === "system" ? <>
          <PanelHead
            title={text("系统与更新", "System and updates")}
            detail={text("管理系统集成并检查 MetaClean 新版本。", "Manage system integration and MetaClean updates.")}
          />
          <div className={ITEM}>
            <div className="flex items-center gap-3">
              <Head title={text("Windows 右键菜单", "Windows context menu")} detail={contextMenuError ?? (contextMenu ? text(contextMenu.detail, contextMenu.available ? (contextMenu.enabled ? "Enabled for supported file types. On Windows 11, use Show more options." : "Enable the File Explorer command for supported types. On Windows 11, it appears under Show more options.") : "Context-menu integration is available on Windows only.") : text("正在检测资源管理器集成…", "Checking File Explorer integration…"))} />
              <Button size="sm" disabled={!contextMenu?.available || busy} onClick={() => void toggleContextMenu()}>
                {busy ? text("处理中…", "Working…") : contextMenu?.enabled ? text("停用", "Disable") : text("启用", "Enable")}
              </Button>
            </div>
          </div>

          <label className={TOGGLE}>
            <Head
              title={text("关闭按钮退出应用", "Exit when closing the window")}
              detail={text("默认完全退出；关闭后也可以选择继续驻留系统托盘。", "Exit completely by default, or keep MetaClean available in the system tray.")}
            />
            <input
              className="switch"
              aria-label={text("关闭按钮退出应用", "Exit when closing the window")}
              type="checkbox"
              checked={!closeToTray}
              onChange={(event) => onCloseToTrayChange(!event.target.checked)}
            />
          </label>

          <div className={ITEM}>
            <div className="flex flex-wrap items-start gap-3">
              <Head title={text("版本更新", "Software updates")} detail={update.status === "checking" ? text("正在检查 GitHub 正式版本…", "Checking the latest stable GitHub release…") : update.status === "updating" ? `${text("正在下载并安装更新…", "Downloading and installing…")}${updatePercent === undefined ? "" : ` ${updatePercent}%`}` : update.status === "available" ? text(`当前 v${update.currentVersion}，可更新到 v${update.info?.availableVersion}。`, `Version ${update.currentVersion} is installed; ${update.info?.availableVersion} is available.`) : update.status === "current" ? text(`当前 v${update.currentVersion}，已经是最新版。`, `Version ${update.currentVersion} is up to date.`) : update.status === "error" ? text("这次检查没有完成。", "This check did not complete.") : text("仅连接 GitHub 官方托管更新源，不发送文件或使用情况。", "Checks GitHub Releases only; no files or usage data are sent.")} />
              <div className="flex shrink-0 gap-2">
                <Button size="sm" disabled={updateBusy} onClick={() => void update.checkUpdate()}>
                  <RefreshCw size={14} strokeWidth={2} className={update.status === "checking" ? "animate-spin" : ""} />
                  {text("检查更新", "Check now")}
                </Button>
                {update.info ? (
                  <Button size="sm" variant="primary" disabled={updateBusy} onClick={() => void update.installUpdate()}>
                    {update.runtime.selfUpdateSupported ? <Download size={14} strokeWidth={2} /> : <ExternalLink size={14} strokeWidth={2} />}
                    {update.runtime.selfUpdateSupported ? text("安装更新", "Install update") : text("前往 GitHub", "Open GitHub")}
                  </Button>
                ) : null}
              </div>
            </div>
            {update.status === "updating" ? (
              <progress className="progress" aria-label={text("更新进度", "Update progress")} max={100} value={updatePercent} />
            ) : null}
            {/* A failure is drawn as a failure.
                The reason used to be the sixth branch of the same ternary that
                renders 已经是最新版 — so an unreachable feed, a proxy hint and a
                raw exception arrived as four lines of ordinary grey body copy,
                in the slot where the good news also appears, pushing the two
                buttons around as it grew. It is now the same bordered block the
                update dialog already uses for the same job, below the row rather
                than inside it. */}
            {update.status === "error" ? (
              <p
                className="selectable rounded-control border border-danger/40 bg-danger/10 px-2.5 py-2 text-sm leading-relaxed text-danger"
                role="alert"
              >
                {text(`检查失败：${update.error}`, `Update check failed: ${update.error}`)}
              </p>
            ) : null}
            {update.info && update.status !== "current" ? (
              <div className="selectable grid gap-1 rounded-control border border-line bg-canvas-deep p-2.5">
                <strong className="caption">{text("更新内容", "What's new")} · {update.info.name}</strong>
                <p className="text-sm leading-relaxed whitespace-pre-line text-muted">
                  {update.info.notes ?? text("请前往 GitHub 查看完整更新说明。", "Open GitHub for the full release notes.")}
                </p>
              </div>
            ) : null}
          </div>

          <label className={TOGGLE}>
            <Head
              title={text("启动后自动检查", "Check automatically after launch")}
              detail={text("关闭后将保持离线，直到你手动检查。", "When disabled, MetaClean stays offline until you check manually.")}
            />
            <input className="switch" type="checkbox" checked={update.autoCheckEnabled} onChange={(event) => update.setAutoCheckEnabled(event.target.checked)} />
          </label>

        </> : null}

        {section === "safety" ? <>
          <PanelHead
            title={text("安全保障", "Safety guarantees")}
            detail={text("这些规则由清理引擎强制执行，不依赖界面设置。", "The cleaning engine enforces these rules independently of UI settings.")}
          />
          <Guarantee icon={<RotateCcw size={17} strokeWidth={1.8} />} title={text("替换前始终备份", "Always back up before replacement")}>
            {text("替换模式会先创建 .bak 文件。", "Replace mode creates a .bak file first.")}
          </Guarantee>
          <Guarantee icon={<ShieldCheck size={17} strokeWidth={1.8} />} title={text("拒绝危险写入", "Reject unsafe writes")}>
            {text("拒绝写入符号链接，并限制单文件最大 256 MiB。", "Symlink writes are refused and files are limited to 256 MiB.")}
          </Guarantee>
          <Guarantee icon={<Wrench size={17} strokeWidth={1.8} />} title={text("原子生成输出", "Atomic output")}>
            {text("先写入临时文件，验证后再替换目标。", "Output is written to a temporary file and verified before replacement.")}
          </Guarantee>
        </> : null}
      </div>
    </section>
  );
}

/**
 * The line under the segments that says what this panel is for.
 *
 * The heading it carries is for structure only: the selected segment already
 * states the panel's name an inch above, and printing it twice is the kind of
 * redundancy that makes a settings page feel padded. Screen readers still get
 * the heading, which is the half of the job the segment cannot do.
 */
function PanelHead({ title, detail }: { title: string; detail: string }) {
  return (
    <header className="pb-0.5">
      <h2 className="sr-only">{title}</h2>
      <p className="text-sm leading-relaxed text-muted">{detail}</p>
    </header>
  );
}

function Head({ title, detail }: { title: string; detail?: string }) {
  return (
    <div className="min-w-0 flex-1 grid gap-0.5">
      <h3 className="text-base font-semibold">{title}</h3>
      {detail ? <p className="text-sm leading-relaxed text-muted">{detail}</p> : null}
    </div>
  );
}

function Tile({ selected, onClick, children }: { selected: boolean; onClick: () => void; children: ReactNode }) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={onClick}
      className={[
        "grid h-[62px] place-items-center content-center gap-1 rounded-control border text-sm transition-colors duration-100",
        selected
          ? "border-brand bg-brand/10 font-medium text-brand"
          : "border-line bg-surface-2/60 text-muted hover:border-line-strong hover:bg-surface-2 hover:text-text",
      ].join(" ")}
    >
      {children}
    </button>
  );
}

function Choice({ selected, icon, title, detail, onClick }: { selected: boolean; icon: ReactNode; title: string; detail: string; onClick: () => void }) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={onClick}
      className={[
        "flex items-start gap-2.5 rounded-control border p-2.5 text-left transition-colors duration-100",
        selected ? "border-brand bg-brand/10" : "border-line bg-surface-2/60 hover:border-line-strong hover:bg-surface-2",
      ].join(" ")}
    >
      <span className={`mt-px shrink-0 ${selected ? "text-brand" : "text-muted"}`} aria-hidden="true">{icon}</span>
      <span className="min-w-0 grid gap-0.5">
        <strong className="text-base font-semibold">{title}</strong>
        <small className="text-xs leading-snug text-muted">{detail}</small>
      </span>
    </button>
  );
}

function Check({ checked, onChange, title, detail }: { checked: boolean; onChange: (value: boolean) => void; title: string; detail?: string }) {
  return (
    <label className="flex cursor-pointer items-start gap-2.5 rounded-control px-2 py-1.5 transition-colors duration-100 hover:bg-surface-2">
      {/* `items-start` puts the box at the top of a label that can run to two
          lines, so it needs nudging down onto the first line's optical centre by
          hand: half a 20px line minus half a 16px box is 2px. It was 1px, set
          for the 15px box this used to be. */}
      <input className="check mt-0.5" type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} />
      <span className="min-w-0 grid gap-0.5">
        <strong className="text-base font-medium">{title}</strong>
        {detail ? <small className="text-xs leading-snug text-muted">{detail}</small> : null}
      </span>
    </label>
  );
}

function Guarantee({ icon, title, children }: { icon: ReactNode; title: string; children: ReactNode }) {
  return (
    <div className="flex items-center gap-3 rounded-panel border border-line bg-surface p-3.5 shadow-panel">
      {/* The same mint chip the privacy page gives its three cards, because
          this is the same claim in a different room and a reader should not
          have to work that out. It replaces a grey chip *and* a tick at the far
          end of the row: two accented marks on one row make neither of them
          mean anything, and the tick was the one carrying no information —
          every guarantee here holds, always, which is what the line above the
          list already says. */}
      <span className="grid size-8 shrink-0 place-items-center rounded-control bg-brand/12 text-brand" aria-hidden="true">{icon}</span>
      <div className="min-w-0 flex-1 grid gap-0.5">
        <h3 className="text-base font-semibold">{title}</h3>
        <p className="text-sm leading-relaxed text-muted">{children}</p>
      </div>
    </div>
  );
}
