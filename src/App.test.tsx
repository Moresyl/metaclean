import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import App from "./App";
import { I18nProvider } from "./lib/i18n";

vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: () => ({ onDragDropEvent: () => Promise.resolve(() => undefined) }) }));

describe("App", () => {
  const renderApp = () => render(<I18nProvider><App /></I18nProvider>);
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
});
