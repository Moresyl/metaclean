import { useCallback, useEffect, useState } from "react";
import { ArrowUpCircle, Sparkles } from "lucide-react";
import Sidebar from "./components/Sidebar";
import DropZone from "./components/DropZone";
import FileQueue from "./components/FileQueue";
import CleanOptions from "./components/CleanOptions";
import HistoryPage from "./components/HistoryPage";
import PrivacyPage from "./components/PrivacyPage";
import SettingsPage from "./components/SettingsPage";
import UpdateDialog from "./components/UpdateDialog";
import { actionableFindingCount, entryFromPath, mergeEntries } from "./lib/files";
import { installZoomLock } from "./lib/window";
import type { CleanMode, FileEntry, HistoryEntry, IntakeResult, Page } from "./types";
import type { CleanResult, ScanReport } from "./types";
import { useI18n } from "./lib/i18n";
import { useUpdate } from "./contexts/UpdateContext";

export default function App() {
  const { text } = useI18n();
  const update = useUpdate();
  const [page, setPage] = useState<Page>("clean");
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [mode, setModeState] = useState<CleanMode>(() => localStorage.getItem("metaclean.outputMode") === "replace" ? "replace" : "copy");
  const [preserveTimestamps, setPreserveTimestampsState] = useState(() => localStorage.getItem("metaclean.preserveTimestamps") !== "false");
  const [preserveOrientation, setPreserveOrientationState] = useState(() => localStorage.getItem("metaclean.preserveOrientation") !== "false");
  const [preserveColorProfile, setPreserveColorProfileState] = useState(() => localStorage.getItem("metaclean.preserveColorProfile") !== "false");
  const [removeExtendedAttributes, setRemoveExtendedAttributesState] = useState(() => localStorage.getItem("metaclean.removeExtendedAttributes") === "true");
  const [history, setHistory] = useState<HistoryEntry[]>(() => {
    try { return JSON.parse(localStorage.getItem("metaclean.history") ?? "[]") as HistoryEntry[]; } catch { return []; }
  });
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string>();
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
  const setMode = useCallback((next: CleanMode) => { setModeState(next); localStorage.setItem("metaclean.outputMode", next); }, []);
  const setPreserveTimestamps = useCallback((next: boolean) => { setPreserveTimestampsState(next); localStorage.setItem("metaclean.preserveTimestamps", String(next)); }, []);
  const setPreserveOrientation = useCallback((next: boolean) => { setPreserveOrientationState(next); localStorage.setItem("metaclean.preserveOrientation", String(next)); }, []);
  const setPreserveColorProfile = useCallback((next: boolean) => { setPreserveColorProfileState(next); localStorage.setItem("metaclean.preserveColorProfile", String(next)); }, []);
  const setRemoveExtendedAttributes = useCallback((next: boolean) => { setRemoveExtendedAttributesState(next); localStorage.setItem("metaclean.removeExtendedAttributes", String(next)); }, []);
  const saveHistory = useCallback((next: HistoryEntry[]) => { const limited = next.slice(0, 100); setHistory(limited); localStorage.setItem("metaclean.history", JSON.stringify(limited)); }, []);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    void import("@tauri-apps/api/webview").then(({ getCurrentWebview }) => getCurrentWebview().onDragDropEvent((event) => {
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
    let dispose: (() => void) | undefined;
    void import("@tauri-apps/api/event").then(({ listen }) => listen<Page>("menu:navigate", (event) => {
      if (["clean", "history", "privacy", "settings"].includes(event.payload)) setPage(event.payload);
    })).then((unlisten) => { dispose = unlisten; }).catch(() => undefined);
    return () => dispose?.();
  }, []);

  useEffect(() => {
    const navigateWithShortcut = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
      const destination = ({ "1": "clean", "2": "history", "3": "privacy", "4": "settings" } as const)[event.key];
      if (!destination) return;
      event.preventDefault();
      setPage(destination);
    };
    window.addEventListener("keydown", navigateWithShortcut);
    return () => window.removeEventListener("keydown", navigateWithShortcut);
  }, []);

  const scanned = entries.length > 0 && entries.every((entry) => entry.status === "scanned" || entry.status === "clean" || entry.status === "error");
  const cleanableEntries = entries.filter((entry) => entry.status === "scanned" && actionableFindingCount(entry.report, preserveColorProfile, removeExtendedAttributes) > 0);

  async function scan() {
    const paths = entries.flatMap((entry) => entry.path ? [entry.path] : []);
    if (paths.length !== entries.length) { setMessage(text("浏览器模式无法取得完整路径，请在桌面应用中选择文件。", "Browser mode cannot access full paths. Choose files in the desktop app.")); return; }
    setBusy(true); setMessage(undefined); setEntries((current) => current.map((entry) => ({ ...entry, status: "scanning" })));
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const reports = await invoke<ScanReport[]>("scan_files", { paths });
      const byPath = new Map(reports.map((report) => [report.path, report]));
      setEntries((current) => current.map((entry) => { const report = entry.path ? byPath.get(entry.path) : undefined; return { ...entry, report, status: report?.error ? "error" : "scanned" }; }));
      const count = reports.reduce((total, report) => total + report.findings.reduce((sum, finding) => sum + finding.count, 0), 0);
      setMessage(text(`扫描完成：${count} 项痕迹等待确认。`, `Scan complete: ${count} trace(s) await confirmation.`));
    } catch (error) { setEntries((current) => current.map((entry) => ({ ...entry, status: "error" }))); setMessage(text(`扫描失败：${String(error)}`, `Scan failed: ${String(error)}`)); }
    finally { setBusy(false); }
  }

  async function clean() {
    const paths = cleanableEntries.flatMap((entry) => entry.path ? [entry.path] : []);
    if (!paths.length) return;
    setBusy(true); setMessage(undefined);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const results = await invoke<CleanResult[]>("clean_files", { request: { paths, mode, preserveTimestamps, preserveOrientation, preserveColorProfile, removeExtendedAttributes } });
      const byPath = new Map(results.map((result) => [result.sourcePath, result]));
      setEntries((current) => current.map((entry) => {
        const result = entry.path ? byPath.get(entry.path) : undefined;
        return result ? { ...entry, status: result.success ? "clean" : "error", result } : entry;
      }));
      const successes = results.filter((result) => result.success);
      const failures = results.length - successes.length;
      setMessage(text(`${successes.length} 个文件清理完成${failures ? `，${failures} 个失败` : ""}。${successes[0]?.outputPath ? ` 输出：${successes[0].outputPath}` : ""}`, `${successes.length} file(s) cleaned${failures ? `; ${failures} failed` : ""}.${successes[0]?.outputPath ? ` Output: ${successes[0].outputPath}` : ""}`));
      saveHistory([{ id: crypto.randomUUID(), createdAt: new Date().toISOString(), mode, results }, ...history]);
    } catch (error) { setMessage(text(`清理失败：${String(error)}`, `Cleanup failed: ${String(error)}`)); }
    finally { setBusy(false); }
  }

  async function reveal(path: string) {
    try {
      const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
      await revealItemInDir(path);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  }

  return (
    <>
    <div className="app-shell">
      <Sidebar page={page} onNavigate={setPage} />
      <main className="workspace">
        <header className="topbar"><div><h1>{page === "clean" ? text("文件净化", "Clean files") : page === "history" ? text("处理记录", "History") : page === "privacy" ? text("隐私说明", "Privacy") : text("设置", "Settings")}</h1><p>{page === "clean" ? text("清除文件里的隐私痕迹，分享前更安心", "Remove private traces before sharing") : text("MetaClean · 纯本地文件隐私工具", "MetaClean · Local file privacy tool")}</p></div>{update.status === "available" ? <button className="update-badge" type="button" onClick={update.showUpdatePrompt} aria-label={text(`发现新版本 ${update.info?.availableVersion}`, `Version ${update.info?.availableVersion} is available`)}><ArrowUpCircle size={16}/><span>{text(`更新至 v${update.info?.availableVersion}`, `Update to v${update.info?.availableVersion}`)}</span></button> : null}</header>
        {page === "clean" ? <div className="content-grid">
          <div className="main-column">
            <div className="notice"><Sparkles size={15} /><span><strong>{text("所有处理均在本机完成。", "All processing happens locally. ")}</strong>{text("MetaClean 不上传、不保存、也不分析你的文件内容。", "MetaClean never uploads, stores, or analyzes your file content.")}</span></div>
            {message ? <div className="result-message" role="status">{message}</div> : null}
            <DropZone onAdd={addEntries} onAddNativePaths={addNativePaths} />
            <FileQueue entries={entries} preserveColorProfile={preserveColorProfile} removeExtendedAttributes={removeExtendedAttributes} onClear={() => setEntries([])} onRemove={(id) => setEntries((current) => current.filter((entry) => entry.id !== id))} onReveal={(path) => void reveal(path)} />
          </div>
          <CleanOptions mode={mode} onModeChange={setMode} preserveTimestamps={preserveTimestamps} onPreserveTimestampsChange={setPreserveTimestamps} preserveOrientation={preserveOrientation} onPreserveOrientationChange={setPreserveOrientation} preserveColorProfile={preserveColorProfile} onPreserveColorProfileChange={setPreserveColorProfile} removeExtendedAttributes={removeExtendedAttributes} onRemoveExtendedAttributesChange={setRemoveExtendedAttributes} disabled={!entries.length} scanned={scanned} hasFindings={cleanableEntries.length > 0} busy={busy} onAction={() => void (scanned ? clean() : scan())} />
        </div> : page === "history" ? <HistoryPage entries={history} onClear={() => saveHistory([])} /> : page === "privacy" ? <PrivacyPage /> : <SettingsPage mode={mode} onModeChange={setMode} preserveTimestamps={preserveTimestamps} onPreserveTimestampsChange={setPreserveTimestamps} preserveOrientation={preserveOrientation} onPreserveOrientationChange={setPreserveOrientation} preserveColorProfile={preserveColorProfile} onPreserveColorProfileChange={setPreserveColorProfile} removeExtendedAttributes={removeExtendedAttributes} onRemoveExtendedAttributesChange={setRemoveExtendedAttributes} />}
      </main>
    </div>
    <UpdateDialog />
    </>
  );
}
