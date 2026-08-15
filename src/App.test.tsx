import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import App from "./App";

vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: () => ({ onDragDropEvent: () => Promise.resolve(() => undefined) }) }));

describe("App", () => {
  it("starts with scanning disabled", () => {
    render(<App />);
    expect(screen.getByRole("button", { name: "扫描隐私痕迹" })).toBeDisabled();
  });

  it("adds a dropped file and enables scanning", () => {
    render(<App />);
    const zone = screen.getByText("拖入要净化的文件").closest("section");
    fireEvent.drop(zone!, { dataTransfer: { files: [new File(["hello"], "notes.md", { type: "text/markdown" })] } });
    expect(screen.getByText("notes.md")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "扫描隐私痕迹" })).toBeEnabled();
  });
});
