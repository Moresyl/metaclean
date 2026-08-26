import { useCallback, useEffect, useState } from "react";
import {
  Bug,
  Check,
  Copy,
  Download,
  ExternalLink,
  FileDown,
  FolderOpen,
  Github,
  Lightbulb,
  LoaderCircle,
  RefreshCw,
  Rocket,
  Scale,
} from "lucide-react";
import appIcon from "../../src-tauri/icons/128x128.png";
import Button, { IconButton } from "./Button";
import { useUpdate } from "../contexts/UpdateContext";
import { useI18n } from "../lib/i18n";
import {
  BUG_REPORT_URL,
  FEATURE_REQUEST_URL,
  LICENSE_URL,
  openProjectUrl,
  RELEASES_URL,
  REPOSITORY_URL,
  type ProjectUrl,
} from "../lib/links";
import { buildDiagnosticReport, type AboutInfo } from "../lib/about";

type CopyTarget = "report" | "appData" | "executable";

function browserAboutInfo(): AboutInfo {
  const agent = navigator.userAgent;
  const arch = /arm64|aarch64/iu.test(agent) ? "arm64" : /win64|x86_64|x64/iu.test(agent) ? "x86_64" : "unknown";
  return {
    version: "0.0.0",
    platform: navigator.platform || "browser",
    arch,
  };
}

export default function AboutPage() {
  const { locale, text } = useI18n();
  const update = useUpdate();
  const [about, setAbout] = useState<AboutInfo>();
  const [error, setError] = useState<string>();
  const [copied, setCopied] = useState<CopyTarget>();
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let active = true;
    void import("@tauri-apps/api/core")
      .then(({ invoke }) => invoke<AboutInfo>("get_about_info"))
      .then((value) => { if (active) setAbout(value); })
      .catch((reason) => {
        if (!active) return;
        if (!("__TAURI_INTERNALS__" in window)) {
          setAbout(browserAboutInfo());
          return;
        }
        setError(String(reason));
      });
    return () => { active = false; };
  }, []);

  const report = useCallback(() => about ? buildDiagnosticReport(about, {
    locale,
    updateStatus: update.status,
    availableVersion: update.info?.availableVersion,
    portable: update.runtime.portable,
    selfUpdateSupported: update.runtime.selfUpdateSupported,
  }) : undefined, [about, locale, update.info?.availableVersion, update.runtime.portable, update.runtime.selfUpdateSupported, update.status]);

  const copy = useCallback(async (value: string, target: CopyTarget) => {
    setError(undefined);
    try {
      await navigator.clipboard.writeText(value);
      setCopied(target);
      window.setTimeout(() => setCopied((current) => current === target ? undefined : current), 1_400);
    } catch (reason) {
      setError(text(`复制失败：${String(reason)}`, `Could not copy: ${String(reason)}`));
    }
  }, [text]);

  const copyReport = useCallback(() => {
    const contents = report();
    if (contents) void copy(contents, "report");
  }, [copy, report]);

  const saveReport = useCallback(async () => {
    const contents = report();
    if (!contents) return;
    setSaving(true);
    setError(undefined);
    try {
      const [{ save }, { invoke }] = await Promise.all([
        import("@tauri-apps/plugin-dialog"),
        import("@tauri-apps/api/core"),
      ]);
      const path = await save({
        title: text("保存 MetaClean 诊断信息", "Save MetaClean diagnostics"),
        defaultPath: `MetaClean-diagnostics-${new Date().toISOString().slice(0, 10)}.json`,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path) return;
      await invoke("export_audit_report", { path, contents });
      const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
      await revealItemInDir(path);
    } catch (reason) {
      setError(text(`保存诊断信息失败：${String(reason)}`, `Could not save diagnostics: ${String(reason)}`));
    } finally {
      setSaving(false);
    }
  }, [report, text]);

  const reveal = useCallback(async (path: string) => {
    setError(undefined);
    try {
      const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
      await revealItemInDir(path);
    } catch (reason) {
      setError(text(`无法打开文件夹：${String(reason)}`, `Could not reveal the folder: ${String(reason)}`));
    }
  }, [text]);

  const openLink = useCallback(async (url: ProjectUrl) => {
    setError(undefined);
    try {
      await openProjectUrl(url);
    } catch (reason) {
      setError(text(`无法打开链接：${String(reason)}`, `Could not open the link: ${String(reason)}`));
    }
  }, [text]);

  const updateBusy = update.status === "checking" || update.status === "updating";
  const percent = update.progress?.total && update.progress.total > 0
    ? Math.min(100, Math.round((update.progress.downloaded / update.progress.total) * 100))
    : undefined;

  return (
    <section className="h-full overflow-y-auto pr-1">
      <div className="mx-auto grid max-w-[720px] gap-3.5 pb-1">
        <div className="flex items-center gap-4 rounded-panel border border-line bg-surface px-4 py-3.5 shadow-panel">
          <img className="size-14 shrink-0 rounded-[12px] shadow-lift" src={appIcon} alt="" />
          <div className="min-w-0 flex-1 grid gap-1">
            <h2 className="font-display text-lg font-semibold">MetaClean</h2>
            <p className="font-mono text-sm text-muted tabular-nums">
              {about ? `v${about.version} · ${about.platform}-${about.arch}` : text("正在读取运行信息…", "Reading runtime information…")}
            </p>
            <p className="text-sm text-muted">{text("纯本地、开源的文件隐私净化工具", "Local, open-source file privacy cleaner")}</p>
          </div>
          <Button size="sm" disabled={updateBusy} onClick={() => void update.checkUpdate()}>
            <RefreshCw size={14} className={update.status === "checking" ? "animate-spin" : ""} />
            {update.status === "checking" ? text("检查中…", "Checking…") : text("检查更新", "Check updates")}
          </Button>
        </div>

        <UpdateCard update={update} percent={percent} />

        <section className="grid gap-2">
          <h2 className="caption">{text("应用位置", "Application paths")}</h2>
          <dl className="divide-y divide-line overflow-hidden rounded-panel border border-line bg-surface shadow-panel">
            <PathRow
              label={text("应用数据", "Application data")}
              path={about?.appDataDir}
              copied={copied === "appData"}
              onCopy={(path) => void copy(path, "appData")}
              onReveal={(path) => void reveal(path)}
            />
            <PathRow
              label={text("程序目录", "Executable directory")}
              path={about?.executableDir}
              copied={copied === "executable"}
              onCopy={(path) => void copy(path, "executable")}
              onReveal={(path) => void reveal(path)}
            />
          </dl>
        </section>

        <section className="grid gap-2">
          <h2 className="caption">{text("诊断与支持", "Diagnostics and support")}</h2>
          <div className="grid gap-3 rounded-panel border border-line bg-surface p-3.5 shadow-panel">
            <div className="grid gap-1">
              <h3 className="text-base font-semibold">{text("生成可复现的环境信息", "Create reproducible environment details")}</h3>
              <p className="text-sm leading-relaxed text-muted">
                {text("包含版本、平台、架构、更新方式和应用目录，不包含已处理文件、历史记录或文件内容。", "Includes version, platform, architecture, update mode and application paths—never processed files, history or file contents.")}
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Button size="sm" disabled={!about} onClick={copyReport}>
                {copied === "report" ? <Check size={14} className="text-ok" /> : <Copy size={14} />}
                {copied === "report" ? text("已复制", "Copied") : text("复制诊断信息", "Copy diagnostics")}
              </Button>
              <Button size="sm" disabled={!about || saving} onClick={() => void saveReport()}>
                {saving ? <LoaderCircle size={14} className="animate-spin" /> : <FileDown size={14} />}
                {saving ? text("保存中…", "Saving…") : text("保存 JSON", "Save JSON")}
              </Button>
            </div>
          </div>
        </section>

        <section className="grid gap-2">
          <h2 className="caption">{text("社区与项目", "Community and project")}</h2>
          <div className="grid grid-cols-3 gap-2">
            <CommunityLink icon={Bug} label={text("报告问题", "Report a bug")} onClick={() => void openLink(BUG_REPORT_URL)} />
            <CommunityLink icon={Lightbulb} label={text("功能建议", "Request a feature")} onClick={() => void openLink(FEATURE_REQUEST_URL)} />
            <CommunityLink icon={Rocket} label={text("正式版本", "Releases")} onClick={() => void openLink(RELEASES_URL)} />
          </div>
        </section>

        <footer className="flex items-center gap-4 px-1 text-sm text-faint">
          <button className="inline-flex items-center gap-1.5 hover:text-brand" type="button" onClick={() => void openLink(REPOSITORY_URL)}>
            <Github size={13} />{text("源代码", "Source code")}<ExternalLink size={11} />
          </button>
          <button className="inline-flex items-center gap-1.5 hover:text-brand" type="button" onClick={() => void openLink(LICENSE_URL)}>
            <Scale size={13} />MIT License<ExternalLink size={11} />
          </button>
        </footer>

        {error || update.error ? (
          <p className="selectable rounded-control border border-danger/40 bg-danger/10 px-3 py-2 text-sm leading-relaxed text-danger" role="alert">
            {error ?? update.error}
          </p>
        ) : null}
      </div>
    </section>
  );
}

function UpdateCard({ update, percent }: { update: ReturnType<typeof useUpdate>; percent?: number }) {
  const { text } = useI18n();
  if (update.status === "idle" || update.status === "checking") return null;

  if (update.status === "available" || update.status === "updating") {
    return (
      <section className="grid gap-2.5 rounded-panel border border-brand/35 bg-brand/10 p-3.5">
        <div className="flex items-start gap-3">
          <div className="min-w-0 flex-1 grid gap-1">
            <h2 className="text-base font-semibold">{text(`发现新版本 v${update.info?.availableVersion}`, `Version ${update.info?.availableVersion} is available`)}</h2>
            <p className="max-h-24 overflow-y-auto whitespace-pre-line text-sm leading-relaxed text-muted">
              {update.info?.notes ?? text("可通过签名更新器安装，或打开正式版本页面查看完整说明。", "Install with the signed updater or open the release page for complete notes.")}
            </p>
          </div>
          <div className="flex shrink-0 gap-2">
            <Button size="sm" onClick={() => void update.openRelease()}><ExternalLink size={14} />{text("版本说明", "Release notes")}</Button>
            <Button size="sm" variant="primary" disabled={update.status === "updating"} onClick={() => void update.installUpdate()}>
              {update.status === "updating" ? <LoaderCircle size={14} className="animate-spin" /> : update.runtime.selfUpdateSupported ? <Download size={14} /> : <ExternalLink size={14} />}
              {update.status === "updating" ? text("更新中…", "Updating…") : update.runtime.selfUpdateSupported ? text("安装更新", "Install update") : text("前往下载", "Open download")}
            </Button>
          </div>
        </div>
        {update.status === "updating" ? <progress className="progress" max={100} value={percent} aria-label={text("更新进度", "Update progress")} /> : null}
      </section>
    );
  }

  if (update.status === "current") {
    return <p className="rounded-panel border border-line bg-surface px-3.5 py-3 text-sm text-muted shadow-panel">{text(`当前 v${update.currentVersion} 已是最新版。`, `Version ${update.currentVersion} is up to date.`)}</p>;
  }

  return null;
}

function PathRow({
  label,
  path,
  copied,
  onCopy,
  onReveal,
}: {
  label: string;
  path?: string;
  copied: boolean;
  onCopy: (path: string) => void;
  onReveal: (path: string) => void;
}) {
  const { text } = useI18n();
  return (
    <div className="flex min-h-10 items-center gap-3 px-3">
      <dt className="shrink-0 text-sm text-muted">{label}</dt>
      <dd className="ml-auto flex min-w-0 items-center gap-1">
        {path ? (
          <>
            <button className="flex min-w-0 items-center gap-1.5 font-mono text-sm text-muted hover:text-brand" type="button" title={path} onClick={() => onReveal(path)}>
              <FolderOpen size={13} className="shrink-0 text-faint" />
              <span className="truncate">{path}</span>
            </button>
            <IconButton size="sm" aria-label={text(`复制${label}`, `Copy ${label}`)} title={text("复制路径", "Copy path")} onClick={() => onCopy(path)}>
              {copied ? <Check size={13} className="text-ok" /> : <Copy size={13} />}
            </IconButton>
          </>
        ) : <span className="font-mono text-sm text-faint">—</span>}
      </dd>
    </div>
  );
}

function CommunityLink({ icon: Icon, label, onClick }: { icon: typeof Bug; label: string; onClick: () => void }) {
  return (
    <button className="flex items-center justify-center gap-2 rounded-panel border border-line bg-surface px-3 py-3 text-sm text-muted shadow-panel transition-colors hover:border-line-strong hover:text-brand" type="button" onClick={onClick}>
      <Icon size={15} />{label}<ExternalLink size={11} />
    </button>
  );
}
