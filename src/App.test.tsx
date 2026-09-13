import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { I18nProvider } from "./lib/i18n";
import { UpdateProvider } from "./contexts/UpdateContext";
import { ThemeProvider } from "./contexts/ThemeContext";
import type { ScanReport } from "./types";
import { ACTIVE_BATCH_STORAGE_KEY } from "./lib/recovery";

const dragDropEventMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: () => ({ onDragDropEvent: dragDropEventMock }) }));
const invokeMock = vi.hoisted(() => vi.fn());
const revealMock = vi.hoisted(() => vi.fn());
const listenMock = vi.hoisted(() => vi.fn());
const pickerOpenMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: () => Promise.resolve("0.4.1") }));
vi.mock("@tauri-apps/plugin-opener", () => ({ revealItemInDir: revealMock }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: pickerOpenMock }));

describe("App", () => {
  const renderApp = () => render(<ThemeProvider initialMode="light"><I18nProvider><UpdateProvider><App /></UpdateProvider></I18nProvider></ThemeProvider>);
  beforeEach(() => {
    localStorage.clear();
    invokeMock.mockClear();
    localStorage.setItem("metaclean.locale", "zh");
    revealMock.mockReset();
    revealMock.mockResolvedValue(undefined);
    pickerOpenMock.mockReset();
    dragDropEventMock.mockReset();
    dragDropEventMock.mockResolvedValue(() => undefined);
    listenMock.mockReset();
    listenMock.mockResolvedValue(() => undefined);
    invokeMock.mockImplementation((command?: string) => command === "get_launch_paths" || command === undefined ? Promise.resolve([]) : command === "set_close_to_tray" ? Promise.resolve(undefined) : command === "get_about_info" ? Promise.resolve({ version: "0.7.0", platform: "windows", arch: "x86_64" }) : command === "expand_paths" ? Promise.resolve({ files: [], skippedCount: 0, issues: [], limitReached: false }) : Promise.reject(new Error(`unexpected ${command}`)));
  });
  it("starts with scanning disabled", () => {
    renderApp();
    expect(screen.getByRole("button", { name: "扫描隐私痕迹" })).toBeDisabled();
  });

  it("fails closed when the native launch-path response is malformed", async () => {
    invokeMock.mockImplementation((command?: string) => command === "get_launch_paths"
      ? Promise.resolve({ paths: ["C:\\work\\notes.txt"] })
      : command === "set_close_to_tray" ? Promise.resolve(undefined) : Promise.resolve([]));
    renderApp();
    expect(await screen.findByText("启动路径返回了无效数据。")).toBeInTheDocument();
    expect(screen.queryByText("notes.txt")).not.toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalledWith("expand_paths", expect.anything());
  });

  it("fails closed when native directory expansion returns malformed data", async () => {
    invokeMock.mockImplementation((command?: string) => command === "get_launch_paths"
      ? Promise.resolve(["C:\\work\\notes.txt"])
      : command === "expand_paths" ? Promise.resolve({ files: "not-an-array", skippedCount: 0, issues: [], limitReached: false })
        : command === "set_close_to_tray" ? Promise.resolve(undefined) : Promise.resolve([]));
    renderApp();
    expect(await screen.findByText("无法展开所选路径：路径展开返回了无效数据")).toBeInTheDocument();
    expect(screen.queryByText("notes.txt")).not.toBeInTheDocument();
  });

  it("fails closed when native scan results are malformed", async () => {
    invokeMock.mockImplementation((command?: string) => command === "get_launch_paths"
      ? Promise.resolve(["C:\\work\\notes.txt"])
      : command === "expand_paths" ? Promise.resolve({ files: ["C:\\work\\notes.txt"], skippedCount: 0, issues: [], limitReached: false })
        : command === "scan_files" ? Promise.resolve([{ path: "C:\\work\\notes.txt", name: "notes.txt", format: "Text", size: -1, supported: true, findings: [] }])
          : command === "set_close_to_tray" ? Promise.resolve(undefined) : Promise.resolve([]));
    renderApp();
    await screen.findByText("notes.txt");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    expect(await screen.findByText("扫描失败：扫描返回了无效数据")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "扫描隐私痕迹" })).toBeEnabled();
  });

  it("fails closed when native cleanup results are malformed", async () => {
    invokeMock.mockImplementation((command?: string) => command === "get_launch_paths"
      ? Promise.resolve(["C:\\work\\notes.txt"])
      : command === "expand_paths" ? Promise.resolve({ files: ["C:\\work\\notes.txt"], skippedCount: 0, issues: [], limitReached: false })
        : command === "scan_files" ? Promise.resolve([{ path: "C:\\work\\notes.txt", name: "notes.txt", format: "Text", size: 4, supported: true, findings: [{ category: "unicode", label: "Invisible Unicode", count: 1, severity: "privacy" }] }])
          : command === "clean_files" ? Promise.resolve([{ sourcePath: "C:\\work\\notes.txt", removed: [], success: "yes" }])
            : command === "set_close_to_tray" ? Promise.resolve(undefined) : Promise.resolve([]));
    renderApp();
    await screen.findByText("notes.txt");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await screen.findByText("发现 1 项痕迹");
    fireEvent.click(screen.getByRole("button", { name: "确认并开始清理" }));
    expect(await screen.findByText("清理失败：清理返回了无效数据")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "确认并开始清理" })).toBeEnabled();
  });

  it("cleans up a drag-drop listener that finishes registering after unmount", async () => {
    let resolveRegistration: ((unlisten: () => void) => void) | undefined;
    const unlisten = vi.fn();
    dragDropEventMock.mockReturnValueOnce(new Promise<() => void>((resolve) => { resolveRegistration = resolve; }));
    const { unmount } = renderApp();
    await waitFor(() => expect(dragDropEventMock).toHaveBeenCalledOnce());
    unmount();
    resolveRegistration?.(unlisten);
    await waitFor(() => expect(unlisten).toHaveBeenCalledOnce());
  });

  it("clears native drag hover and reports an invalid drop payload", async () => {
    let handle: ((event: { payload: unknown }) => void) | undefined;
    dragDropEventMock.mockImplementation((listener: (event: { payload: unknown }) => void) => {
      handle = listener;
      return Promise.resolve(() => undefined);
    });
    renderApp();
    await waitFor(() => expect(dragDropEventMock).toHaveBeenCalledOnce());
    const zone = screen.getByText("拖入要净化的文件").closest("section")!;
    handle?.({ payload: { type: "enter", paths: [] } });
    await waitFor(() => expect(zone.className).toContain("border-brand"));
    handle?.({ payload: { type: "drop", paths: Array.from({ length: 10_001 }, (_, index) => `C:\\drop-${index}.txt`) } });
    expect(await screen.findByRole("status")).toHaveTextContent("拖放数据无效或超过安全上限");
    expect(zone.className).not.toContain("border-brand");
  });

  it("clears native drag hover even when the event payload is revoked", async () => {
    let handle: ((event: { payload: unknown }) => void) | undefined;
    dragDropEventMock.mockImplementation((listener: (event: { payload: unknown }) => void) => {
      handle = listener;
      return Promise.resolve(() => undefined);
    });
    renderApp();
    await waitFor(() => expect(dragDropEventMock).toHaveBeenCalledOnce());
    const zone = screen.getByText("拖入要净化的文件").closest("section")!;
    handle?.({ payload: { type: "enter", paths: [] } });
    await waitFor(() => expect(zone.className).toContain("border-brand"));
    const revoked = Proxy.revocable({ type: "drop", paths: [] }, {});
    revoked.revoke();
    handle?.({ payload: revoked.proxy });
    await waitFor(() => expect(zone.className).not.toContain("border-brand"));
  });

  it("reports a native drop whose paths are all filtered out", async () => {
    let handle: ((event: { payload: unknown }) => void) | undefined;
    dragDropEventMock.mockImplementation((listener: (event: { payload: unknown }) => void) => {
      handle = listener;
      return Promise.resolve(() => undefined);
    });
    renderApp();
    await waitFor(() => expect(dragDropEventMock).toHaveBeenCalledOnce());
    handle?.({ payload: { type: "drop", paths: ["", 4, "界".repeat(20_000)] } });
    expect(await screen.findByRole("status")).toHaveTextContent("拖放数据无效或未包含可处理文件");
    expect(invokeMock).not.toHaveBeenCalledWith("expand_paths", expect.anything());
  });

  it("releases event listeners when a later native subscription fails", async () => {
    const unlistenMenu = vi.fn();
    listenMock
      .mockResolvedValueOnce(unlistenMenu)
      .mockRejectedValueOnce(new Error("event bridge unavailable"));
    renderApp();
    await waitFor(() => expect(listenMock).toHaveBeenCalledTimes(2));
    expect(unlistenMenu).toHaveBeenCalledOnce();
  });

  it("releases established listeners while a later subscription is still pending", async () => {
    const unlistenMenu = vi.fn();
    const unlistenProgress = vi.fn();
    let resolveProgress: ((unlisten: () => void) => void) | undefined;
    listenMock
      .mockResolvedValueOnce(unlistenMenu)
      .mockReturnValueOnce(new Promise((resolve) => { resolveProgress = resolve; }));
    const { unmount } = renderApp();
    await waitFor(() => expect(listenMock).toHaveBeenCalledTimes(2));
    unmount();
    expect(unlistenMenu).toHaveBeenCalledOnce();
    resolveProgress?.(unlistenProgress);
    await waitFor(() => expect(unlistenProgress).toHaveBeenCalledOnce());
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

  it("keeps picker commands usable after leaving the clean page", async () => {
    pickerOpenMock.mockRejectedValue(new Error("dialog unavailable"));
    const { container } = renderApp();
    fireEvent.click(screen.getByRole("button", { name: "设置" }));
    const rootInput = container.querySelector("input[type=file]") as HTMLInputElement;
    const click = vi.spyOn(rootInput, "click");
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    const palette = await screen.findByRole("dialog", { name: "命令" });
    fireEvent.click(within(palette).getByRole("option", { name: "选择文件" }));
    await waitFor(() => expect(pickerOpenMock).toHaveBeenCalledWith({ multiple: true, directory: false }));
    await waitFor(() => expect(click).toHaveBeenCalledOnce());
  });

  it("surfaces an invalid browser picker batch instead of silently dropping it", async () => {
    pickerOpenMock.mockRejectedValue(new Error("dialog unavailable"));
    const { container } = renderApp();
    const input = container.querySelector("input[type=file]") as HTMLInputElement;
    fireEvent.change(input, { target: { files: { length: 10_001 } } });
    expect(await screen.findByRole("status")).toHaveTextContent("浏览器选择无效或超过 10,000 个文件");
  });

  it("fails closed when the root browser picker list is revoked", async () => {
    const files = Object.defineProperty({}, "length", { get: () => { throw new Error("revoked"); } });
    const { container } = renderApp();
    fireEvent.change(container.querySelector("input[type=file]") as HTMLInputElement, { target: { files } });
    expect(await screen.findByRole("status")).toHaveTextContent("选择器返回了无效数据：浏览器选择无效");
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

  it("does not let a late scan result update a remounted app", async () => {
    let finishScan: ((reports: ScanReport[]) => void) | undefined;
    const pendingScan = new Promise<ScanReport[]>((resolve) => { finishScan = resolve; });
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_launch_paths") return Promise.resolve(["C:\\work\\notes.md"]);
      if (command === "expand_paths") return Promise.resolve({ files: ["C:\\work\\notes.md"], skippedCount: 0, issues: [], limitReached: false });
      if (command === "scan_files") return pendingScan;
      if (command === "set_close_to_tray") return Promise.resolve(undefined);
      return Promise.reject(new Error(`unexpected ${command}`));
    });
    const first = renderApp();
    await screen.findByText("notes.md");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("scan_files", expect.anything()));
    first.unmount();
    finishScan?.([]);
    renderApp();
    expect(screen.queryByText("notes.md")).not.toBeInTheDocument();
  });

  it("does not let a late cleanup result update a remounted app", async () => {
    let finishClean: ((results: never[]) => void) | undefined;
    const pendingClean = new Promise<never[]>((resolve) => { finishClean = resolve; });
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_launch_paths") return Promise.resolve(["C:\\work\\notes.txt"]);
      if (command === "expand_paths") return Promise.resolve({ files: ["C:\\work\\notes.txt"], skippedCount: 0, issues: [], limitReached: false });
      if (command === "scan_files") return Promise.resolve([{ path: "C:\\work\\notes.txt", name: "notes.txt", format: "Text", size: 4, supported: true, findings: [{ category: "unicode", label: "Invisible Unicode", count: 1, severity: "privacy" }] }]);
      if (command === "clean_files") return pendingClean;
      if (command === "set_close_to_tray") return Promise.resolve(undefined);
      return Promise.reject(new Error(`unexpected ${command}`));
    });
    const first = renderApp();
    await screen.findByText("notes.txt");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await screen.findByText("发现 1 项痕迹");
    fireEvent.click(screen.getByRole("button", { name: "确认并开始清理" }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("clean_files", expect.anything()));
    first.unmount();
    finishClean?.([]);
    renderApp();
    expect(screen.queryByText("notes.txt")).not.toBeInTheDocument();
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

  it("cancels an active scan at a file boundary and leaves missing files retryable", async () => {
    let finishScan: ((reports: ScanReport[]) => void) | undefined;
    const pendingScan = new Promise<ScanReport[]>((resolve) => { finishScan = resolve; });
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_launch_paths") return Promise.resolve(["C:\\work\\notes.txt"]);
      if (command === "expand_paths") return Promise.resolve({ files: ["C:\\work\\notes.txt"], skippedCount: 0, issues: [], limitReached: false });
      if (command === "scan_files") return pendingScan;
      if (command === "cancel_scan_batch") return Promise.resolve(true);
      if (command === "set_close_to_tray") return Promise.resolve(undefined);
      return Promise.reject(new Error(`unexpected ${command}`));
    });
    renderApp();
    await screen.findByText("notes.txt");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("scan_files", { paths: ["C:\\work\\notes.txt"], batchId: expect.any(String) }));
    fireEvent.click(screen.getByRole("button", { name: "取消扫描" }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("cancel_scan_batch", { batchId: expect.any(String) }));
    finishScan?.([]);
    await screen.findByText(/已取消扫描/);
    expect(screen.getByRole("button", { name: "扫描隐私痕迹" })).toBeEnabled();
  });

  it("ignores a stale cancellation response from an older batch", async () => {
    let finishFirstScan: ((reports: ScanReport[]) => void) | undefined;
    let finishSecondScan: ((reports: ScanReport[]) => void) | undefined;
    let rejectFirstCancel: ((accepted: boolean) => void) | undefined;
    let finishSecondCancel: ((accepted: boolean) => void) | undefined;
    const firstScan = new Promise<ScanReport[]>((resolve) => { finishFirstScan = resolve; });
    const secondScan = new Promise<ScanReport[]>((resolve) => { finishSecondScan = resolve; });
    const firstCancel = new Promise<boolean>((resolve) => { rejectFirstCancel = resolve; });
    const secondCancel = new Promise<boolean>((resolve) => { finishSecondCancel = resolve; });
    let scanCount = 0;
    let cancelCount = 0;
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_launch_paths") return Promise.resolve(["C:\\work\\notes.txt"]);
      if (command === "expand_paths") return Promise.resolve({ files: ["C:\\work\\notes.txt"], skippedCount: 0, issues: [], limitReached: false });
      if (command === "scan_files") return scanCount++ === 0 ? firstScan : secondScan;
      if (command === "cancel_scan_batch") return cancelCount++ === 0 ? firstCancel : secondCancel;
      if (command === "set_close_to_tray") return Promise.resolve(undefined);
      return Promise.reject(new Error(`unexpected ${command}`));
    });
    renderApp();
    await screen.findByText("notes.txt");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("scan_files", expect.anything()));
    fireEvent.click(screen.getByRole("button", { name: "取消扫描" }));
    finishFirstScan?.([]);
    await screen.findByText(/已取消扫描/);
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await waitFor(() => expect(invokeMock.mock.calls.filter(([name]) => name === "scan_files")).toHaveLength(2));
    fireEvent.click(screen.getByRole("button", { name: "取消扫描" }));
    rejectFirstCancel?.(false);
    await waitFor(() => expect(screen.getByRole("button", { name: "正在取消…" })).toBeDisabled());
    finishSecondCancel?.(true);
    finishSecondScan?.([]);
    await screen.findByText(/已取消扫描/);
  });

  it("requires confirmation before clearing the queue and preserves it on cancel", () => {
    renderApp();
    const zone = screen.getByText("拖入要净化的文件").closest("section");
    fireEvent.drop(zone!, { dataTransfer: { files: [new File(["hello"], "notes.md", { type: "text/markdown" })] } });
    fireEvent.click(screen.getByRole("button", { name: "清空" }));
    const dialog = screen.getByRole("dialog", { name: "清空文件队列？" });
    expect(dialog).toBeInTheDocument();
    fireEvent.click(within(dialog).getAllByRole("button", { name: "取消" })[1]);
    expect(screen.getByText("notes.md")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "清空" }));
    fireEvent.click(screen.getByRole("button", { name: /^清空队列$/ }));
    expect(screen.queryByText("notes.md")).not.toBeInTheDocument();
    expect(screen.getByText("添加文件后，将在这里展示扫描状态")).toBeInTheDocument();
  });

  it("requires confirmation before deleting persisted processing history", async () => {
    localStorage.setItem("metaclean.history", JSON.stringify([{
      id: "history-entry",
      createdAt: "2026-09-13T00:00:00.000Z",
      mode: "copy",
      results: [{ sourcePath: "C:\\work\\notes.txt", outputPath: "C:\\work\\notes.cleaned.txt", removed: [], success: true }],
    }]));
    renderApp();
    fireEvent.click(screen.getByRole("button", { name: "处理记录" }));
    expect(await screen.findByText("notes.txt")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "清空记录" }));
    expect(screen.getByRole("dialog", { name: "清空处理记录？" })).toBeInTheDocument();
    fireEvent.click(within(screen.getByRole("dialog", { name: "清空处理记录？" })).getByRole("button", { name: "清空记录" }));
    await waitFor(() => expect(screen.getByText("还没有处理记录")).toBeInTheDocument());
    expect(localStorage.getItem("metaclean.history")).toBe("[]");
  });

  it("shows count-only scan progress and clears it after the scan", async () => {
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
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("scan_files", { paths: ["C:\\work\\notes.txt"], batchId: expect.any(String) }));
    const scanCall = invokeMock.mock.calls.find(([command]) => command === "scan_files");
    const batchId = scanCall?.[1].batchId as string;
    const progressListener = listenMock.mock.calls.find(([name]) => name === "batch-progress")?.[1] as ((event: { payload: { operation: "scan"; batchId: string; completed: number; total: number; failed: number; cancelled: boolean } }) => void) | undefined;
    progressListener?.({ payload: { operation: "scan", batchId, completed: 1, total: 1, failed: 0, cancelled: false } });
    await waitFor(() => expect(screen.getByRole("contentinfo")).toHaveTextContent("正在扫描 1/1"));
    finishScan?.([]);
    await screen.findByText(/扫描完成/);
    expect(screen.getByRole("contentinfo")).toHaveTextContent("就绪");
    expect(screen.getByRole("contentinfo")).not.toHaveTextContent("正在扫描 1/1");
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
      if (command === "scan_files") return Promise.resolve([report, { ...report, error: "不应覆盖首个结果" }, { ...report, path: "C:\\foreign.txt" }]);
      if (command === "set_close_to_tray") return Promise.resolve(undefined);
      return Promise.reject(new Error(`unexpected ${command}`));
    });
    renderApp();
    await screen.findByText("second.txt");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("1 个文件未返回结果"));
    expect(screen.queryByText("不应覆盖首个结果")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "扫描隐私痕迹" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await waitFor(() => expect(invokeMock.mock.calls.filter(([name]) => name === "scan_files")).toHaveLength(2));
    const retryCall = invokeMock.mock.calls.filter(([name]) => name === "scan_files")[1];
    expect(retryCall[1].paths).toEqual(["C:\\work\\second.txt"]);
  });

  it("keeps a file-level scan error retryable instead of treating it as complete", async () => {
    const failed = { path: "C:\\work\\broken.pdf", name: "broken.pdf", format: "PDF", size: 4, supported: false, findings: [], error: "文件格式无效" };
    const recovered = { ...failed, supported: true, error: undefined, findings: [] };
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_launch_paths") return Promise.resolve(["C:\\work\\broken.pdf"]);
      if (command === "expand_paths") return Promise.resolve({ files: ["C:\\work\\broken.pdf"], skippedCount: 0, issues: [], limitReached: false });
      if (command === "scan_files") return Promise.resolve(invokeMock.mock.calls.filter(([name]) => name === "scan_files").length === 1 ? [failed] : [recovered]);
      if (command === "set_close_to_tray") return Promise.resolve(undefined);
      return Promise.reject(new Error(`unexpected ${command}`));
    });
    renderApp();
    await screen.findByText("broken.pdf");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await screen.findByText("文件格式无效");
    const retry = screen.getByRole("button", { name: "扫描隐私痕迹" });
    expect(retry).toBeEnabled();
    fireEvent.click(retry);
    await waitFor(() => expect(invokeMock.mock.calls.filter(([name]) => name === "scan_files")).toHaveLength(2));
    await screen.findByText("未发现隐私痕迹");
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
