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

const openMock = vi.hoisted(() => vi.fn());
const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: openMock }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

const wrap = (node: React.ReactNode) => render(<I18nProvider>{node}</I18nProvider>);

beforeEach(() => {
  openMock.mockReset();
  invokeMock.mockReset();
});

describe("desktop components", () => {
  it("adds native dialog selections and dropped browser files", async () => {
    const onAdd = vi.fn();
    openMock.mockResolvedValue(["C:\\work\\photo.jpg", "C:\\work\\paper.pdf"]);
    wrap(<DropZone onAdd={onAdd} />);
    fireEvent.click(screen.getByRole("button", { name: "选择文件" }));
    await waitFor(() => expect(onAdd).toHaveBeenCalledWith(expect.arrayContaining([expect.objectContaining({ name: "photo.jpg" }), expect.objectContaining({ name: "paper.pdf" })])));
    const zone = screen.getByText("拖入要净化的文件").closest("section")!;
    fireEvent.dragOver(zone);
    fireEvent.drop(zone, { dataTransfer: { files: [new File(["x"], "notes.md")] } });
    expect(onAdd).toHaveBeenLastCalledWith([expect.objectContaining({ name: "notes.md" })]);
  });

  it("falls back to the browser input when the native dialog is unavailable", async () => {
    const onAdd = vi.fn();
    openMock.mockRejectedValue(new Error("browser mode"));
    const { container } = wrap(<DropZone onAdd={onAdd} />);
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
    wrap(<FileQueue entries={entries} onRemove={onRemove} onClear={vi.fn()} />);
    expect(screen.getByText("发现 2 项痕迹")).toBeInTheDocument();
    expect(screen.getByText("图片元数据 · 2")).toBeInTheDocument();
    expect(screen.getByText("格式损坏")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "移除 photo.jpg" }));
    expect(onRemove).toHaveBeenCalledWith("1");
  });

  it("renders empty and every queue lifecycle status", () => {
    const { rerender } = wrap(<FileQueue entries={[]} onRemove={vi.fn()} onClear={vi.fn()} />);
    expect(screen.getByText("添加文件后，将在这里展示扫描状态")).toBeInTheDocument();
    const entries: FileEntry[] = [
      { id: "1", name: "scan.txt", kind: "text", status: "scanning" },
      { id: "2", name: "clean.txt", kind: "text", status: "clean" },
      { id: "3", name: "safe.txt", kind: "text", status: "scanned", report: { path: "safe.txt", name: "safe.txt", format: "Text", size: 1, supported: true, findings: [] } },
      { id: "4", name: "mystery.bin", kind: "unknown", status: "ready" },
    ];
    rerender(<I18nProvider><FileQueue entries={entries} onRemove={vi.fn()} onClear={vi.fn()} /></I18nProvider>);
    expect(screen.getByText("正在扫描…")).toBeInTheDocument();
    expect(screen.getByText("清理完成")).toBeInTheDocument();
    expect(screen.getByText("未发现隐私痕迹")).toBeInTheDocument();
    expect(screen.getByText("格式将在扫描时确认")).toBeInTheDocument();
  });

  it("switches cleanup mode and exposes every action state", () => {
    const onMode = vi.fn();
    const onAction = vi.fn();
    const { rerender } = wrap(<CleanOptions mode="copy" onModeChange={onMode} disabled={false} scanned={false} hasFindings={false} busy={false} onAction={onAction} />);
    fireEvent.click(screen.getByText("替换原文件"));
    expect(onMode).toHaveBeenCalledWith("replace");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    expect(onAction).toHaveBeenCalled();
    rerender(<I18nProvider><CleanOptions mode="replace" onModeChange={onMode} disabled={false} scanned hasFindings busy={false} onAction={onAction} /></I18nProvider>);
    expect(screen.getByRole("button", { name: "确认并开始清理" })).toBeEnabled();
    fireEvent.click(screen.getByText("保存为安全副本"));
    expect(onMode).toHaveBeenCalledWith("copy");
    rerender(<I18nProvider><CleanOptions mode="replace" onModeChange={onMode} disabled={false} scanned hasFindings={false} busy={false} onAction={onAction} /></I18nProvider>);
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
    const { unmount } = wrap(<SettingsPage mode="copy" onModeChange={onMode} />);
    const enable = await screen.findByRole("button", { name: "启用" });
    fireEvent.click(enable);
    await screen.findByRole("button", { name: "停用" });
    fireEvent.click(screen.getByText("替换并备份"));
    expect(onMode).toHaveBeenCalledWith("replace");
    unmount();
    wrap(<PrivacyPage />);
    expect(screen.getByText("纯本地运行")).toBeInTheDocument();
    expect(screen.getByText("当前支持范围")).toBeInTheDocument();
  });

  it("shows unavailable Windows integration without enabling it", async () => {
    invokeMock.mockResolvedValue({ available: false, enabled: false, detail: "仅 Windows" });
    wrap(<SettingsPage mode="replace" onModeChange={vi.fn()} />);
    expect(await screen.findByRole("button", { name: "启用" })).toBeDisabled();
  });

  it("rejects i18n usage outside its provider", () => {
    function InvalidConsumer() { useI18n(); return null; }
    expect(() => render(<InvalidConsumer />)).toThrow("useI18n must be used inside I18nProvider");
  });
});
