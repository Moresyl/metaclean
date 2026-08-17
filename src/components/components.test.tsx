import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import CleanOptions from "./CleanOptions";
import DropZone from "./DropZone";
import FileQueue from "./FileQueue";
import HistoryPage from "./HistoryPage";
import PrivacyPage from "./PrivacyPage";
import SettingsPage from "./SettingsPage";
import { I18nProvider, useI18n } from "../lib/i18n";
import type { FileEntry, HistoryEntry } from "../types";
import { UpdateProvider } from "../contexts/UpdateContext";
import { ThemeProvider } from "../contexts/ThemeContext";

const openMock = vi.hoisted(() => vi.fn());
const invokeMock = vi.hoisted(() => vi.fn());
const checkForUpdateMock = vi.hoisted(() => vi.fn());
const openUrlMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: openMock }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: openUrlMock }));
vi.mock("../lib/update", () => ({
  RELEASES_PAGE_URL: "https://github.com/Moresyl/metaclean/releases/latest",
  checkForUpdate: checkForUpdateMock,
}));

const wrap = (node: React.ReactNode) => render(<ThemeProvider initialMode="light"><I18nProvider><UpdateProvider>{node}</UpdateProvider></I18nProvider></ThemeProvider>);

beforeEach(() => {
  openMock.mockReset();
  invokeMock.mockReset();
  checkForUpdateMock.mockReset();
  checkForUpdateMock.mockResolvedValue({ status: "current", currentVersion: "0.1.0" });
  openUrlMock.mockReset();
});

describe("desktop components", () => {
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

  it("renders queue findings, errors and removal controls", () => {
    const onRemove = vi.fn();
    const entries: FileEntry[] = [
      { id: "1", name: "photo.jpg", path: "photo.jpg", kind: "image", status: "scanned", report: { path: "photo.jpg", name: "photo.jpg", format: "JPEG", size: 1, supported: true, findings: [{ category: "image_metadata", label: "metadata", count: 2, severity: "privacy" }] } },
      { id: "2", name: "bad.pdf", path: "bad.pdf", kind: "pdf", status: "error", report: { path: "bad.pdf", name: "bad.pdf", format: "PDF", size: 1, supported: false, findings: [], error: "格式损坏" } },
    ];
    wrap(<FileQueue entries={entries} preserveColorProfile onRemove={onRemove} onClear={vi.fn()} />);
    expect(screen.getByText("发现 2 项痕迹")).toBeInTheDocument();
    expect(screen.getByText("图片元数据 · 2")).toBeInTheDocument();
    expect(screen.getByText("格式损坏")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "移除 photo.jpg" }));
    expect(onRemove).toHaveBeenCalledWith("1");
  });

  it("renders empty and every queue lifecycle status", () => {
    const { rerender } = wrap(<FileQueue entries={[]} preserveColorProfile onRemove={vi.fn()} onClear={vi.fn()} />);
    expect(screen.getByText("添加文件后，将在这里展示扫描状态")).toBeInTheDocument();
    const entries: FileEntry[] = [
      { id: "1", name: "scan.txt", kind: "text", status: "scanning" },
      { id: "2", name: "clean.txt", kind: "text", status: "clean" },
      { id: "3", name: "safe.txt", kind: "text", status: "scanned", report: { path: "safe.txt", name: "safe.txt", format: "Text", size: 1, supported: true, findings: [] } },
      { id: "4", name: "mystery.bin", kind: "unknown", status: "ready" },
    ];
    rerender(<I18nProvider><FileQueue entries={entries} preserveColorProfile onRemove={vi.fn()} onClear={vi.fn()} /></I18nProvider>);
    expect(screen.getByText("正在扫描…")).toBeInTheDocument();
    expect(screen.getByText("清理完成")).toBeInTheDocument();
    expect(screen.getByText("未发现隐私痕迹")).toBeInTheDocument();
    expect(screen.getByText("格式将在扫描时确认")).toBeInTheDocument();
  });

  it("switches cleanup mode and exposes every action state", () => {
    const onMode = vi.fn();
    const onAction = vi.fn();
    const fidelity = { preserveTimestamps: true, onPreserveTimestampsChange: vi.fn(), preserveOrientation: true, onPreserveOrientationChange: vi.fn(), preserveColorProfile: true, onPreserveColorProfileChange: vi.fn() };
    const { rerender } = wrap(<CleanOptions {...fidelity} mode="copy" onModeChange={onMode} disabled={false} scanned={false} hasFindings={false} busy={false} onAction={onAction} />);
    fireEvent.click(screen.getByText("替换原文件"));
    expect(onMode).toHaveBeenCalledWith("replace");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    expect(onAction).toHaveBeenCalled();
    rerender(<I18nProvider><CleanOptions {...fidelity} mode="replace" onModeChange={onMode} disabled={false} scanned hasFindings busy={false} onAction={onAction} /></I18nProvider>);
    expect(screen.getByRole("button", { name: "确认并开始清理" })).toBeEnabled();
    fireEvent.click(screen.getByText("保存为安全副本"));
    expect(onMode).toHaveBeenCalledWith("copy");
    rerender(<I18nProvider><CleanOptions {...fidelity} mode="replace" onModeChange={onMode} disabled={false} scanned hasFindings={false} busy={false} onAction={onAction} /></I18nProvider>);
    expect(screen.getByRole("button", { name: "没有需要清理的痕迹" })).toBeDisabled();
  });

  it("renders history success and failure details and clears it", () => {
    const onClear = vi.fn();
    const entries: HistoryEntry[] = [{ id: "job", createdAt: "2026-08-15T00:00:00Z", mode: "replace", results: [
      { sourcePath: "C:\\a.txt", outputPath: "C:\\a.txt", removed: [], success: true },
      { sourcePath: "C:\\b.txt", removed: [], success: false, error: "失败原因" },
    ] }];
    wrap(<HistoryPage entries={entries} onClear={onClear} />);
    expect(screen.getByText("1/2 成功")).toBeInTheDocument();
    expect(screen.getByText("失败原因")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "清空记录" }));
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
    const { unmount } = wrap(<SettingsPage mode="copy" onModeChange={onMode} preserveTimestamps onPreserveTimestampsChange={vi.fn()} preserveOrientation onPreserveOrientationChange={vi.fn()} preserveColorProfile onPreserveColorProfileChange={vi.fn()} />);
    const enable = await screen.findByRole("button", { name: "启用" });
    fireEvent.click(enable);
    await screen.findByRole("button", { name: "停用" });
    fireEvent.click(screen.getByText("替换并备份"));
    expect(onMode).toHaveBeenCalledWith("replace");
    unmount();
    wrap(<PrivacyPage />);
    expect(screen.getByText("文件纯本地处理")).toBeInTheDocument();
    expect(screen.getByText("当前支持范围")).toBeInTheDocument();
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
    wrap(<SettingsPage mode="copy" onModeChange={vi.fn()} preserveTimestamps onPreserveTimestampsChange={vi.fn()} preserveOrientation onPreserveOrientationChange={vi.fn()} preserveColorProfile onPreserveColorProfileChange={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "检查更新" }));
    const download = await screen.findByRole("button", { name: "下载新版本" });
    expect(screen.getByText(/可更新到 0.2.0/)).toBeInTheDocument();
    fireEvent.click(download);
    await waitFor(() => expect(openUrlMock).toHaveBeenCalledWith("https://github.com/Moresyl/metaclean/releases/tag/v0.2.0"));
  });

  it("shows unavailable Windows integration without enabling it", async () => {
    invokeMock.mockResolvedValue({ available: false, enabled: false, detail: "仅 Windows" });
    wrap(<SettingsPage mode="replace" onModeChange={vi.fn()} preserveTimestamps onPreserveTimestampsChange={vi.fn()} preserveOrientation onPreserveOrientationChange={vi.fn()} preserveColorProfile onPreserveColorProfileChange={vi.fn()} />);
    expect(await screen.findByRole("button", { name: "启用" })).toBeDisabled();
  });

  it("persists the selected interface theme", async () => {
    invokeMock.mockResolvedValue({ available: false, enabled: false, detail: "仅 Windows" });
    wrap(<SettingsPage mode="copy" onModeChange={vi.fn()} preserveTimestamps onPreserveTimestampsChange={vi.fn()} preserveOrientation onPreserveOrientationChange={vi.fn()} preserveColorProfile onPreserveColorProfileChange={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "深色" }));
    await waitFor(() => expect(localStorage.getItem("metaclean.theme")).toBe("dark"));
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("rejects i18n usage outside its provider", () => {
    function InvalidConsumer() { useI18n(); return null; }
    expect(() => render(<InvalidConsumer />)).toThrow("useI18n must be used inside I18nProvider");
  });
});
