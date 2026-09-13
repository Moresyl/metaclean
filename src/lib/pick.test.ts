import { beforeEach, describe, expect, it, vi } from "vitest";
import { PickerUnavailableError, pickPaths } from "./pick";

const openMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: openMock }));

describe("pickPaths", () => {
  beforeEach(() => openMock.mockReset());

  it("normalizes a single selection and forwards directory mode", async () => {
    openMock.mockResolvedValue("C:\\work\\photo.jpg");
    await expect(pickPaths(false)).resolves.toEqual(["C:\\work\\photo.jpg"]);
    expect(openMock).toHaveBeenCalledWith({ multiple: true, directory: false });
  });

  it("preserves cancellation as a null result", async () => {
    openMock.mockResolvedValue(null);
    await expect(pickPaths(true)).resolves.toBeNull();
    expect(openMock).toHaveBeenCalledWith({ multiple: false, directory: true });
  });

  it("distinguishes an unavailable dialog from a malformed response", async () => {
    openMock.mockRejectedValue(new Error("dialog unavailable"));
    await expect(pickPaths(false)).rejects.toBeInstanceOf(PickerUnavailableError);

    openMock.mockResolvedValue({ paths: ["C:\\work\\photo.jpg"] });
    await expect(pickPaths(false)).rejects.toThrow(/无效或超限/u);
  });

  it("rejects an oversized native selection before returning paths", async () => {
    openMock.mockResolvedValue(["x".repeat(32 * 1024 + 1)]);
    await expect(pickPaths(false)).rejects.toThrow(/无效或超限/u);
  });
});
