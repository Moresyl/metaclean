import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useState } from "react";
import CleanOptions from "./CleanOptions";
import DropZone from "./DropZone";
import FileQueue from "./FileQueue";
import HistoryPage from "./HistoryPage";
import PrivacyPage from "./PrivacyPage";
import SettingsPage from "./SettingsPage";
import StatusBar from "./StatusBar";
import UpdateDialog from "./UpdateDialog";
import AboutPage from "./AboutPage";
import ConfirmDialog from "./ConfirmDialog";
import { buildDiagnosticReport } from "../lib/about";
import { I18nProvider, useI18n } from "../lib/i18n";
import type { FileEntry, HistoryEntry } from "../types";
import { UpdateProvider, useUpdate } from "../contexts/UpdateContext";
import { ThemeProvider } from "../contexts/ThemeContext";

const openMock = vi.hoisted(() => vi.fn());
const saveMock = vi.hoisted(() => vi.fn());
const invokeMock = vi.hoisted(() => vi.fn());
const checkForUpdateMock = vi.hoisted(() => vi.fn());
const getInstalledVersionMock = vi.hoisted(() => vi.fn());
const getUpdateRuntimeMock = vi.hoisted(() => vi.fn());
const installAvailableUpdateMock = vi.hoisted(() => vi.fn());
const openUrlMock = vi.hoisted(() => vi.fn());
const revealItemMock = vi.hoisted(() => vi.fn());
const clipboardMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: openMock, save: saveMock }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: vi.fn().mockResolvedValue("0.6.1") }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: openUrlMock, revealItemInDir: revealItemMock }));
vi.mock("../lib/update", () => ({
  RELEASES_PAGE_URL: "https://github.com/Moresyl/metaclean/releases/latest",
  checkForUpdate: checkForUpdateMock,
  getInstalledVersion: getInstalledVersionMock,
  getUpdateRuntime: getUpdateRuntimeMock,
  installAvailableUpdate: installAvailableUpdateMock,
}));

const wrap = (node: React.ReactNode) => render(<ThemeProvider initialMode="light"><I18nProvider><UpdateProvider>{node}</UpdateProvider></I18nProvider></ThemeProvider>);

function UpdateDialogHarness() {
  const update = useUpdate();
  return <><button type="button" onClick={() => void update.checkUpdate()}>trigger update</button><UpdateDialog /></>;
}

beforeEach(() => {
  openMock.mockReset();
  saveMock.mockReset();
  invokeMock.mockReset();
  checkForUpdateMock.mockReset();
  checkForUpdateMock.mockResolvedValue({ status: "current", currentVersion: "0.1.0" });
  getInstalledVersionMock.mockReset();
  getInstalledVersionMock.mockResolvedValue("0.4.1");
  getUpdateRuntimeMock.mockReset();
  getUpdateRuntimeMock.mockResolvedValue({ selfUpdateSupported: false, portable: true });
  installAvailableUpdateMock.mockReset();
  openUrlMock.mockReset();
  revealItemMock.mockReset();
  revealItemMock.mockResolvedValue(undefined);
  clipboardMock.mockReset();
  clipboardMock.mockResolvedValue(undefined);
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: clipboardMock } });
});

describe("desktop components", () => {
  it("keeps destructive confirmation modal keyboard reachable and dismissible", () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();
    wrap(<ConfirmDialog title="确认操作" description="不会修改文件" confirmLabel="继续" onConfirm={onConfirm} onCancel={onCancel} />);
    expect(screen.getByRole("dialog", { name: "确认操作" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "继续" })).toHaveFocus();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onCancel).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: "继续" }));
    expect(onConfirm).toHaveBeenCalledOnce();
  });

  it("returns focus to the main content when the trigger is removed after confirmation", () => {
    function Harness() {
      const [open, setOpen] = useState(true);
      return <><main tabIndex={-1} /><button type="button" disabled={!open}>触发</button>{open ? <ConfirmDialog title="确认" description="说明" confirmLabel="删除" onConfirm={() => setOpen(false)} onCancel={() => setOpen(false)} /> : null}</>;
    }
    wrap(<Harness />);
    fireEvent.click(screen.getByRole("button", { name: "删除" }));
    expect(screen.getByRole("main")).toHaveFocus();
  });

  it("keeps local processing state and version visible in the status bar", async () => {
    wrap(<StatusBar busy={false} fileCount={3} />);
    expect(screen.getByText("就绪")).toBeInTheDocument();
    expect(screen.getByText("纯本地处理")).toBeInTheDocument();
    expect(screen.getByText("3 个文件")).toBeInTheDocument();
    expect(await screen.findByText("MetaClean v0.4.1")).toBeInTheDocument();
  });

  it("shows bounded batch progress without exposing a file path", async () => {
    wrap(<StatusBar busy fileCount={3} progress={{ operation: "clean", batchId: "batch-1", completed: 2, total: 3, failed: 1, cancelled: false }} />);
    expect(screen.getByText("正在清理 2/3")).toBeInTheDocument();
    expect(screen.queryByText(/C:\\/)).not.toBeInTheDocument();
  });

  it("shows bounded scan progress without exposing a file path", () => {
    wrap(<StatusBar busy operation="scan" fileCount={3} progress={{ operation: "scan", batchId: "batch-1", completed: 2, total: 3, failed: 1, cancelled: false }} />);
    expect(screen.getByText("正在扫描 2/3")).toBeInTheDocument();
    expect(screen.queryByText(/C:\\/)).not.toBeInTheDocument();
  });

  it("labels an active scan separately from cleanup", () => {
    wrap(<StatusBar busy operation="scan" fileCount={2} />);
    expect(screen.getByText("正在扫描")).toBeInTheDocument();
  });

  it("shows a stopping state after cancellation is accepted", () => {
    wrap(<StatusBar busy fileCount={3} progress={{ operation: "clean", batchId: "batch-1", completed: 2, total: 3, failed: 1, cancelled: true }} />);
    expect(screen.getByText("正在停止…")).toBeInTheDocument();
  });

  it("adds native dialog selections and dropped browser files", async () => {
    const onAdd = vi.fn();
    openMock.mockResolvedValue(["C:\\work\\photo.jpg", "C:\\work\\paper.pdf"]);
    const onAddNativePaths = vi.fn().mockResolvedValue(undefined);
    wrap(<DropZone onAdd={onAdd} onAddNativePaths={onAddNativePaths} />);
    fireEvent.click(screen.getByRole("button", { name: "选择文件" }));
    await waitFor(() => expect(onAddNativePaths).toHaveBeenCalledWith(["C:\\work\\photo.jpg", "C:\\work\\paper.pdf"]));
    const zone = screen.getByText("拖入要净化的文件").closest("section")!;
    fireEvent.dragOver(zone);
    fireEvent.drop(zone, { dataTransfer: { files: [new File(["x"], "notes.md")] } });
    expect(onAdd).toHaveBeenLastCalledWith([expect.objectContaining({ name: "notes.md" })]);
  });

  it("falls back to the browser input when the native dialog is unavailable", async () => {
    const onAdd = vi.fn();
    openMock.mockRejectedValue(new Error("browser mode"));
    const { container } = wrap(<DropZone onAdd={onAdd} onAddNativePaths={vi.fn().mockRejectedValue(new Error("browser mode"))} />);
    const input = container.querySelector("input[type=file]") as HTMLInputElement;
    const click = vi.spyOn(input, "click");
    fireEvent.click(screen.getByRole("button", { name: "选择文件" }));
    await waitFor(() => expect(click).toHaveBeenCalled());
    fireEvent.change(input, { target: { files: [new File(["x"], "local.txt")] } });
    expect(onAdd).toHaveBeenCalledWith([expect.objectContaining({ name: "local.txt" })]);
  });

  it("renders queue findings, errors and removal controls", async () => {
    const onRemove = vi.fn();
    const entries: FileEntry[] = [
      { id: "1", name: "photo.jpg", path: "photo.jpg", kind: "image", status: "scanned", report: { path: "photo.jpg", name: "photo.jpg", format: "JPEG", size: 1, supported: true, findings: [{ category: "image_metadata", label: "metadata", count: 2, severity: "privacy" }] } },
      { id: "2", name: "bad.pdf", path: "bad.pdf", kind: "pdf", status: "error", report: { path: "bad.pdf", name: "bad.pdf", format: "PDF", size: 1, supported: false, findings: [], error: "格式损坏" } },
    ];
    wrap(<FileQueue entries={entries} preserveColorProfile removeExtendedAttributes={false} onRemove={onRemove} onClear={vi.fn()} onReveal={vi.fn()} onNotify={vi.fn()} />);
    expect(screen.getByText("发现 2 项痕迹")).toBeInTheDocument();
    expect(screen.getByText("图片元数据 · 2")).toBeInTheDocument();
    expect(screen.getByText("格式损坏")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "复制全部路径" }));
    await waitFor(() => expect(clipboardMock).toHaveBeenCalledWith("photo.jpg\r\nbad.pdf"));
    fireEvent.click(screen.getByRole("button", { name: "移除 photo.jpg" }));
    fireEvent.click(within(screen.getByRole("dialog", { name: "从队列中移除？" })).getByRole("button", { name: "移除" }));
    expect(onRemove).toHaveBeenCalledWith("1");
  });

  it("guards queue clearing until the destructive action is confirmed", () => {
    const onClear = vi.fn();
    wrap(<FileQueue entries={[{ id: "clear-me", name: "notes.txt", kind: "text", status: "ready" }]} preserveColorProfile removeExtendedAttributes={false} onRemove={vi.fn()} onClear={onClear} onReveal={vi.fn()} onNotify={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "清空" }));
    expect(onClear).not.toHaveBeenCalled();
    fireEvent.click(within(screen.getByRole("dialog", { name: "清空文件队列？" })).getByRole("button", { name: "清空队列" }));
    expect(onClear).toHaveBeenCalledOnce();
  });

  it("keeps a generated backup directly copyable and revealable after failure", async () => {
    const onReveal = vi.fn();
    const onNotify = vi.fn();
    const backupPath = "C:\\work\\photo.jpg.bak";
    const entries: FileEntry[] = [{
      id: "failed-replace",
      name: "photo.jpg",
      path: "C:\\work\\photo.jpg",
      kind: "image",
      status: "error",
      result: { sourcePath: "C:\\work\\photo.jpg", backupPath, removed: [], success: false, error: "写入失败" },
    }];
    wrap(<FileQueue entries={entries} preserveColorProfile removeExtendedAttributes={false} onRemove={vi.fn()} onClear={vi.fn()} onReveal={onReveal} onNotify={onNotify} />);
    expect(screen.getByText(`备份：${backupPath}`)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: `备份 · 复制路径 (${backupPath})` }));
    await waitFor(() => expect(clipboardMock).toHaveBeenCalledWith(backupPath));
    fireEvent.click(screen.getByRole("button", { name: `备份 · 在文件夹中显示 (${backupPath})` }));
    expect(onReveal).toHaveBeenCalledWith(backupPath);
  });

  it("includes generated backups in the batch path copy", async () => {
    const backupPath = "C:\\work\\photo.jpg.bak";
    wrap(<FileQueue entries={[{
      id: "failed-replace",
      name: "photo.jpg",
      path: "C:\\work\\photo.jpg",
      kind: "image",
      status: "error",
      result: { sourcePath: "C:\\work\\photo.jpg", backupPath, removed: [], success: false, error: "写入失败" },
    }]} preserveColorProfile removeExtendedAttributes={false} onRemove={vi.fn()} onClear={vi.fn()} onReveal={vi.fn()} onNotify={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "复制全部路径" }));
    await waitFor(() => expect(clipboardMock).toHaveBeenCalledWith(`C:\\work\\photo.jpg\r\n${backupPath}`));
  });

  it("deduplicates Windows aliases in the batch path copy", async () => {
    wrap(<FileQueue entries={[{
      id: "first",
      name: "photo.jpg",
      path: "C:\\work\\photo.jpg",
      kind: "image",
      status: "clean",
      result: { sourcePath: "C:\\work\\photo.jpg", removed: [], success: true },
    }, {
      id: "alias",
      name: "PHOTO.JPG",
      path: "\\\\?\\C:\\WORK\\PHOTO.JPG",
      kind: "image",
      status: "clean",
      result: { sourcePath: "\\\\?\\C:\\WORK\\PHOTO.JPG", removed: [], success: true },
    }]} preserveColorProfile removeExtendedAttributes={false} onRemove={vi.fn()} onClear={vi.fn()} onReveal={vi.fn()} onNotify={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "复制全部路径" }));
    await waitFor(() => expect(clipboardMock).toHaveBeenCalledWith("C:\\work\\photo.jpg"));
  });

  it("exports a bounded value-free audit report through the native command", async () => {
    saveMock.mockResolvedValue("C:\\reports\\metaclean-audit.json");
    const entries: FileEntry[] = [
      { id: "1", name: "photo.jpg", path: "C:\\private\\photo.jpg", kind: "image", status: "scanned", report: { path: "C:\\private\\photo.jpg", name: "photo.jpg", format: "JPEG", size: 12, supported: true, findings: [{ category: "image_metadata", label: "EXIF", count: 2, severity: "privacy" }] } },
    ];
    const onNotify = vi.fn();
    wrap(<FileQueue entries={entries} preserveColorProfile removeExtendedAttributes={false} onRemove={vi.fn()} onClear={vi.fn()} onReveal={vi.fn()} onNotify={onNotify} />);
    fireEvent.click(screen.getByRole("button", { name: "导出审计报告" }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("export_audit_report", expect.objectContaining({ path: "C:\\reports\\metaclean-audit.json" })));
    const contents = JSON.parse(invokeMock.mock.calls[0][1].contents as string);
    expect(contents).toMatchObject({ schemaVersion: 1, product: "MetaClean", version: "0.6.1", summary: { files: 1, findings: 2 } });
    expect(contents.files[0]).not.toHaveProperty("value");
    expect(onNotify).toHaveBeenCalledWith(expect.stringContaining("审计报告已导出"));
  });

  it("serializes audit exports before the save dialog resolves", async () => {
    let finishSave: ((path: string) => void) | undefined;
    saveMock.mockReturnValueOnce(new Promise((resolve) => { finishSave = resolve; }));
    const entries: FileEntry[] = [{
      id: "export-once",
      name: "notes.txt",
      path: "C:\\work\\notes.txt",
      kind: "text",
      status: "scanned",
      report: { path: "C:\\work\\notes.txt", name: "notes.txt", format: "Text", size: 4, supported: true, findings: [] },
    }];
    wrap(<FileQueue entries={entries} preserveColorProfile removeExtendedAttributes={false} onRemove={vi.fn()} onClear={vi.fn()} onReveal={vi.fn()} onNotify={vi.fn()} />);
    const exportButton = screen.getByRole("button", { name: "导出审计报告" });
    fireEvent.click(exportButton);
    await waitFor(() => expect(saveMock).toHaveBeenCalledOnce());
    fireEvent.click(exportButton);
    expect(saveMock).toHaveBeenCalledOnce();
    finishSave?.("C:\\reports\\once.json");
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("export_audit_report", expect.anything()));
  });

  it("renders empty and every queue lifecycle status", () => {
    const { rerender } = wrap(<FileQueue entries={[]} preserveColorProfile removeExtendedAttributes={false} onRemove={vi.fn()} onClear={vi.fn()} onReveal={vi.fn()} onNotify={vi.fn()} />);
    expect(screen.getByText("添加文件后，将在这里展示扫描状态")).toBeInTheDocument();
    const entries: FileEntry[] = [
      { id: "1", name: "scan.txt", kind: "text", status: "scanning" },
      { id: "2", name: "clean.txt", kind: "text", status: "clean" },
      { id: "3", name: "safe.txt", kind: "text", status: "scanned", report: { path: "safe.txt", name: "safe.txt", format: "Text", size: 1, supported: true, findings: [] } },
      { id: "4", name: "mystery.bin", kind: "unknown", status: "ready" },
      { id: "5", name: "failed.txt", kind: "text", status: "error", result: { sourcePath: "failed.txt", removed: [], success: false, error: "写入失败" } },
    ];
    rerender(<I18nProvider><FileQueue entries={entries} preserveColorProfile removeExtendedAttributes={false} onRemove={vi.fn()} onClear={vi.fn()} onReveal={vi.fn()} onNotify={vi.fn()} /></I18nProvider>);
    expect(screen.getByText("正在扫描…")).toBeInTheDocument();
    expect(screen.getByText("清理完成")).toBeInTheDocument();
    expect(screen.getByText("未发现隐私痕迹")).toBeInTheDocument();
    expect(screen.getByText("格式将在扫描时确认")).toBeInTheDocument();
    expect(screen.getByText("写入失败")).toBeInTheDocument();
  });

  it("sorts the queue stably and reveals cleaned output with its size delta", () => {
    const onReveal = vi.fn();
    const entries: FileEntry[] = [
      { id: "1", name: "zeta.txt", kind: "text", status: "scanned", report: { path: "zeta.txt", name: "zeta.txt", format: "Text", size: 2048, supported: true, findings: [] } },
      { id: "2", name: "alpha.jpg", kind: "image", status: "clean", result: { sourcePath: "alpha.jpg", outputPath: "alpha.cleaned.jpg", sourceSize: 2048, outputSize: 1024, removed: [], success: true } },
      { id: "3", name: "beta.txt", kind: "text", status: "scanned", report: { path: "beta.txt", name: "beta.txt", format: "Text", size: 2048, supported: true, findings: [] } },
    ];
    const { container } = wrap(<FileQueue entries={entries} preserveColorProfile removeExtendedAttributes={false} onRemove={vi.fn()} onClear={vi.fn()} onReveal={onReveal} onNotify={vi.fn()} />);
    expect([...container.querySelectorAll(".file-name strong")].map((element) => element.textContent)).toEqual(["alpha.jpg", "beta.txt", "zeta.txt"]);
    expect(screen.getByText("2.0 KB → 1.0 KB (−1.0 KB)")).toBeInTheDocument();
    fireEvent.change(screen.getByRole("combobox", { name: "待处理文件" }), { target: { value: "type" } });
    expect([...container.querySelectorAll(".file-name strong")].map((element) => element.textContent)).toEqual(["alpha.jpg", "zeta.txt", "beta.txt"]);
    fireEvent.click(screen.getByRole("button", { name: "改为降序" }));
    expect([...container.querySelectorAll(".file-name strong")].map((element) => element.textContent)).toEqual(["zeta.txt", "beta.txt", "alpha.jpg"]);
    fireEvent.click(screen.getByRole("button", { name: "alpha.cleaned.jpg" }));
    expect(onReveal).toHaveBeenCalledWith("alpha.cleaned.jpg");
  });

  it("switches cleanup mode and exposes every action state", () => {
    const onMode = vi.fn();
    const onAction = vi.fn();
    const onRemoveExtendedAttributesChange = vi.fn();
    const fidelity = { preserveTimestamps: true, onPreserveTimestampsChange: vi.fn(), preserveOrientation: true, onPreserveOrientationChange: vi.fn(), preserveColorProfile: true, onPreserveColorProfileChange: vi.fn(), removeExtendedAttributes: false, onRemoveExtendedAttributesChange };
    const { rerender } = wrap(<CleanOptions {...fidelity} mode="copy" onModeChange={onMode} disabled={false} scanned={false} hasFindings={false} busy={false} cancelable={false} cancelRequested={false} onCancel={vi.fn()} onAction={onAction} />);
    fireEvent.click(screen.getByText("替换原文件"));
    expect(onMode).toHaveBeenCalledWith("replace");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    expect(onAction).toHaveBeenCalled();
    fireEvent.click(screen.getByRole("checkbox", { name: /macOS · xattr/ }));
    expect(onRemoveExtendedAttributesChange).toHaveBeenCalledWith(true);
    rerender(<I18nProvider><CleanOptions {...fidelity} mode="replace" onModeChange={onMode} disabled={false} scanned hasFindings busy={false} cancelable={false} cancelRequested={false} onCancel={vi.fn()} onAction={onAction} /></I18nProvider>);
    expect(screen.getByRole("button", { name: "确认并开始清理" })).toBeEnabled();
    fireEvent.click(screen.getByText("保存为安全副本"));
    expect(onMode).toHaveBeenCalledWith("copy");
    rerender(<I18nProvider><CleanOptions {...fidelity} mode="replace" onModeChange={onMode} disabled={false} scanned hasFindings={false} busy={false} cancelable={false} cancelRequested={false} onCancel={vi.fn()} onAction={onAction} /></I18nProvider>);
    expect(screen.getByRole("button", { name: "没有需要清理的痕迹" })).toBeDisabled();
    const onCancel = vi.fn();
    rerender(<I18nProvider><CleanOptions {...fidelity} mode="replace" onModeChange={onMode} disabled={false} scanned hasFindings busy cancelable cancelRequested={false} onCancel={onCancel} onAction={onAction} /></I18nProvider>);
    fireEvent.click(screen.getByRole("button", { name: "取消处理" }));
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it("renders history success and failure details and clears it", () => {
    const onClear = vi.fn();
    const entries: HistoryEntry[] = [{ id: "job", createdAt: "2026-08-15T00:00:00Z", mode: "replace", results: [
      { sourcePath: "C:\\a.txt", outputPath: "C:\\a.txt", removed: [], success: true },
      { sourcePath: "C:\\b.txt", backupPath: "C:\\b.txt.bak", removed: [], success: false, error: "失败原因" },
    ] }];
    wrap(<HistoryPage entries={entries} onClear={onClear} />);
    expect(screen.getByText("1/2 成功")).toBeInTheDocument();
    expect(screen.getByText("失败原因")).toBeInTheDocument();
    expect(screen.getByText("备份：C:\\b.txt.bak")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "清空记录" }));
    expect(screen.getByRole("dialog", { name: "清空处理记录？" })).toBeInTheDocument();
    fireEvent.click(within(screen.getByRole("dialog", { name: "清空处理记录？" })).getByRole("button", { name: "清空记录" }));
    expect(onClear).toHaveBeenCalled();
  });

  it("renders the empty history state", () => {
    wrap(<HistoryPage entries={[]} onClear={vi.fn()} />);
    expect(screen.getByText("还没有处理记录")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "清空记录" })).toBeDisabled();
  });

  it("renders privacy scope and toggles Windows integration", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_context_menu_status") return Promise.resolve({ available: true, enabled: false, detail: "可启用" });
      if (command === "set_context_menu_enabled") return Promise.resolve({ available: true, enabled: true, detail: "已启用" });
      return Promise.reject(new Error(command));
    });
    const onMode = vi.fn();
    const onCloseToTrayChange = vi.fn();
    const { unmount } = wrap(<SettingsPage mode="copy" onModeChange={onMode} preserveTimestamps onPreserveTimestampsChange={vi.fn()} preserveOrientation onPreserveOrientationChange={vi.fn()} preserveColorProfile onPreserveColorProfileChange={vi.fn()} removeExtendedAttributes={false} onRemoveExtendedAttributesChange={vi.fn()} closeToTray={false} onCloseToTrayChange={onCloseToTrayChange} />);
    fireEvent.click(screen.getByRole("button", { name: "系统与更新" }));
    const enable = await screen.findByRole("button", { name: "启用" });
    fireEvent.click(enable);
    await screen.findByRole("button", { name: "停用" });
    fireEvent.click(screen.getByRole("checkbox", { name: "关闭按钮退出应用" }));
    expect(onCloseToTrayChange).toHaveBeenCalledWith(true);
    fireEvent.click(screen.getByRole("button", { name: "清理偏好" }));
    fireEvent.click(screen.getByText("替换并备份"));
    expect(onMode).toHaveBeenCalledWith("replace");
    unmount();
    wrap(<PrivacyPage />);
    expect(screen.getByText("文件纯本地处理")).toBeInTheDocument();
    expect(screen.getByText("当前支持范围")).toBeInTheDocument();
  });

  it("does not let an unmounted settings request overwrite a newer instance", async () => {
    let resolveFirst: ((status: { available: boolean; enabled: boolean; detail: string }) => void) | undefined;
    invokeMock
      .mockReturnValueOnce(new Promise((resolve) => { resolveFirst = resolve; }))
      .mockImplementation((command: string) => command === "get_context_menu_status"
        ? Promise.resolve({ available: true, enabled: false, detail: "当前状态" })
        : Promise.reject(new Error(command)));
    const first = wrap(<SettingsPage mode="copy" onModeChange={vi.fn()} preserveTimestamps onPreserveTimestampsChange={vi.fn()} preserveOrientation onPreserveOrientationChange={vi.fn()} preserveColorProfile onPreserveColorProfileChange={vi.fn()} removeExtendedAttributes={false} onRemoveExtendedAttributesChange={vi.fn()} closeToTray={false} onCloseToTrayChange={vi.fn()} />);
    first.unmount();
    wrap(<SettingsPage mode="copy" onModeChange={vi.fn()} preserveTimestamps onPreserveTimestampsChange={vi.fn()} preserveOrientation onPreserveOrientationChange={vi.fn()} preserveColorProfile onPreserveColorProfileChange={vi.fn()} removeExtendedAttributes={false} onRemoveExtendedAttributesChange={vi.fn()} closeToTray={false} onCloseToTrayChange={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "系统与更新" }));
    expect(await screen.findByRole("button", { name: "启用" })).toBeInTheDocument();
    resolveFirst?.({ available: true, enabled: true, detail: "旧实例" });
    await Promise.resolve();
    expect(screen.getByRole("button", { name: "启用" })).toBeInTheDocument();
  });

  it("shows runtime facts and copies or saves a bounded diagnostics report", async () => {
    invokeMock.mockImplementation((command: string, args?: { contents?: string }) => {
      if (command === "get_about_info") return Promise.resolve({
        version: "0.7.0",
        platform: "windows",
        arch: "x86_64",
        appDataDir: "C:\\Users\\tester\\AppData\\Roaming\\com.moresl.metaclean",
        executableDir: "C:\\Program Files\\MetaClean",
      });
      if (command === "export_audit_report") return args?.contents ? Promise.resolve(undefined) : Promise.reject(new Error("missing report"));
      return Promise.reject(new Error(command));
    });
    saveMock.mockResolvedValue("C:\\reports\\MetaClean-diagnostics.json");
    wrap(<AboutPage />);

    expect(await screen.findByText("v0.7.0 · windows-x86_64")).toBeInTheDocument();
    expect(screen.getByText("C:\\Program Files\\MetaClean")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "复制诊断信息" }));
    await waitFor(() => expect(clipboardMock).toHaveBeenCalledOnce());
    expect(screen.getByRole("status")).toHaveTextContent("已复制到剪贴板");
    const copied = JSON.parse(clipboardMock.mock.calls[0][0]);
    expect(copied).toMatchObject({ product: "MetaClean", version: "0.7.0", platform: "windows", arch: "x86_64" });
    expect(JSON.stringify(copied)).not.toContain("processedFiles");

    fireEvent.click(screen.getByRole("button", { name: "保存 JSON" }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("export_audit_report", expect.objectContaining({ path: "C:\\reports\\MetaClean-diagnostics.json" })));
    expect(revealItemMock).toHaveBeenCalledWith("C:\\reports\\MetaClean-diagnostics.json");

    fireEvent.click(screen.getByRole("link", { name: "报告问题" }));
    await waitFor(() => expect(openUrlMock).toHaveBeenCalledWith("https://github.com/Moresyl/metaclean/issues/new?labels=bug"));
  });

  it("does not continue exporting diagnostics after the about page unmounts", async () => {
    let finishSave: ((path: string) => void) | undefined;
    invokeMock.mockImplementation((command: string) => command === "get_about_info"
      ? Promise.resolve({ version: "0.7.0", platform: "windows", arch: "x86_64" })
      : Promise.reject(new Error(command)));
    saveMock.mockReturnValueOnce(new Promise((resolve) => { finishSave = resolve; }));
    const rendered = wrap(<AboutPage />);
    await screen.findByText("v0.7.0 · windows-x86_64");
    fireEvent.click(screen.getByRole("button", { name: "保存 JSON" }));
    await waitFor(() => expect(saveMock).toHaveBeenCalledOnce());
    rendered.unmount();
    finishSave?.("C:\\reports\\late.json");
    await Promise.resolve();
    expect(invokeMock).not.toHaveBeenCalledWith("export_audit_report", expect.anything());
  });

  it("builds diagnostics from explicit runtime facts without file history", () => {
    const report = JSON.parse(buildDiagnosticReport({
      version: "0.7.0",
      platform: "linux",
      arch: "x86_64",
      appDataDir: "/home/test/.local/share/metaclean",
      executableDir: "/opt/metaclean",
    }, {
      locale: "zh",
      updateStatus: "current",
      portable: false,
      selfUpdateSupported: true,
    }));
    expect(report.runtime).toEqual({ portable: false, selfUpdateSupported: true });
    expect(report.paths.executableDirectory).toBe("/opt/metaclean");
    expect(report).not.toHaveProperty("history");
  });

  it("selects a folder for recursive native expansion", async () => {
    const onAddNativePaths = vi.fn().mockResolvedValue(undefined);
    openMock.mockResolvedValue("C:\\photos");
    wrap(<DropZone onAdd={vi.fn()} onAddNativePaths={onAddNativePaths} />);
    fireEvent.click(screen.getByRole("button", { name: "选择文件夹" }));
    await waitFor(() => expect(onAddNativePaths).toHaveBeenCalledWith(["C:\\photos"]));
    expect(openMock).toHaveBeenCalledWith({ multiple: false, directory: true });
  });

  it("discovers and opens a newer stable release", async () => {
    invokeMock.mockResolvedValue({ available: false, enabled: false, detail: "仅 Windows" });
    checkForUpdateMock.mockResolvedValue({
      status: "available",
      info: {
        currentVersion: "0.1.0",
        availableVersion: "0.2.0",
        name: "MetaClean v0.2.0",
        releaseUrl: "https://github.com/Moresyl/metaclean/releases/tag/v0.2.0",
      },
    });
    wrap(<SettingsPage mode="copy" onModeChange={vi.fn()} preserveTimestamps onPreserveTimestampsChange={vi.fn()} preserveOrientation onPreserveOrientationChange={vi.fn()} preserveColorProfile onPreserveColorProfileChange={vi.fn()} removeExtendedAttributes={false} onRemoveExtendedAttributesChange={vi.fn()} closeToTray={false} onCloseToTrayChange={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "系统与更新" }));
    fireEvent.click(screen.getByRole("button", { name: "检查更新" }));
    const download = await screen.findByRole("button", { name: "前往 GitHub" });
    expect(screen.getByText(/可更新到 v0.2.0/)).toBeInTheDocument();
    fireEvent.click(download);
    await waitFor(() => expect(openUrlMock).toHaveBeenCalledWith("https://github.com/Moresyl/metaclean/releases/tag/v0.2.0"));
  });

  it("shows unavailable Windows integration without enabling it", async () => {
    invokeMock.mockResolvedValue({ available: false, enabled: false, detail: "仅 Windows" });
    wrap(<SettingsPage mode="replace" onModeChange={vi.fn()} preserveTimestamps onPreserveTimestampsChange={vi.fn()} preserveOrientation onPreserveOrientationChange={vi.fn()} preserveColorProfile onPreserveColorProfileChange={vi.fn()} removeExtendedAttributes={false} onRemoveExtendedAttributesChange={vi.fn()} closeToTray={false} onCloseToTrayChange={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "系统与更新" }));
    expect(await screen.findByRole("button", { name: "启用" })).toBeDisabled();
  });

  it("reports Windows integration failures without leaving the control busy", async () => {
    invokeMock.mockImplementation((command: string) => command === "get_context_menu_status"
      ? Promise.resolve({ available: true, enabled: false, detail: "可启用" })
      : Promise.reject(new Error("registry denied")));
    wrap(<SettingsPage mode="copy" onModeChange={vi.fn()} preserveTimestamps onPreserveTimestampsChange={vi.fn()} preserveOrientation onPreserveOrientationChange={vi.fn()} preserveColorProfile onPreserveColorProfileChange={vi.fn()} removeExtendedAttributes={false} onRemoveExtendedAttributesChange={vi.fn()} closeToTray={false} onCloseToTrayChange={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "系统与更新" }));
    const enable = await screen.findByRole("button", { name: "启用" });
    fireEvent.click(enable);
    expect(await screen.findByText(/更新右键菜单失败.*registry denied/)).toBeInTheDocument();
    await waitFor(() => expect(enable).toBeEnabled());
  });

  it("serializes repeated context-menu toggles before the first result returns", async () => {
    let finishToggle: ((status: { available: boolean; enabled: boolean; detail: string }) => void) | undefined;
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_context_menu_status") return Promise.resolve({ available: true, enabled: false, detail: "可启用" });
      if (command === "set_context_menu_enabled") return new Promise((resolve) => { finishToggle = resolve; });
      return Promise.reject(new Error(command));
    });
    wrap(<SettingsPage mode="copy" onModeChange={vi.fn()} preserveTimestamps onPreserveTimestampsChange={vi.fn()} preserveOrientation onPreserveOrientationChange={vi.fn()} preserveColorProfile onPreserveColorProfileChange={vi.fn()} removeExtendedAttributes={false} onRemoveExtendedAttributesChange={vi.fn()} closeToTray={false} onCloseToTrayChange={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "系统与更新" }));
    const enable = await screen.findByRole("button", { name: "启用" });
    fireEvent.click(enable);
    fireEvent.click(enable);
    expect(invokeMock.mock.calls.filter(([command]) => command === "set_context_menu_enabled")).toHaveLength(1);
    finishToggle?.({ available: true, enabled: true, detail: "已启用" });
    expect(await screen.findByRole("button", { name: "停用" })).toBeInTheDocument();
  });

  it("shows release notes and opens the GitHub release from the update prompt", async () => {
    checkForUpdateMock.mockResolvedValue({
      status: "available",
      info: {
        currentVersion: "0.4.1",
        availableVersion: "0.5.0",
        name: "MetaClean v0.5.0",
        notes: "新增原生设置分栏\n修复空窗口菜单",
        publishedAt: "2026-08-20T00:00:00Z",
        releaseUrl: "https://github.com/Moresyl/metaclean/releases/tag/v0.5.0",
      },
    });
    wrap(<UpdateDialogHarness />);
    fireEvent.click(screen.getByRole("button", { name: "trigger update" }));
    expect(await screen.findByRole("dialog", { name: "v0.5.0" })).toHaveTextContent("新增原生设置分栏");
    fireEvent.click(screen.getByRole("button", { name: /前往 GitHub 查看并下载/ }));
    await waitFor(() => expect(openUrlMock).toHaveBeenCalledWith("https://github.com/Moresyl/metaclean/releases/tag/v0.5.0"));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("installs a signed update directly from the update prompt", async () => {
    getUpdateRuntimeMock.mockResolvedValue({ selfUpdateSupported: true, portable: false });
    checkForUpdateMock.mockResolvedValue({
      status: "available",
      info: {
        currentVersion: "0.4.1",
        availableVersion: "0.5.0",
        name: "MetaClean v0.5.0",
        releaseUrl: "https://github.com/Moresyl/metaclean/releases/tag/v0.5.0",
      },
    });
    installAvailableUpdateMock.mockImplementation(async ({ onProgress }) => {
      onProgress({ stage: "downloading", downloaded: 40, total: 100 });
      return false;
    });
    wrap(<UpdateDialogHarness />);
    fireEvent.click(screen.getByRole("button", { name: "trigger update" }));
    const install = await screen.findByRole("button", { name: "一键安装更新" });
    fireEvent.click(install);
    await waitFor(() => expect(installAvailableUpdateMock).toHaveBeenCalledWith(expect.objectContaining({ expectedVersion: "0.5.0" })));
    expect(openUrlMock).not.toHaveBeenCalled();
  });

  it("keeps keyboard focus inside the update prompt and restores it on close", async () => {
    checkForUpdateMock.mockResolvedValue({
      status: "available",
      info: { currentVersion: "0.4.1", availableVersion: "0.5.0", name: "MetaClean v0.5.0", releaseUrl: "https://github.com/Moresyl/metaclean/releases/tag/v0.5.0" },
    });
    wrap(<UpdateDialogHarness />);
    const trigger = screen.getByRole("button", { name: "trigger update" });
    trigger.focus();
    fireEvent.click(trigger);
    await screen.findByRole("dialog", { name: "v0.5.0" });
    expect(screen.getByRole("button", { name: /前往 GitHub 查看并下载/ })).toHaveFocus();
    fireEvent.keyDown(window, { key: "Tab" });
    expect(screen.getByRole("button", { name: "稍后提醒" })).toHaveFocus();
    fireEvent.keyDown(window, { key: "Tab", shiftKey: true });
    expect(screen.getByRole("button", { name: /前往 GitHub 查看并下载/ })).toHaveFocus();
    fireEvent.click(screen.getByRole("button", { name: "稍后提醒" }));
    expect(trigger).toHaveFocus();
  });

  it("persists the selected interface theme", async () => {
    invokeMock.mockResolvedValue({ available: false, enabled: false, detail: "仅 Windows" });
    wrap(<SettingsPage mode="copy" onModeChange={vi.fn()} preserveTimestamps onPreserveTimestampsChange={vi.fn()} preserveOrientation onPreserveOrientationChange={vi.fn()} preserveColorProfile onPreserveColorProfileChange={vi.fn()} removeExtendedAttributes={false} onRemoveExtendedAttributesChange={vi.fn()} closeToTray={false} onCloseToTrayChange={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "深色" }));
    await waitFor(() => expect(localStorage.getItem("metaclean.theme")).toBe("dark"));
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("rejects i18n usage outside its provider", () => {
    function InvalidConsumer() { useI18n(); return null; }
    expect(() => render(<InvalidConsumer />)).toThrow("useI18n must be used inside I18nProvider");
  });
});
