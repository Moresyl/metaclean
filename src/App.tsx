import { useCallback, useEffect, useState } from "react";
import { Sparkles } from "lucide-react";
import Sidebar from "./components/Sidebar";
import DropZone from "./components/DropZone";
import FileQueue from "./components/FileQueue";
import CleanOptions from "./components/CleanOptions";
import HistoryPage from "./components/HistoryPage";
import PrivacyPage from "./components/PrivacyPage";
import SettingsPage from "./components/SettingsPage";
import { entryFromPath, mergeEntries } from "./lib/files";
import type { CleanMode, FileEntry, HistoryEntry, Page } from "./types";
import type { CleanResult, ScanReport } from "./types";

export default function App() {
  const [page, setPage] = useState<Page>("clean");
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [mode, setModeState] = useState<CleanMode>(() => localStorage.getItem("metaclean.outputMode") === "replace" ? "replace" : "copy");
  const [history, setHistory] = useState<HistoryEntry[]>(() => {
    try { return JSON.parse(localStorage.getItem("metaclean.history") ?? "[]") as HistoryEntry[]; } catch { return []; }
  });
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string>();
  const addEntries = useCallback((incoming: FileEntry[]) => setEntries((current) => mergeEntries(current, incoming)), []);
  const setMode = useCallback((next: CleanMode) => { setModeState(next); localStorage.setItem("metaclean.outputMode", next); }, []);
  const saveHistory = useCallback((next: HistoryEntry[]) => { const limited = next.slice(0, 100); setHistory(limited); localStorage.setItem("metaclean.history", JSON.stringify(limited)); }, []);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    void import("@tauri-apps/api/webview").then(({ getCurrentWebview }) => getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "drop") addEntries(event.payload.paths.map(entryFromPath));
    })).then((unlisten) => { dispose = unlisten; }).catch(() => undefined);
    return () => dispose?.();
  }, [addEntries]);

  const scanned = entries.length > 0 && entries.every((entry) => entry.status === "scanned" || entry.status === "clean" || entry.status === "error");

  async function scan() {
    const paths = entries.flatMap((entry) => entry.path ? [entry.path] : []);
    if (paths.length !== entries.length) { setMessage("浏览器模式无法取得完整路径，请在桌面应用中选择文件。"); return; }
    setBusy(true); setMessage(undefined); setEntries((current) => current.map((entry) => ({ ...entry, status: "scanning" })));
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const reports = await invoke<ScanReport[]>("scan_files", { paths });
      const byPath = new Map(reports.map((report) => [report.path, report]));
      setEntries((current) => current.map((entry) => { const report = entry.path ? byPath.get(entry.path) : undefined; return { ...entry, report, status: report?.error ? "error" : "scanned" }; }));
      setMessage(`扫描完成：${reports.reduce((total, report) => total + report.findings.reduce((sum, finding) => sum + finding.count, 0), 0)} 项痕迹等待确认。`);
    } catch (error) { setEntries((current) => current.map((entry) => ({ ...entry, status: "error" }))); setMessage(`扫描失败：${String(error)}`); }
    finally { setBusy(false); }
  }

  async function clean() {
    const paths = entries.flatMap((entry) => entry.path ? [entry.path] : []);
    setBusy(true); setMessage(undefined);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const results = await invoke<CleanResult[]>("clean_files", { request: { paths, mode } });
      const byPath = new Map(results.map((result) => [result.sourcePath, result]));
      setEntries((current) => current.map((entry) => ({ ...entry, status: entry.path && byPath.get(entry.path)?.success ? "clean" : "error" })));
      const successes = results.filter((result) => result.success);
      const failures = results.length - successes.length;
      setMessage(`${successes.length} 个文件清理完成${failures ? `，${failures} 个失败` : ""}。${successes[0]?.outputPath ? ` 输出：${successes[0].outputPath}` : ""}`);
      saveHistory([{ id: crypto.randomUUID(), createdAt: new Date().toISOString(), mode, results }, ...history]);
    } catch (error) { setMessage(`清理失败：${String(error)}`); }
    finally { setBusy(false); }
  }

  return (
    <div className="app-shell">
      <Sidebar page={page} onNavigate={setPage} />
      <main className="workspace">
        <header className="topbar"><div><h1>{page === "clean" ? "文件净化" : page === "history" ? "处理记录" : page === "privacy" ? "隐私说明" : "设置"}</h1><p>{page === "clean" ? "清除文件里的隐私痕迹，分享前更安心" : "MetaClean · 纯本地文件隐私工具"}</p></div></header>
        {page === "clean" ? <div className="content-grid">
          <div className="main-column">
            <div className="notice"><Sparkles size={15} /><span><strong>所有处理均在本机完成。</strong>MetaClean 不上传、不保存、也不分析你的文件内容。</span></div>
            {message ? <div className="result-message" role="status">{message}</div> : null}
            <DropZone onAdd={addEntries} />
            <FileQueue entries={entries} onClear={() => setEntries([])} onRemove={(id) => setEntries((current) => current.filter((entry) => entry.id !== id))} />
          </div>
          <CleanOptions mode={mode} onModeChange={setMode} disabled={!entries.length} scanned={scanned} busy={busy} onAction={() => void (scanned ? clean() : scan())} />
        </div> : page === "history" ? <HistoryPage entries={history} onClear={() => saveHistory([])} /> : page === "privacy" ? <PrivacyPage /> : <SettingsPage mode={mode} onModeChange={setMode} />}
      </main>
    </div>
  );
}
