import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { I18nProvider } from "./lib/i18n";
import { UpdateProvider } from "./contexts/UpdateContext";
import { ThemeProvider } from "./contexts/ThemeContext";
import type { ScanReport } from "./types";
import { ACTIVE_BATCH_STORAGE_KEY } from "./lib/recovery";

vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: () => ({ onDragDropEvent: () => Promise.resolve(() => undefined) }) }));
const invokeMock = vi.hoisted(() => vi.fn());
const revealMock = vi.hoisted(() => vi.fn());
const listenMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: () => Promise.resolve("0.4.1") }));
vi.mock("@tauri-apps/plugin-opener", () => ({ revealItemInDir: revealMock }));

describe("App", () => {
  const renderApp = () => render(<ThemeProvider initialMode="light"><I18nProvider><UpdateProvider><App /></UpdateProvider></I18nProvider></ThemeProvider>);
  beforeEach(() => {
    localStorage.clear();
    invokeMock.mockClear();
    localStorage.setItem("metaclean.locale", "zh");
    revealMock.mockReset();
    revealMock.mockResolvedValue(undefined);
    listenMock.mockReset();
    listenMock.mockResolvedValue(() => undefined);
    invokeMock.mockImplementation((command?: string) => command === "get_launch_paths" || command === undefined ? Promise.resolve([]) : command === "set_close_to_tray" ? Promise.resolve(undefined) : command === "get_about_info" ? Promise.resolve({ version: "0.7.0", platform: "windows", arch: "x86_64" }) : command === "expand_paths" ? Promise.resolve({ files: [], skippedCount: 0, issues: [], limitReached: false }) : Promise.reject(new Error(`unexpected ${command}`)));
  });
  it("starts with scanning disabled", () => {
    renderApp();
    expect(screen.getByRole("button", { name: "扫描隐私痕迹" })).toBeDisabled();
  });

  it("exits on close by default and persists the optional tray behavior", async () => {
    renderApp();
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("set_close_to_tray", { enabled: false }));
    fireEvent.click(screen.getByRole("button", { name: "设置" }));
    fireEvent.click(await screen.findByRole("button", { name: "系统与更新" }));
    fireEvent.click(screen.getByRole("checkbox", { name: "关闭按钮退出应用" }));
    expect(localStorage.getItem("metaclean.closeToTray")).toBe("true");
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("set_close_to_tray", { enabled: true }));
  });

  it("shows the installed version in the sidebar without waiting for an update check", async () => {
    renderApp();
    expect(await screen.findByText("MetaClean v0.4.1")).toBeInTheDocument();
  });

  it("adds a dropped file and enables scanning", () => {
    renderApp();
    const zone = screen.getByText("拖入要净化的文件").closest("section");
    fireEvent.drop(zone!, { dataTransfer: { files: [new File(["hello"], "notes.md", { type: "text/markdown" })] } });
    expect(screen.getByText("notes.md")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "扫描隐私痕迹" })).toBeEnabled();
  });

  it("switches the complete navigation to English", () => {
    renderApp();
    fireEvent.click(screen.getByRole("button", { name: "设置" }));
    fireEvent.change(screen.getByRole("combobox", { name: "界面语言" }), { target: { value: "en" } });
    expect(screen.getByRole("button", { name: "Clean files" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Cleaning" }));
    expect(screen.getByText("Default output mode")).toBeInTheDocument();
    expect(localStorage.getItem("metaclean.locale")).toBe("en");
  });

  it("navigates every primary page with desktop accelerators", async () => {
    renderApp();
    fireEvent.keyDown(window, { key: "4", ctrlKey: true });
    expect(await screen.findByRole("button", { name: "外观与语言" })).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "2", metaKey: true });
    expect(await screen.findByText("还没有处理记录")).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "3", ctrlKey: true });
    expect(await screen.findByText("文件纯本地处理")).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "1", ctrlKey: true });
    expect(screen.getByRole("button", { name: "扫描隐私痕迹" })).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "5", ctrlKey: true });
    expect(await screen.findByText("诊断与支持")).toBeInTheDocument();
  });

  it("switches all primary navigation labels to Japanese", () => {
    renderApp();
    fireEvent.click(screen.getByRole("button", { name: "设置" }));
    fireEvent.change(screen.getByRole("combobox", { name: "界面语言" }), { target: { value: "ja" } });
    expect(screen.getByRole("button", { name: "ファイルをクリーン" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Cleaning" }));
    expect(screen.getByText("既定の出力モード")).toBeInTheDocument();
    expect(document.documentElement.lang).toBe("ja");
  });

  it("switches the document direction for Arabic", () => {
    renderApp();
    fireEvent.click(screen.getByRole("button", { name: "设置" }));
    fireEvent.change(screen.getByRole("combobox", { name: "界面语言" }), { target: { value: "ar" } });
    expect(screen.getByRole("button", { name: "تنظيف الملفات" })).toBeInTheDocument();
    expect(document.documentElement.lang).toBe("ar");
    expect(document.documentElement.dir).toBe("rtl");
  });

  it("scans native launch files, cleans findings, and stores history", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_launch_paths") return Promise.resolve(["C:\\work\\notes.txt"]);
      if (command === "expand_paths") return Promise.resolve({ files: ["C:\\work\\notes.txt"], skippedCount: 0, issues: [], limitReached: false });
      if (command === "scan_files") return Promise.resolve([{ path: "C:\\work\\notes.txt", name: "notes.txt", format: "Text", size: 4, supported: true, findings: [{ category: "unicode", label: "Invisible Unicode", count: 1, severity: "privacy" }] }]);
      if (command === "clean_files") return Promise.resolve([{ sourcePath: "C:\\work\\notes.txt", outputPath: "C:\\work\\notes.cleaned.txt", sourceSize: 4, outputSize: 3, removed: [], success: true }]);
      if (command === undefined) return Promise.resolve([]);
      return Promise.reject(new Error(`unexpected ${command}`));
    });
    renderApp();
    await screen.findByText("notes.txt");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await screen.findByText("发现 1 项痕迹");
    fireEvent.click(screen.getByRole("button", { name: "确认并开始清理" }));
    await screen.findByText(/1 个文件清理完成/);
    expect(invokeMock).toHaveBeenCalledWith("clean_files", { request: expect.objectContaining({ preserveColorProfile: true, removeExtendedAttributes: false }) });
    expect(JSON.parse(localStorage.getItem("metaclean.history") ?? "[]")).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: "C:\\work\\notes.cleaned.txt" }));
    await waitFor(() => expect(revealMock).toHaveBeenCalledWith("C:\\work\\notes.cleaned.txt"));
    revealMock.mockRejectedValueOnce(new Error("无法打开文件夹"));
    fireEvent.click(screen.getByRole("button", { name: "C:\\work\\notes.cleaned.txt" }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("无法打开文件夹"));
  });

  it("starts only one native scan when the action is triggered twice before React rerenders", async () => {
    let finishScan: ((reports: ScanReport[]) => void) | undefined;
    const pendingScan = new Promise<ScanReport[]>((resolve) => { finishScan = resolve; });
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_launch_paths") return Promise.resolve(["C:\\work\\notes.txt"]);
      if (command === "expand_paths") return Promise.resolve({ files: ["C:\\work\\notes.txt"], skippedCount: 0, issues: [], limitReached: false });
      if (command === "scan_files") return pendingScan;
      if (command === "set_close_to_tray") return Promise.resolve(undefined);
      return Promise.reject(new Error(`unexpected ${command}`));
    });
    renderApp();
    await screen.findByText("notes.txt");
    const button = screen.getByRole("button", { name: "扫描隐私痕迹" });
    fireEvent.click(button);
    fireEvent.click(button);
    await waitFor(() => expect(invokeMock.mock.calls.filter(([command]) => command === "scan_files")).toHaveLength(1));
    finishScan?.([]);
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("扫描完成"));
  });

  it("keeps files retryable when the native cleaner omits a result", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_launch_paths") return Promise.resolve(["C:\\work\\notes.txt"]);
      if (command === "expand_paths") return Promise.resolve({ files: ["C:\\work\\notes.txt"], skippedCount: 0, issues: [], limitReached: false });
      if (command === "scan_files") return Promise.resolve([{ path: "C:\\work\\notes.txt", name: "notes.txt", format: "Text", size: 4, supported: true, findings: [{ category: "unicode", label: "Invisible Unicode", count: 1, severity: "privacy" }] }]);
      if (command === "clean_files") return Promise.resolve([{ sourcePath: "C:\\foreign.txt", outputPath: "C:\\foreign.cleaned.txt", sourceSize: 4, outputSize: 3, removed: [], success: true }]);
      if (command === "set_close_to_tray") return Promise.resolve(undefined);
      return Promise.reject(new Error(`unexpected ${command}`));
    });
    renderApp();
    await screen.findByText("notes.txt");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await screen.findByText("发现 1 项痕迹");
    fireEvent.click(screen.getByRole("button", { name: "确认并开始清理" }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("1 个未返回结果、可重试"));
    expect(screen.getByRole("button", { name: "确认并开始清理" })).toBeEnabled();
    expect(localStorage.getItem("metaclean.history")).toBeNull();
  });

  it("ignores late or stale batch progress events after cleanup finishes", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_launch_paths") return Promise.resolve(["C:\\work\\notes.txt"]);
      if (command === "expand_paths") return Promise.resolve({ files: ["C:\\work\\notes.txt"], skippedCount: 0, issues: [], limitReached: false });
      if (command === "scan_files") return Promise.resolve([{ path: "c:/WORK/NOTES.TXT", name: "NOTES.TXT", format: "Text", size: 4, supported: true, findings: [{ category: "unicode", label: "Invisible Unicode", count: 1, severity: "privacy" }] }]);
      if (command === "clean_files") return Promise.resolve([{ sourcePath: "\\\\?\\C:\\WORK\\NOTES.TXT", outputPath: "C:\\work\\notes.cleaned.txt", sourceSize: 4, outputSize: 3, removed: [], success: true }]);
      if (command === "set_close_to_tray") return Promise.resolve(undefined);
      return Promise.reject(new Error(`unexpected ${command}`));
    });
    renderApp();
    await screen.findByText("notes.txt");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await screen.findByText("发现 1 项痕迹");
    fireEvent.click(screen.getByRole("button", { name: "确认并开始清理" }));
    await screen.findByText(/1 个文件清理完成/);
    const cleanupCall = invokeMock.mock.calls.find(([command]) => command === "clean_files");
    expect(cleanupCall?.[1].request.batchId).toEqual(expect.any(String));
    const progressListener = listenMock.mock.calls.find(([name]) => name === "batch-progress")?.[1] as ((event: { payload: { operation: "clean"; batchId: string; completed: number; total: number; failed: number; cancelled: boolean } }) => void) | undefined;
    expect(progressListener).toBeDefined();
    progressListener?.({ payload: { operation: "clean", batchId: "stale", completed: 99, total: 100, failed: 0, cancelled: false } });
    expect(screen.queryByText("正在清理 99/100")).not.toBeInTheDocument();
    expect(screen.getByText("就绪")).toBeInTheDocument();
    const closeListener = listenMock.mock.calls.find(([name]) => name === "close-blocked")?.[1] as ((event: { payload: string }) => void) | undefined;
    closeListener?.({ payload: "任务正在进行，请等待完成；清理任务可以先取消。" });
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("任务正在进行"));
  });

  it("cancels an active cleanup batch without cancelling the scan", async () => {
    let finishClean: ((results: never[]) => void) | undefined;
    const pendingClean = new Promise<never[]>((resolve) => { finishClean = resolve; });
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_launch_paths") return Promise.resolve(["C:\\work\\notes.txt"]);
      if (command === "expand_paths") return Promise.resolve({ files: ["C:\\work\\notes.txt"], skippedCount: 0, issues: [], limitReached: false });
      if (command === "scan_files") return Promise.resolve([{ path: "C:\\work\\notes.txt", name: "notes.txt", format: "Text", size: 4, supported: true, findings: [{ category: "unicode", label: "Invisible Unicode", count: 1, severity: "privacy" }] }]);
      if (command === "clean_files") return pendingClean;
      if (command === "cancel_clean_batch") return Promise.resolve(true);
      if (command === "set_close_to_tray") return Promise.resolve(undefined);
      return Promise.reject(new Error(`unexpected ${command}`));
    });
    renderApp();
    await screen.findByText("notes.txt");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await screen.findByText("发现 1 项痕迹");
    fireEvent.click(screen.getByRole("button", { name: "确认并开始清理" }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("clean_files", expect.anything()));
    const marker = JSON.parse(localStorage.getItem(ACTIVE_BATCH_STORAGE_KEY) ?? "null") as { batchId?: string; total?: number };
    expect(marker).toMatchObject({ batchId: expect.any(String), total: 1 });
    expect(localStorage.getItem(ACTIVE_BATCH_STORAGE_KEY)).not.toContain("C:\\\\");
    fireEvent.click(screen.getByRole("button", { name: "取消处理" }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("cancel_clean_batch", { batchId: expect.any(String) }));
    finishClean?.([]);
    await screen.findByText(/已取消清理/);
    expect(localStorage.getItem(ACTIVE_BATCH_STORAGE_KEY)).toBeNull();
  });

  it("warns once when the previous cleanup may have been interrupted", async () => {
    localStorage.setItem(ACTIVE_BATCH_STORAGE_KEY, JSON.stringify({ batchId: "old-batch", total: 4, completed: 2, mode: "copy", startedAt: "2026-09-12T10:00:00.000Z" }));
    renderApp();
    expect(await screen.findByRole("status")).toHaveTextContent("上次清理可能在 2/4 个文件后被中断");
    await waitFor(() => expect(localStorage.getItem(ACTIVE_BATCH_STORAGE_KEY)).toBeNull());
  });

  it("does not let duplicate or foreign scan reports hide a missing path", async () => {
    const report = { path: "C:\\work\\first.txt", name: "first.txt", format: "Text", size: 4, supported: true, findings: [] };
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_launch_paths") return Promise.resolve(["C:\\work\\first.txt", "C:\\work\\second.txt"]);
      if (command === "expand_paths") return Promise.resolve({ files: ["C:\\work\\first.txt", "C:\\work\\second.txt"], skippedCount: 0, issues: [], limitReached: false });
      if (command === "scan_files") return Promise.resolve([report, report, { ...report, path: "C:\\foreign.txt" }]);
      if (command === "set_close_to_tray") return Promise.resolve(undefined);
      return Promise.reject(new Error(`unexpected ${command}`));
    });
    renderApp();
    await screen.findByText("second.txt");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("1 个文件未返回结果"));
    expect(screen.getByRole("button", { name: "扫描隐私痕迹" })).toBeEnabled();
  });

  it("keeps an explicitly failed cleanup retryable", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_launch_paths") return Promise.resolve(["C:\\work\\notes.txt"]);
      if (command === "expand_paths") return Promise.resolve({ files: ["C:\\work\\notes.txt"], skippedCount: 0, issues: [], limitReached: false });
      if (command === "scan_files") return Promise.resolve([{ path: "C:\\work\\notes.txt", name: "notes.txt", format: "Text", size: 4, supported: true, findings: [{ category: "unicode", label: "Invisible Unicode", count: 1, severity: "privacy" }] }]);
      if (command === "clean_files") return Promise.resolve([{ sourcePath: "C:\\work\\notes.txt", sourceSize: 4, removed: [], success: false, error: "文件暂时被占用" }]);
      if (command === "set_close_to_tray") return Promise.resolve(undefined);
      return Promise.reject(new Error(`unexpected ${command}`));
    });
    renderApp();
    await screen.findByText("notes.txt");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await screen.findByText("发现 1 项痕迹");
    fireEvent.click(screen.getByRole("button", { name: "确认并开始清理" }));
    await screen.findByText("文件暂时被占用");
    expect(screen.getByRole("button", { name: "确认并开始清理" })).toBeEnabled();
  });

  it("persists ICC preservation and treats profiles as actionable only when removal is selected", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_launch_paths") return Promise.resolve(["C:\\work\\photo.jpg"]);
      if (command === "expand_paths") return Promise.resolve({ files: ["C:\\work\\photo.jpg"], skippedCount: 0, issues: [], limitReached: false });
      if (command === "scan_files") return Promise.resolve([{ path: "C:\\work\\photo.jpg", name: "photo.jpg", format: "JPEG", size: 4, supported: true, findings: [{ category: "color_profile", label: "ICC 色彩配置文件", count: 1, severity: "informational" }] }]);
      if (command === "get_context_menu_status") return Promise.resolve({ available: false, enabled: false, detail: "仅 Windows" });
      return Promise.reject(new Error(`unexpected ${command}`));
    });
    renderApp();
    await screen.findByText("photo.jpg");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    expect(await screen.findByRole("button", { name: "没有需要清理的痕迹" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "设置" }));
    fireEvent.click(screen.getByRole("button", { name: "清理偏好" }));
    fireEvent.click(screen.getByRole("checkbox", { name: /图片 · ICC \/ sRGB/ }));
    expect(localStorage.getItem("metaclean.preserveColorProfile")).toBe("false");
    fireEvent.click(screen.getByRole("button", { name: "文件净化" }));
    expect(screen.getByRole("button", { name: "确认并开始清理" })).toBeEnabled();
  });

  it("keeps macOS provenance attributes by default and removes them only after explicit selection", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_launch_paths") return Promise.resolve(["/Users/test/report.pdf"]);
      if (command === "expand_paths") return Promise.resolve({ files: ["/Users/test/report.pdf"], skippedCount: 0, issues: [], limitReached: false });
      if (command === "scan_files") return Promise.resolve([{ path: "/Users/test/report.pdf", name: "report.pdf", format: "PDF", size: 4, supported: true, findings: [{ category: "macos_xattr", label: "macOS provenance attributes", count: 2, severity: "informational" }] }]);
      if (command === "get_context_menu_status") return Promise.resolve({ available: false, enabled: false, detail: "仅 Windows" });
      return Promise.reject(new Error(`unexpected ${command}`));
    });
    renderApp();
    await screen.findByText("report.pdf");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    expect(await screen.findByRole("button", { name: "没有需要清理的痕迹" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "设置" }));
    fireEvent.click(screen.getByRole("button", { name: "清理偏好" }));
    fireEvent.click(screen.getByRole("checkbox", { name: /macOS · xattr/ }));
    expect(localStorage.getItem("metaclean.removeExtendedAttributes")).toBe("true");
    fireEvent.click(screen.getByRole("button", { name: "文件净化" }));
    expect(screen.getByRole("button", { name: "确认并开始清理" })).toBeEnabled();
  });

  it("reports native scan failures without modifying files", async () => {
    invokeMock.mockImplementation((command?: string) => command === "get_launch_paths" ? Promise.resolve(["C:\\broken.pdf"]) : command === "expand_paths" ? Promise.resolve({ files: ["C:\\broken.pdf"], skippedCount: 0, issues: [], limitReached: false }) : command === undefined ? Promise.resolve([]) : Promise.reject(new Error("engine unavailable")));
    renderApp();
    await screen.findByText("broken.pdf");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("扫描失败"));
    const retry = screen.getByRole("button", { name: "扫描隐私痕迹" });
    expect(retry).toBeEnabled();
    fireEvent.click(retry);
    await waitFor(() => expect(invokeMock.mock.calls.filter(([command]) => command === "scan_files")).toHaveLength(2));
  });
});
