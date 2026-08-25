import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowUpCircle, FileCheck2, FilePlus2, FolderOpen, History, Moon, MonitorCog, ScanSearch, Settings, ShieldCheck, Sun, Trash2 } from "lucide-react";
import Sidebar from "./components/Sidebar";
import DropZone from "./components/DropZone";
import FileQueue from "./components/FileQueue";
import CleanOptions from "./components/CleanOptions";
import HistoryPage from "./components/HistoryPage";
import PrivacyPage from "./components/PrivacyPage";
import SettingsPage from "./components/SettingsPage";
import UpdateDialog from "./components/UpdateDialog";
import TitleBar from "./components/TitleBar";
import StatusBar from "./components/StatusBar";
import TooltipHost from "./components/TooltipHost";
import CommandPalette, { type Command } from "./components/CommandPalette";
import { actionableFindingCount, applyScanReports, entryFromPath, markEntryPaths, mergeEntries } from "./lib/files";
import { installZoomLock } from "./lib/window";
import { commandKeyLabel } from "./lib/keys";
import { pickPaths } from "./lib/pick";
import { loadHistory, persistHistory } from "./lib/history";
import { readStorage, writeStorage } from "./lib/storage";
import type { CleanMode, FileEntry, HistoryEntry, IntakeResult, Page } from "./types";
import type { CleanResult, ScanReport } from "./types";
import { useI18n } from "./lib/i18n";
import { useTheme } from "./contexts/ThemeContext";
import { useUpdate } from "./contexts/UpdateContext";

export default function App() {
  const { text } = useI18n();
  const update = useUpdate();
  const theme = useTheme();
  const [page, setPage] = useState<Page>("clean");
  const [commandsOpen, setCommandsOpen] = useState(false);
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [mode, setModeState] = useState<CleanMode>(() => readStorage("metaclean.outputMode") === "replace" ? "replace" : "copy");
  const [preserveTimestamps, setPreserveTimestampsState] = useState(() => readStorage("metaclean.preserveTimestamps") !== "false");
  const [preserveOrientation, setPreserveOrientationState] = useState(() => readStorage("metaclean.preserveOrientation") !== "false");
  const [preserveColorProfile, setPreserveColorProfileState] = useState(() => readStorage("metaclean.preserveColorProfile") !== "false");
  const [removeExtendedAttributes, setRemoveExtendedAttributesState] = useState(() => readStorage("metaclean.removeExtendedAttributes") === "true");
  const [closeToTray, setCloseToTrayState] = useState(() => readStorage("metaclean.closeToTray") === "true");
  const [history, setHistory] = useState<HistoryEntry[]>(loadHistory);
  const [busy, setBusy] = useState(false);
  const operationRef = useRef(false);
  const [message, setMessage] = useState<string>();
  const [dragActive, setDragActive] = useState(false);
  const addEntries = useCallback((incoming: FileEntry[]) => setEntries((current) => mergeEntries(current, incoming)), []);
  const addNativePaths = useCallback(async (paths: string[]) => {
    if (!paths.length) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const intake = await invoke<IntakeResult>("expand_paths", { paths });
      addEntries(intake.files.map(entryFromPath));
      if (intake.skippedCount || intake.limitReached) {
        const firstIssue = intake.issues[0];
        setMessage(text(
          `已添加 ${intake.files.length} 个文件，跳过 ${intake.skippedCount} 项。${firstIssue ? ` ${firstIssue.reason}：${firstIssue.path}` : ""}`,
          `Added ${intake.files.length} file(s); skipped ${intake.skippedCount}.${firstIssue ? ` ${firstIssue.reason}: ${firstIssue.path}` : ""}`,
        ));
      }
    } catch (error) {
      setMessage(text(`无法展开所选路径：${String(error)}`, `Could not expand the selected paths: ${String(error)}`));
    }
  }, [addEntries, text]);
  const setMode = useCallback((next: CleanMode) => { setModeState(next); writeStorage("metaclean.outputMode", next); }, []);
  const setPreserveTimestamps = useCallback((next: boolean) => { setPreserveTimestampsState(next); writeStorage("metaclean.preserveTimestamps", String(next)); }, []);
  const setPreserveOrientation = useCallback((next: boolean) => { setPreserveOrientationState(next); writeStorage("metaclean.preserveOrientation", String(next)); }, []);
  const setPreserveColorProfile = useCallback((next: boolean) => { setPreserveColorProfileState(next); writeStorage("metaclean.preserveColorProfile", String(next)); }, []);
  const setRemoveExtendedAttributes = useCallback((next: boolean) => { setRemoveExtendedAttributesState(next); writeStorage("metaclean.removeExtendedAttributes", String(next)); }, []);
  const setCloseToTray = useCallback((next: boolean) => { setCloseToTrayState(next); writeStorage("metaclean.closeToTray", String(next)); }, []);
  const addHistory = useCallback((entry: HistoryEntry) => setHistory((current) => persistHistory([entry, ...current])), []);
  const clearHistory = useCallback(() => setHistory(persistHistory([])), []);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    void import("@tauri-apps/api/webview").then(({ getCurrentWebview }) => getCurrentWebview().onDragDropEvent((event) => {
      setDragActive(event.payload.type === "enter" || event.payload.type === "over");
      if (event.payload.type === "drop") void addNativePaths(event.payload.paths);
    })).then((unlisten) => { dispose = unlisten; }).catch(() => undefined);
    void import("@tauri-apps/api/core").then(({ invoke }) => invoke<string[]>("get_launch_paths"))
      .then((paths) => { if (paths.length) void addNativePaths(paths); })
      .catch(() => undefined);
    return () => dispose?.();
  }, [addNativePaths]);

  useEffect(() => {
    return installZoomLock();
  }, []);

  useEffect(() => {
    void invoke("set_close_to_tray", { enabled: closeToTray })
      .catch(() => undefined);
  }, [closeToTray]);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    void import("@tauri-apps/api/event").then(({ listen }) => listen<Page>("menu:navigate", (event) => {
      if (["clean", "history", "privacy", "settings"].includes(event.payload)) setPage(event.payload);
    })).then((unlisten) => { dispose = unlisten; }).catch(() => undefined);
    return () => dispose?.();
  }, []);

  useEffect(() => {
    const runShortcut = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
      if (event.key.toLowerCase() === "k") {
        event.preventDefault();
        setCommandsOpen((open) => !open);
        return;
      }
      const destination = ({ "1": "clean", "2": "history", "3": "privacy", "4": "settings" } as const)[event.key];
      if (!destination) return;
      event.preventDefault();
      setPage(destination);
    };
    window.addEventListener("keydown", runShortcut);
    return () => window.removeEventListener("keydown", runShortcut);
  }, []);

  const scanned = entries.length > 0 && entries.every((entry) => entry.status === "scanned" || entry.status === "clean" || entry.status === "error");
  const cleanableEntries = entries.filter((entry) => (
    entry.status === "scanned" || (entry.status === "error" && entry.result?.success === false)
  ) && actionableFindingCount(entry.report, preserveColorProfile, removeExtendedAttributes) > 0);

  async function scan() {
    if (operationRef.current) return;
    const paths = entries.flatMap((entry) => entry.path ? [entry.path] : []);
    if (paths.length !== entries.length) { setMessage(text("浏览器模式无法取得完整路径，请在桌面应用中选择文件。", "Browser mode cannot access full paths. Choose files in the desktop app.")); return; }
    operationRef.current = true;
    setBusy(true); setMessage(undefined); setEntries((current) => markEntryPaths(current, paths, "scanning"));
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const reports = await invoke<ScanReport[]>("scan_files", { paths });
      const requested = new Set(paths);
      const relevant = [...new Map(reports
        .filter((report) => requested.has(report.path))
        .map((report) => [report.path, report])).values()];
      setEntries((current) => applyScanReports(current, paths, relevant));
      const count = relevant.reduce((total, report) => total + report.findings.reduce((sum, finding) => sum + finding.count, 0), 0);
      const missing = paths.length - relevant.length;
      setMessage(text(`扫描完成：${count} 项痕迹等待确认。${missing > 0 ? ` ${missing} 个文件未返回结果，可重试扫描。` : ""}`, `Scan complete: ${count} trace(s) await confirmation.${missing > 0 ? ` ${missing} file(s) returned no result and can be retried.` : ""}`));
    } catch (error) { setEntries((current) => markEntryPaths(current, paths, "error")); setMessage(text(`扫描失败：${String(error)}`, `Scan failed: ${String(error)}`)); }
    finally { operationRef.current = false; setBusy(false); }
  }

  async function clean() {
    if (operationRef.current) return;
    const paths = cleanableEntries.flatMap((entry) => entry.path ? [entry.path] : []);
    if (!paths.length) return;
    operationRef.current = true;
    setBusy(true); setMessage(undefined);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const results = await invoke<CleanResult[]>("clean_files", { request: { paths, mode, preserveTimestamps, preserveOrientation, preserveColorProfile, removeExtendedAttributes } });
      const requested = new Set(paths);
      const relevant = [...new Map(results
        .filter((result) => requested.has(result.sourcePath))
        .map((result) => [result.sourcePath, result])).values()];
      const byPath = new Map(relevant.map((result) => [result.sourcePath, result]));
      setEntries((current) => current.map((entry) => {
        const result = entry.path ? byPath.get(entry.path) : undefined;
        return result ? { ...entry, status: result.success ? "clean" : "error", result } : entry;
      }));
      const successes = relevant.filter((result) => result.success);
      const failures = relevant.length - successes.length;
      const missing = paths.length - relevant.length;
      setMessage(text(`${successes.length} 个文件清理完成${failures ? `，${failures} 个失败` : ""}${missing > 0 ? `，${missing} 个未返回结果、可重试` : ""}。${successes[0]?.outputPath ? ` 输出：${successes[0].outputPath}` : ""}`, `${successes.length} file(s) cleaned${failures ? `; ${failures} failed` : ""}${missing > 0 ? `; ${missing} returned no result and can be retried` : ""}.${successes[0]?.outputPath ? ` Output: ${successes[0].outputPath}` : ""}`));
      if (relevant.length) addHistory({ id: crypto.randomUUID(), createdAt: new Date().toISOString(), mode, results: relevant });
    } catch (error) { setMessage(text(`清理失败：${String(error)}`, `Cleanup failed: ${String(error)}`)); }
    finally { operationRef.current = false; setBusy(false); }
  }

  async function reveal(path: string) {
    try {
      const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
      await revealItemInDir(path);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  }

  async function choose(directory: boolean) {
    try {
      const paths = await pickPaths(directory);
      if (paths) await addNativePaths(paths);
    } catch {
      /* Only the desktop build has a system picker; the drop zone has its own fallback. */
    }
  }

  /* Everything the window can do, in one list. The palette searches it, and it
     doubles as the inventory that keeps the accelerators honest. */
  const modifier = commandKeyLabel();
  const go = text("前往", "Go to");
  const act = text("操作", "Actions");
  const appearance = text("外观", "Appearance");
  const commands: Command[] = [
    { id: "go-clean", group: go, label: text("文件净化", "Clean files"), icon: <FileCheck2 size={14} />, accelerator: `${modifier}1`, run: () => setPage("clean") },
    { id: "go-history", group: go, label: text("处理记录", "History"), icon: <History size={14} />, accelerator: `${modifier}2`, run: () => setPage("history") },
    { id: "go-privacy", group: go, label: text("隐私说明", "Privacy"), icon: <ShieldCheck size={14} />, accelerator: `${modifier}3`, run: () => setPage("privacy") },
    { id: "go-settings", group: go, label: text("设置", "Settings"), icon: <Settings size={14} />, accelerator: `${modifier}4`, run: () => setPage("settings") },
    { id: "pick-files", group: act, label: text("选择文件", "Choose files"), icon: <FilePlus2 size={14} />, run: () => void choose(false) },
    { id: "pick-folder", group: act, label: text("选择文件夹", "Choose folder"), icon: <FolderOpen size={14} />, run: () => void choose(true) },
    { id: "scan", group: act, label: text("扫描隐私痕迹", "Scan privacy traces"), icon: <ScanSearch size={14} />, disabled: busy || !entries.length || scanned, run: () => { setPage("clean"); void scan(); } },
    { id: "clean", group: act, label: text("确认并开始清理", "Confirm and clean"), icon: <ShieldCheck size={14} />, disabled: busy || !scanned || !cleanableEntries.length, run: () => { setPage("clean"); void clean(); } },
    { id: "clear", group: act, label: text("清空队列", "Clear queue"), icon: <Trash2 size={14} />, disabled: busy || !entries.length, run: () => setEntries([]) },
    { id: "theme-light", group: appearance, label: text("浅色", "Light"), icon: <Sun size={14} />, disabled: theme.mode === "light", run: () => theme.setMode("light") },
    { id: "theme-dark", group: appearance, label: text("深色", "Dark"), icon: <Moon size={14} />, disabled: theme.mode === "dark", run: () => theme.setMode("dark") },
    { id: "theme-system", group: appearance, label: text("跟随系统", "System"), icon: <MonitorCog size={14} />, disabled: theme.mode === "system", run: () => theme.setMode("system") },
  ];

  const titles: Record<Page, [string, string]> = {
    clean: [text("文件净化", "Clean files"), text("清除文件里的隐私痕迹，分享前更安心", "Remove private traces before sharing")],
    history: [text("处理记录", "History"), text("记录仅保存在此设备的应用数据中，不包含文件内容。", "History stays on this device and never stores file content.")],
    privacy: [text("隐私说明", "Privacy"), text("MetaClean 的处理边界清晰且可验证。", "MetaClean has clear, verifiable processing boundaries.")],
    settings: [text("设置", "Settings"), text("MetaClean · 纯本地文件隐私工具", "MetaClean · Local file privacy tool")],
  };
  const [title, subtitle] = titles[page];

  return (
    <>
    {/* Three fixed bands and one that takes what is left: the title bar and the
        status strip are chrome, and chrome that resizes with the content is the
        thing that makes a window feel like a page. */}
    <div className="app-shell grid h-screen grid-rows-[36px_minmax(0,1fr)_26px] overflow-hidden bg-canvas text-text">
    <TitleBar closeToTray={closeToTray} onOpenCommands={() => setCommandsOpen(true)} />
    <div className="grid min-h-0 grid-cols-[72px_minmax(0,1fr)]">
      <Sidebar page={page} onNavigate={setPage} />
      <main className="flex min-h-0 flex-col overflow-hidden">
        <header className="flex shrink-0 items-start gap-4 px-5 pt-4 pb-3.5">
          <div className="min-w-0 flex-1 grid gap-0.5">
            <h1 className="font-display truncate text-xl font-semibold">{title}</h1>
            <p className="truncate text-sm text-muted">{subtitle}</p>
          </div>
          {update.status === "available" ? (
            <button
              className="mt-0.5 flex h-[26px] shrink-0 items-center gap-1.5 rounded-full border border-brand/45 bg-brand/10 px-2.5 text-sm font-medium text-brand transition-colors duration-100 hover:bg-brand/18"
              type="button"
              onClick={update.showUpdatePrompt}
              aria-label={text(`发现新版本 ${update.info?.availableVersion}`, `Version ${update.info?.availableVersion} is available`)}
            >
              <ArrowUpCircle size={14} strokeWidth={2} />
              <span>{text(`更新至 v${update.info?.availableVersion}`, `Update to v${update.info?.availableVersion}`)}</span>
            </button>
          ) : null}
        </header>
        {/* Keyed on the page so switching remounts, and the new page rises into
            place instead of appearing mid-scroll where the last one left off. */}
        <div className="animate-rise min-h-0 flex-1 px-5 pb-5" key={page}>
        {page === "clean" ? <div className="grid h-full max-w-[1180px] grid-cols-[minmax(0,1fr)_296px] gap-3">
          <div className="flex min-h-0 flex-col gap-3">
            {message ? (
              <div className="shrink-0 rounded-control border border-line bg-surface px-2.5 py-2 text-sm text-muted shadow-panel" role="status">
                {message}
              </div>
            ) : null}
            <DropZone onAdd={addEntries} onAddNativePaths={addNativePaths} dragActive={dragActive} compact={entries.length > 0} />
            <FileQueue entries={entries} preserveColorProfile={preserveColorProfile} removeExtendedAttributes={removeExtendedAttributes} busy={busy} onClear={() => setEntries([])} onRemove={(id) => setEntries((current) => current.filter((entry) => entry.id !== id))} onReveal={(path) => void reveal(path)} onNotify={setMessage} />
          </div>
          <CleanOptions mode={mode} onModeChange={setMode} preserveTimestamps={preserveTimestamps} onPreserveTimestampsChange={setPreserveTimestamps} preserveOrientation={preserveOrientation} onPreserveOrientationChange={setPreserveOrientation} preserveColorProfile={preserveColorProfile} onPreserveColorProfileChange={setPreserveColorProfile} removeExtendedAttributes={removeExtendedAttributes} onRemoveExtendedAttributesChange={setRemoveExtendedAttributes} disabled={!entries.length} scanned={scanned} hasFindings={cleanableEntries.length > 0} busy={busy} onAction={() => void (scanned ? clean() : scan())} />
        </div> : page === "history" ? <HistoryPage entries={history} onClear={clearHistory} /> : page === "privacy" ? <PrivacyPage /> : <SettingsPage mode={mode} onModeChange={setMode} preserveTimestamps={preserveTimestamps} onPreserveTimestampsChange={setPreserveTimestamps} preserveOrientation={preserveOrientation} onPreserveOrientationChange={setPreserveOrientation} preserveColorProfile={preserveColorProfile} onPreserveColorProfileChange={setPreserveColorProfile} removeExtendedAttributes={removeExtendedAttributes} onRemoveExtendedAttributesChange={setRemoveExtendedAttributes} closeToTray={closeToTray} onCloseToTrayChange={setCloseToTray} />}
        </div>
      </main>
    </div>
    <StatusBar busy={busy} fileCount={entries.length} />
    </div>
    <UpdateDialog />
    <TooltipHost />
    {commandsOpen ? <CommandPalette commands={commands} onClose={() => setCommandsOpen(false)} /> : null}
    </>
  );
}
