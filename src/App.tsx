import { lazy, Suspense, useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowUpCircle, CircleHelp, FileCheck2, FilePlus2, FolderOpen, History, Moon, MonitorCog, ScanSearch, Settings, ShieldCheck, Sun, Trash2 } from "lucide-react";
import Sidebar from "./components/Sidebar";
import DropZone from "./components/DropZone";
import FileQueue from "./components/FileQueue";
import CleanOptions from "./components/CleanOptions";
import UpdateDialog from "./components/UpdateDialog";
import ConfirmDialog from "./components/ConfirmDialog";
import TitleBar from "./components/TitleBar";
import StatusBar from "./components/StatusBar";
import TooltipHost from "./components/TooltipHost";
import CommandPalette, { type Command } from "./components/CommandPalette";
import { actionableFindingCount, applyScanReports, entryFromPath, markEntryPaths, mergeEntries, normalizeBrowserFiles, pathIdentity, sumFindingCounts } from "./lib/files";
import { installZoomLock } from "./lib/window";
import { commandKeyLabel } from "./lib/keys";
import { loadHistory, persistHistory } from "./lib/history";
import { readStorage, writeStorage } from "./lib/storage";
import { clearActiveBatch, readActiveBatch, updateActiveBatchProgress, writeActiveBatch } from "./lib/recovery";
import type { BatchProgress, CleanMode, FileEntry, HistoryEntry, Page } from "./types";
import { useI18n } from "./lib/i18n";
import { useTheme } from "./contexts/ThemeContext";
import { useUpdate } from "./contexts/UpdateContext";
import { boundedErrorMessage } from "./lib/errors";
import { normalizeBatchProgress } from "./lib/progress";
import { normalizeNativeDropEvent } from "./lib/drag";
import { normalizeIntakeResult, normalizePathList } from "./lib/intake";
import { normalizeCleanResults, normalizeScanReports } from "./lib/results";
import { normalizeCloseBlocked, normalizeNavigationPage } from "./lib/events";
import { PickerUnavailableError, pickPaths } from "./lib/pick";

const HistoryPage = lazy(() => import("./components/HistoryPage"));
const PrivacyPage = lazy(() => import("./components/PrivacyPage"));
const SettingsPage = lazy(() => import("./components/SettingsPage"));
const AboutPage = lazy(() => import("./components/AboutPage"));

export default function App() {
  const { text } = useI18n();
  const update = useUpdate();
  const theme = useTheme();
  const [page, setPage] = useState<Page>("clean");
  const [commandsOpen, setCommandsOpen] = useState(false);
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const entriesRef = useRef<FileEntry[]>([]);
  const [mode, setModeState] = useState<CleanMode>(() => readStorage("metaclean.outputMode") === "replace" ? "replace" : "copy");
  const [preserveTimestamps, setPreserveTimestampsState] = useState(() => readStorage("metaclean.preserveTimestamps") !== "false");
  const [preserveOrientation, setPreserveOrientationState] = useState(() => readStorage("metaclean.preserveOrientation") !== "false");
  const [preserveColorProfile, setPreserveColorProfileState] = useState(() => readStorage("metaclean.preserveColorProfile") !== "false");
  const [removeExtendedAttributes, setRemoveExtendedAttributesState] = useState(() => readStorage("metaclean.removeExtendedAttributes") === "true");
  const [closeToTray, setCloseToTrayState] = useState(() => readStorage("metaclean.closeToTray") === "true");
  const [history, setHistory] = useState<HistoryEntry[]>(loadHistory);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<BatchProgress>();
  const [cancelRequested, setCancelRequested] = useState(false);
  const [activeBatchId, setActiveBatchId] = useState<string>();
  const [activeOperation, setActiveOperation] = useState<"scan" | "clean">();
  const operationRef = useRef(false);
  const operationKindRef = useRef<"scan" | "clean" | undefined>(undefined);
  const batchIdRef = useRef<string | undefined>(undefined);
  const cancelRequestedRef = useRef(false);
  const recoveryProgressRef = useRef<{ batchId?: string; completed: number; persistedAt: number }>({ completed: 0, persistedAt: 0 });
  const [recoveryNotice] = useState(() => readActiveBatch());
  const [message, setMessage] = useState<string>();
  const pendingMergeSkippedRef = useRef(0);
  const [queueClearPromptOpen, setQueueClearPromptOpen] = useState(false);
  const [dragActive, setDragActive] = useState(false);
  const mountedRef = useRef(true);
  const pickerInputRef = useRef<HTMLInputElement>(null);
  const pickerBusyRef = useRef(false);
  entriesRef.current = entries;
  const updateEntries = useCallback((next: FileEntry[] | ((current: FileEntry[]) => FileEntry[])) => {
    const value = typeof next === "function" ? next(entriesRef.current) : next;
    entriesRef.current = value;
    setEntries(value);
  }, []);
  const addEntries = useCallback((incoming: FileEntry[]) => {
    if (!incoming.length) return;
    const merged = mergeEntries(entriesRef.current, incoming);
    pendingMergeSkippedRef.current += merged.skipped;
    updateEntries(merged.entries);
  }, [updateEntries]);
  const reportPickerError = useCallback((error: unknown) => {
    if (!mountedRef.current) return;
    const detail = boundedErrorMessage(error);
    setMessage(text(`选择器返回了无效数据：${detail}`, `The picker returned invalid data: ${detail}`));
  }, [text]);
  const addBrowserFiles = useCallback((files: FileList | null) => {
    if (!files) return;
    let length: number;
    try {
      length = files.length;
    } catch {
      reportPickerError(new Error("浏览器选择无效 / The browser selection is invalid"));
      return;
    }
    const incoming = normalizeBrowserFiles(files);
    if (length > 0 && incoming.length === 0) {
      setMessage(text("浏览器选择无效或超过 10,000 个文件。", "The browser selection is invalid or exceeds 10,000 files."));
      return;
    }
    addEntries(incoming);
  }, [addEntries, reportPickerError, text]);
  const removeEntry = useCallback((id: string) => {
    updateEntries((current) => current.filter((entry) => entry.id !== id));
  }, [updateEntries]);
  const clearQueue = useCallback(() => {
    pendingMergeSkippedRef.current = 0;
    updateEntries([]);
  }, [updateEntries]);
  const addNativePaths = useCallback(async (paths: string[]) => {
    if (!mountedRef.current || !paths.length) return;
    try {
      const intake = normalizeIntakeResult(await invoke<unknown>("expand_paths", { paths }));
      if (!intake) throw new Error(text("路径展开返回了无效数据", "Path expansion returned invalid data"));
      if (!mountedRef.current) return;
      addEntries(intake.files.map(entryFromPath));
      if (intake.skippedCount || intake.limitReached) {
        const firstIssue = intake.issues[0];
        setMessage(text(
          `已添加 ${intake.files.length} 个文件，跳过 ${intake.skippedCount} 项。${firstIssue ? ` ${firstIssue.reason}：${firstIssue.path}` : ""}`,
          `Added ${intake.files.length} file(s); skipped ${intake.skippedCount}.${firstIssue ? ` ${firstIssue.reason}: ${firstIssue.path}` : ""}`,
        ));
      }
    } catch (error) {
      if (!mountedRef.current) return;
      const detail = boundedErrorMessage(error);
      setMessage(text(`无法展开所选路径：${detail}`, `Could not expand the selected paths: ${detail}`));
    }
  }, [addEntries, text]);
  const openPicker = useCallback(async (directory: boolean) => {
    if (pickerBusyRef.current) return;
    pickerBusyRef.current = true;
    try {
      const paths = await pickPaths(directory);
      if (paths) await addNativePaths(paths);
    } catch (error) {
      if (error instanceof PickerUnavailableError) {
        pickerInputRef.current?.toggleAttribute("webkitdirectory", directory);
        pickerInputRef.current?.click();
        return;
      }
      reportPickerError(error);
    } finally {
      pickerBusyRef.current = false;
    }
  }, [addNativePaths, reportPickerError]);
  const setMode = useCallback((next: CleanMode) => { setModeState(next); writeStorage("metaclean.outputMode", next); }, []);
  const setPreserveTimestamps = useCallback((next: boolean) => { setPreserveTimestampsState(next); writeStorage("metaclean.preserveTimestamps", String(next)); }, []);
  const setPreserveOrientation = useCallback((next: boolean) => { setPreserveOrientationState(next); writeStorage("metaclean.preserveOrientation", String(next)); }, []);
  const setPreserveColorProfile = useCallback((next: boolean) => { setPreserveColorProfileState(next); writeStorage("metaclean.preserveColorProfile", String(next)); }, []);
  const setRemoveExtendedAttributes = useCallback((next: boolean) => { setRemoveExtendedAttributesState(next); writeStorage("metaclean.removeExtendedAttributes", String(next)); }, []);
  const setCloseToTray = useCallback((next: boolean) => { setCloseToTrayState(next); writeStorage("metaclean.closeToTray", String(next)); }, []);
  const addHistory = useCallback((entry: HistoryEntry) => setHistory((current) => persistHistory([entry, ...current])), []);
  const clearHistory = useCallback(() => setHistory(persistHistory([])), []);

  useEffect(() => {
    if (!recoveryNotice) return;
    setMessage(text(
      `上次清理可能在 ${recoveryNotice.completed}/${recoveryNotice.total} 个文件后被中断；为安全起见不会自动恢复文件操作，请重新导入并扫描。`,
      `The previous cleanup may have stopped after ${recoveryNotice.completed}/${recoveryNotice.total} file(s); file operations are not resumed automatically. Re-import and scan to continue safely.`,
    ));
    clearActiveBatch(recoveryNotice.batchId);
  }, [recoveryNotice, text]);

  useEffect(() => {
    const skipped = pendingMergeSkippedRef.current;
    if (!skipped) return;
    pendingMergeSkippedRef.current = 0;
    setMessage(text(
      `队列最多保留 10,000 个唯一文件，${skipped} 个新文件未加入；可先处理当前队列后再继续导入。`,
      `The queue keeps at most 10,000 unique files; ${skipped} new file(s) were not added. Process the current queue before importing more.`,
    ));
  }, [entries, text]);

  useEffect(() => {
    let active = true;
    let dispose: (() => void) | undefined;
    void import("@tauri-apps/api/webview").then(({ getCurrentWebview }) => getCurrentWebview().onDragDropEvent((event) => {
      if (!active) return;
      const payload = normalizeNativeDropEvent(event.payload);
      if (!payload) return;
      setDragActive(payload.type === "enter" || payload.type === "over");
      if (payload.type === "drop" && payload.paths.length) void addNativePaths(payload.paths);
    })).then((unlisten) => {
      if (active) dispose = unlisten;
      else unlisten();
    }).catch(() => undefined);
    void invoke<unknown>("get_launch_paths")
      .then((value) => {
        if (!active) return;
        const paths = normalizePathList(value);
        if (!paths) {
          setMessage(text("启动路径返回了无效数据。", "The launch-path response was invalid."));
          return;
        }
        if (paths.length) void addNativePaths(paths);
      })
      .catch(() => undefined);
    return () => { active = false; dispose?.(); };
  }, [addNativePaths]);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  useEffect(() => {
    return installZoomLock();
  }, []);

  useEffect(() => {
    void invoke("set_close_to_tray", { enabled: closeToTray })
      .catch(() => undefined);
  }, [closeToTray]);

  useEffect(() => {
    let active = true;
    let dispose: (() => void) | undefined;
    const cleanups: Array<() => void> = [];
    let disposed = false;
    const cleanupPartial = () => {
      disposed = true;
      while (cleanups.length) cleanups.pop()?.();
    };
    void import("@tauri-apps/api/event").then(async ({ listen }) => {
      try {
        const register = async <T,>(event: string, handler: (event: { payload: T }) => void) => {
          const unlisten = await listen<T>(event, handler);
          if (disposed || !active) {
            unlisten();
            return;
          }
          cleanups.push(unlisten);
        };
        await register<unknown>("menu:navigate", (event) => {
          if (!active) return;
          const destination = normalizeNavigationPage(event.payload);
          if (destination) setPage(destination);
        });
        if (!active || disposed) return;
        await register<unknown>("batch-progress", (event) => {
          if (!active) return;
          const progress = normalizeBatchProgress(event.payload);
          if (!progress) return;
          if (operationKindRef.current === progress.operation && batchIdRef.current === progress.batchId) {
            setProgress(progress);
            const now = Date.now();
            const recovery = recoveryProgressRef.current;
            const shouldPersist = progress.cancelled
              || progress.completed === progress.total
              || recovery.batchId !== progress.batchId
              || progress.completed - recovery.completed >= 16
              || now - recovery.persistedAt >= 250;
            if (progress.operation === "clean" && shouldPersist) {
              updateActiveBatchProgress(progress.batchId, progress.completed);
              recoveryProgressRef.current = { batchId: progress.batchId, completed: progress.completed, persistedAt: now };
            }
          }
        });
        if (!active || disposed) return;
        await register<unknown>("close-blocked", (event) => {
          const detail = normalizeCloseBlocked(event.payload);
          if (active && detail) setMessage(detail);
        });
        if (active) {
          dispose = cleanupPartial;
          return;
        }
      } catch {
        // A partial registration is still ours to release; do not leave an
        // earlier listener behind when a later native subscription fails.
      }
      cleanupPartial();
    }).catch(() => cleanupPartial());
    return () => { active = false; cleanupPartial(); dispose?.(); };
  }, []);

  useEffect(() => {
    const runShortcut = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
      if (event.key.toLowerCase() === "k") {
        event.preventDefault();
        setCommandsOpen((open) => !open);
        return;
      }
      const destination = ({ "1": "clean", "2": "history", "3": "privacy", "4": "settings", "5": "about" } as const)[event.key];
      if (!destination) return;
      event.preventDefault();
      setPage(destination);
    };
    window.addEventListener("keydown", runShortcut);
    return () => window.removeEventListener("keydown", runShortcut);
  }, []);

  const scanned = entries.length > 0 && entries.every((entry) => entry.status === "scanned" || entry.status === "clean" || (entry.status === "error" && Boolean(entry.result)));
  const cleanableEntries = entries.filter((entry) => (
    entry.status === "scanned" || (entry.status === "error" && entry.result?.success === false)
  ) && actionableFindingCount(entry.report, preserveColorProfile, removeExtendedAttributes) > 0);

  async function scan() {
    if (!mountedRef.current || operationRef.current) return;
    const pendingEntries = entries.filter((entry) => entry.status === "ready" || (entry.status === "error" && !entry.result));
    if (!pendingEntries.length) return;
    const paths = pendingEntries.flatMap((entry) => entry.path ? [entry.path] : []);
    if (paths.length !== pendingEntries.length) { setMessage(text("浏览器模式无法取得完整路径，请在桌面应用中选择文件。", "Browser mode cannot access full paths. Choose files in the desktop app.")); return; }
    operationRef.current = true;
    operationKindRef.current = "scan";
    setActiveOperation("scan");
    const batchId = crypto.randomUUID();
    batchIdRef.current = batchId;
    setActiveBatchId(batchId);
    cancelRequestedRef.current = false;
    setCancelRequested(false);
    setBusy(true); setProgress(undefined); setMessage(undefined); updateEntries((current) => markEntryPaths(current, paths, "scanning"));
    try {
      const reports = normalizeScanReports(await invoke<unknown>("scan_files", { paths, batchId }));
      if (!reports) throw new Error(text("扫描返回了无效数据", "Scanning returned invalid data"));
      if (!mountedRef.current) return;
      const requested = new Set(paths.map(pathIdentity));
      const seenReports = new Set<string>();
      const relevant = reports.filter((report) => {
        const identity = pathIdentity(report.path);
        if (!requested.has(identity) || seenReports.has(identity)) return false;
        seenReports.add(identity);
        return true;
      });
      updateEntries((current) => applyScanReports(current, paths, relevant));
      const count = sumFindingCounts(relevant.flatMap((report) => report.findings));
      const missing = paths.length - relevant.length;
      setMessage(cancelRequestedRef.current
        ? text(`已取消扫描：${relevant.length} 个文件已返回结果，${missing} 个可重试。`, `Scan cancelled: ${relevant.length} file(s) returned results; ${missing} can be retried.`)
        : text(`扫描完成：${count} 项痕迹等待确认。${missing > 0 ? ` ${missing} 个文件未返回结果，可重试扫描。` : ""}`, `Scan complete: ${count} trace(s) await confirmation.${missing > 0 ? ` ${missing} file(s) returned no result and can be retried.` : ""}`));
    } catch (error) {
      if (mountedRef.current) {
        updateEntries((current) => markEntryPaths(current, paths, "ready"));
        const detail = boundedErrorMessage(error);
        setMessage(text(`扫描失败：${detail}`, `Scan failed: ${detail}`));
      }
    }
    finally {
      operationRef.current = false;
      operationKindRef.current = undefined;
      batchIdRef.current = undefined;
      cancelRequestedRef.current = false;
      if (mountedRef.current) {
        setActiveBatchId(undefined);
        setActiveOperation(undefined);
        setCancelRequested(false);
        setBusy(false);
        setProgress(undefined);
      }
    }
  }

  async function clean() {
    if (!mountedRef.current || operationRef.current) return;
    const paths = cleanableEntries.flatMap((entry) => entry.path ? [entry.path] : []);
    if (!paths.length) return;
    operationRef.current = true;
    operationKindRef.current = "clean";
    setActiveOperation("clean");
    const batchId = crypto.randomUUID();
    batchIdRef.current = batchId;
    setActiveBatchId(batchId);
    cancelRequestedRef.current = false;
    setCancelRequested(false);
    setBusy(true); setProgress(undefined); setMessage(undefined);
    writeActiveBatch({ batchId, total: paths.length, completed: 0, mode, startedAt: new Date().toISOString() });
    recoveryProgressRef.current = { batchId, completed: 0, persistedAt: Date.now() };
    try {
      const results = normalizeCleanResults(await invoke<unknown>("clean_files", { request: { paths, batchId, mode, preserveTimestamps, preserveOrientation, preserveColorProfile, removeExtendedAttributes } }));
      if (!results) throw new Error(text("清理返回了无效数据", "Cleanup returned invalid data"));
      if (!mountedRef.current) return;
      const requested = new Set(paths.map(pathIdentity));
      const seenResults = new Set<string>();
      const relevant = results.filter((result) => {
        const identity = pathIdentity(result.sourcePath);
        if (!requested.has(identity) || seenResults.has(identity)) return false;
        seenResults.add(identity);
        return true;
      });
      const byPath = new Map(relevant.map((result) => [pathIdentity(result.sourcePath), result]));
      updateEntries((current) => current.map((entry) => {
        const result = entry.path ? byPath.get(pathIdentity(entry.path)) : undefined;
        return result ? { ...entry, status: result.success ? "clean" : "error", result } : entry;
      }));
      const successes = relevant.filter((result) => result.success);
      const failures = relevant.length - successes.length;
      const missing = paths.length - relevant.length;
      setMessage(cancelRequestedRef.current && missing > 0
        ? text(`已取消清理：${successes.length} 个完成${missing > 0 ? `，${missing} 个未处理、可重试` : ""}。`, `Cleanup cancelled: ${successes.length} completed${missing > 0 ? `; ${missing} not processed and can be retried` : ""}.`)
        : text(`${successes.length} 个文件清理完成${failures ? `，${failures} 个失败` : ""}${missing > 0 ? `，${missing} 个未返回结果、可重试` : ""}。${successes[0]?.outputPath ? ` 输出：${successes[0].outputPath}` : ""}`, `${successes.length} file(s) cleaned${failures ? `; ${failures} failed` : ""}${missing > 0 ? `; ${missing} returned no result and can be retried` : ""}.${successes[0]?.outputPath ? ` Output: ${successes[0].outputPath}` : ""}`));
      if (relevant.length) addHistory({ id: crypto.randomUUID(), createdAt: new Date().toISOString(), mode, results: relevant });
    } catch (error) {
      if (mountedRef.current) {
        const detail = boundedErrorMessage(error);
        setMessage(text(`清理失败：${detail}`, `Cleanup failed: ${detail}`));
      }
    }
    finally {
      clearActiveBatch(batchId);
      recoveryProgressRef.current = { completed: 0, persistedAt: 0 };
      operationRef.current = false;
      operationKindRef.current = undefined;
      batchIdRef.current = undefined;
      cancelRequestedRef.current = false;
      if (mountedRef.current) {
        setActiveBatchId(undefined);
        setActiveOperation(undefined);
        setCancelRequested(false);
        setBusy(false);
        setProgress(undefined);
      }
    }
  }

  function cancelOperation() {
    const batchId = batchIdRef.current;
    if (!batchId || cancelRequestedRef.current) return;
    cancelRequestedRef.current = true;
    setCancelRequested(true);
    const command = operationKindRef.current === "scan" ? "cancel_scan_batch" : "cancel_clean_batch";
    void invoke<unknown>(command, { batchId }).then((accepted) => {
      if (accepted !== true && mountedRef.current && batchIdRef.current === batchId && operationKindRef.current === (command === "cancel_scan_batch" ? "scan" : "clean")) {
        cancelRequestedRef.current = false;
        setCancelRequested(false);
      }
    }).catch((error) => {
      if (!mountedRef.current || batchIdRef.current !== batchId || operationKindRef.current !== (command === "cancel_scan_batch" ? "scan" : "clean")) return;
      cancelRequestedRef.current = false;
      setCancelRequested(false);
      const detail = boundedErrorMessage(error);
      setMessage(text(`取消处理失败：${detail}`, `Could not cancel operation: ${detail}`));
    });
  }

  async function reveal(path: string) {
    if (!mountedRef.current) return;
    try {
      const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
      if (!mountedRef.current) return;
      await revealItemInDir(path);
    } catch (error) {
      if (!mountedRef.current) return;
      setMessage(boundedErrorMessage(error));
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
    { id: "go-about", group: go, label: text("关于", "About"), icon: <CircleHelp size={14} />, accelerator: `${modifier}5`, run: () => setPage("about") },
    { id: "pick-files", group: act, label: text("选择文件", "Choose files"), icon: <FilePlus2 size={14} />, run: () => void openPicker(false) },
    { id: "pick-folder", group: act, label: text("选择文件夹", "Choose folder"), icon: <FolderOpen size={14} />, run: () => void openPicker(true) },
    { id: "scan", group: act, label: text("扫描隐私痕迹", "Scan privacy traces"), icon: <ScanSearch size={14} />, disabled: busy || !entries.length || scanned, run: () => { setPage("clean"); void scan(); } },
    { id: "clean", group: act, label: text("确认并开始清理", "Confirm and clean"), icon: <ShieldCheck size={14} />, disabled: busy || !scanned || !cleanableEntries.length, run: () => { setPage("clean"); void clean(); } },
    { id: "clear", group: act, label: text("清空队列", "Clear queue"), icon: <Trash2 size={14} />, disabled: busy || !entries.length, run: () => setQueueClearPromptOpen(true) },
    { id: "theme-light", group: appearance, label: text("浅色", "Light"), icon: <Sun size={14} />, disabled: theme.mode === "light", run: () => theme.setMode("light") },
    { id: "theme-dark", group: appearance, label: text("深色", "Dark"), icon: <Moon size={14} />, disabled: theme.mode === "dark", run: () => theme.setMode("dark") },
    { id: "theme-system", group: appearance, label: text("跟随系统", "System"), icon: <MonitorCog size={14} />, disabled: theme.mode === "system", run: () => theme.setMode("system") },
  ];

  const titles: Record<Page, [string, string]> = {
    clean: [text("文件净化", "Clean files"), text("清除文件里的隐私痕迹，分享前更安心", "Remove private traces before sharing")],
    history: [text("处理记录", "History"), text("记录仅保存在此设备的应用数据中，不包含文件内容。", "History stays on this device and never stores file content.")],
    privacy: [text("隐私说明", "Privacy"), text("MetaClean 的处理边界清晰且可验证。", "MetaClean has clear, verifiable processing boundaries.")],
    settings: [text("设置", "Settings"), text("MetaClean · 纯本地文件隐私工具", "MetaClean · Local file privacy tool")],
    about: [text("关于", "About"), text("版本、运行环境、诊断信息与项目支持", "Version, runtime, diagnostics and project support")],
  };
  const [title, subtitle] = titles[page];

  return (
    <>
    <input
      ref={pickerInputRef}
      className="sr-only"
      type="file"
      multiple
      onChange={(event) => {
        addBrowserFiles(event.currentTarget.files);
        event.currentTarget.removeAttribute("webkitdirectory");
        event.currentTarget.value = "";
      }}
    />
    {/* Three fixed bands and one that takes what is left: the title bar and the
        status strip are chrome, and chrome that resizes with the content is the
        thing that makes a window feel like a page. */}
    <div className="app-shell grid h-screen grid-rows-[36px_minmax(0,1fr)_26px] overflow-hidden bg-canvas text-text">
    <TitleBar closeToTray={closeToTray} onOpenCommands={() => setCommandsOpen(true)} />
    <div className="grid min-h-0 grid-cols-[72px_minmax(0,1fr)]">
      <Sidebar page={page} onNavigate={setPage} />
      <main tabIndex={-1} className="flex min-h-0 flex-col overflow-hidden">
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
        <Suspense fallback={<div className="grid h-full place-items-center text-sm text-muted" role="status">{text("正在加载页面…", "Loading page…")}</div>}>
          {page === "clean" ? <div className="grid h-full max-w-[1180px] grid-cols-[minmax(0,1fr)_296px] gap-3">
            <div className="flex min-h-0 flex-col gap-3">
              {message ? (
                <div className="shrink-0 rounded-control border border-line bg-surface px-2.5 py-2 text-sm text-muted shadow-panel" role="status" aria-live="polite">
                  {message}
                </div>
              ) : null}
              <DropZone onAdd={addEntries} onAddNativePaths={addNativePaths} onError={reportPickerError} onOpenPicker={(directory) => void openPicker(directory)} dragActive={dragActive} compact={entries.length > 0} />
              <FileQueue entries={entries} preserveColorProfile={preserveColorProfile} removeExtendedAttributes={removeExtendedAttributes} busy={busy} onClear={clearQueue} onRemove={removeEntry} onReveal={(path) => void reveal(path)} onNotify={setMessage} />
            </div>
              <CleanOptions mode={mode} onModeChange={setMode} preserveTimestamps={preserveTimestamps} onPreserveTimestampsChange={setPreserveTimestamps} preserveOrientation={preserveOrientation} onPreserveOrientationChange={setPreserveOrientation} preserveColorProfile={preserveColorProfile} onPreserveColorProfileChange={setPreserveColorProfile} removeExtendedAttributes={removeExtendedAttributes} onRemoveExtendedAttributesChange={setRemoveExtendedAttributes} disabled={!entries.length} scanned={scanned} hasFindings={cleanableEntries.length > 0} busy={busy} operation={activeOperation} cancelable={Boolean(activeBatchId)} cancelRequested={cancelRequested} onCancel={cancelOperation} onAction={() => void (scanned ? clean() : scan())} />
          </div> : page === "history" ? <HistoryPage entries={history} onClear={clearHistory} /> : page === "privacy" ? <PrivacyPage /> : page === "about" ? <AboutPage /> : <SettingsPage mode={mode} onModeChange={setMode} preserveTimestamps={preserveTimestamps} onPreserveTimestampsChange={setPreserveTimestamps} preserveOrientation={preserveOrientation} onPreserveOrientationChange={setPreserveOrientation} preserveColorProfile={preserveColorProfile} onPreserveColorProfileChange={setPreserveColorProfile} removeExtendedAttributes={removeExtendedAttributes} onRemoveExtendedAttributesChange={setRemoveExtendedAttributes} closeToTray={closeToTray} onCloseToTrayChange={setCloseToTray} />}
        </Suspense>
        </div>
      </main>
    </div>
    <StatusBar busy={busy} operation={activeOperation} fileCount={entries.length} progress={progress} />
    </div>
    <UpdateDialog />
    {queueClearPromptOpen ? (
      <ConfirmDialog
        title={text("清空文件队列？", "Clear the file queue?")}
        description={text(`将移除当前队列中的 ${entries.length} 个文件，文件本身不会被修改。`, `This removes ${entries.length} file(s) from the queue; the files themselves will not be changed.`)}
        confirmLabel={text("清空队列", "Clear queue")}
        onCancel={() => setQueueClearPromptOpen(false)}
        onConfirm={() => { setQueueClearPromptOpen(false); clearQueue(); }}
      />
    ) : null}
    <TooltipHost />
    {commandsOpen ? <CommandPalette commands={commands} onClose={() => setCommandsOpen(false)} /> : null}
    </>
  );
}
