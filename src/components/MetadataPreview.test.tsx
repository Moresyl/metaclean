import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import MetadataPreview from "./MetadataPreview";
import { I18nProvider } from "../lib/i18n";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
beforeEach(() => { invoke.mockReset(); localStorage.clear(); });
const show = (path = "C:/report.docx", disabled = false) => <I18nProvider><MetadataPreview path={path} disabled={disabled} /></I18nProvider>;

it("loads only on demand, renders values as text and clears on close", async () => {
  invoke.mockResolvedValue({ fields: [{ name: "creator", source: "core.xml", value: "<img src=x onerror=alert(1)>张三" }], truncated: true });
  const view = render(show());
  expect(invoke).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button"));
  expect(await screen.findByText("<img src=x onerror=alert(1)>张三")).toBeVisible();
  expect(view.container.querySelector("img")).toBeNull();
  expect(invoke).toHaveBeenCalledWith("preview_metadata", { path: "C:/report.docx" });
  expect(screen.getByText(/128/)).toBeVisible();
  fireEvent.click(screen.getByRole("button"));
  expect(screen.queryByText(/张三/)).toBeNull();
  expect(JSON.stringify(localStorage)).not.toContain("张三");
});

it("does not show late responses after dismissal or a file switch", async () => {
  let resolve!: (value: unknown) => void;
  invoke.mockImplementationOnce(() => new Promise(r => { resolve = r; }));
  const view = render(show());
  fireEvent.click(screen.getByRole("button"));
  fireEvent.click(screen.getByRole("button"));
  await act(async () => resolve({ fields: [{ name: "author", source: "old", value: "Old private value" }], truncated: false }));
  expect(screen.queryByText("Old private value")).toBeNull();
  invoke.mockResolvedValue({ fields: [], truncated: false });
  view.rerender(show("C:/next.pdf"));
  fireEvent.click(screen.getByRole("button"));
  expect(await screen.findByText(/No previewable|未发现可预览/)).toBeVisible();
});

it("hides values during cleanup and does not render native error contents", async () => {
  invoke.mockRejectedValue(new Error("private secret author"));
  const view = render(show());
  fireEvent.click(screen.getByRole("button"));
  expect(await screen.findByRole("alert")).not.toHaveTextContent("private secret");
  view.rerender(show("C:/report.docx", true));
  expect(screen.getByRole("button")).toBeDisabled();
  expect(screen.queryByRole("alert")).toBeNull();
});
