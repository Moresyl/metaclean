import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { I18nProvider } from "./lib/i18n";
import { UpdateProvider } from "./contexts/UpdateContext";

vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: () => ({ onDragDropEvent: () => Promise.resolve(() => undefined) }) }));
const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

describe("App", () => {
  const renderApp = () => render(<I18nProvider><UpdateProvider><App /></UpdateProvider></I18nProvider>);
  beforeEach(() => invokeMock.mockImplementation((command?: string) => command === "get_launch_paths" || command === undefined ? Promise.resolve([]) : command === "expand_paths" ? Promise.resolve({ files: [], skippedCount: 0, issues: [], limitReached: false }) : Promise.reject(new Error(`unexpected ${command}`))));
  it("starts with scanning disabled", () => {
    renderApp();
    expect(screen.getByRole("button", { name: "扫描隐私痕迹" })).toBeDisabled();
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
    fireEvent.click(screen.getByRole("button", { name: "English" }));
    expect(screen.getByRole("button", { name: "Clean files" })).toBeInTheDocument();
    expect(screen.getByText("Default output mode")).toBeInTheDocument();
    expect(localStorage.getItem("metaclean.locale")).toBe("en");
  });

  it("scans native launch files, cleans findings, and stores history", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_launch_paths") return Promise.resolve(["C:\\work\\notes.txt"]);
      if (command === "expand_paths") return Promise.resolve({ files: ["C:\\work\\notes.txt"], skippedCount: 0, issues: [], limitReached: false });
      if (command === "scan_files") return Promise.resolve([{ path: "C:\\work\\notes.txt", name: "notes.txt", format: "Text", size: 4, supported: true, findings: [{ category: "unicode", label: "Invisible Unicode", count: 1, severity: "privacy" }] }]);
      if (command === "clean_files") return Promise.resolve([{ sourcePath: "C:\\work\\notes.txt", outputPath: "C:\\work\\notes.cleaned.txt", removed: [], success: true }]);
      if (command === undefined) return Promise.resolve([]);
      return Promise.reject(new Error(`unexpected ${command}`));
    });
    renderApp();
    await screen.findByText("notes.txt");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await screen.findByText("发现 1 项痕迹");
    fireEvent.click(screen.getByRole("button", { name: "确认并开始清理" }));
    await screen.findByText(/1 个文件清理完成/);
    expect(JSON.parse(localStorage.getItem("metaclean.history") ?? "[]")).toHaveLength(1);
  });

  it("reports native scan failures without modifying files", async () => {
    invokeMock.mockImplementation((command?: string) => command === "get_launch_paths" ? Promise.resolve(["C:\\broken.pdf"]) : command === "expand_paths" ? Promise.resolve({ files: ["C:\\broken.pdf"], skippedCount: 0, issues: [], limitReached: false }) : command === undefined ? Promise.resolve([]) : Promise.reject(new Error("engine unavailable")));
    renderApp();
    await screen.findByText("broken.pdf");
    fireEvent.click(screen.getByRole("button", { name: "扫描隐私痕迹" }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("扫描失败"));
  });
});
