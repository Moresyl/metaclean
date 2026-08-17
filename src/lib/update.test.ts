import { beforeEach, describe, expect, it, vi } from "vitest";
import { checkForUpdate, compareVersions, getUpdateRuntime, installAvailableUpdate } from "./update";

const getVersionMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/app", () => ({ getVersion: getVersionMock }));

describe("compareVersions", () => {
  it.each([
    ["1.2.0", "1.1.9", 1],
    ["v1.2", "1.2.0", 0],
    ["1.2.0-beta.2", "1.2.0-beta.10", -1],
    ["1.2.0", "1.2.0-rc.1", 1],
    ["1.2.0+build.9", "1.2.0+build.1", 0],
    ["1.2.0-alpha", "1.2.0-alpha.1", -1],
    ["1.2.0-alpha.1", "1.2.0-alpha", 1],
    ["1.2.0-1", "1.2.0-alpha", -1],
    ["1.2.0-alpha", "1.2.0-1", 1],
    ["1.2.0-zeta", "1.2.0-alpha", 1],
  ])("compares %s with %s", (left, right, expected) => {
    expect(compareVersions(left, right)).toBe(expected);
  });

  it("rejects malformed versions", () => {
    expect(() => compareVersions("latest", "1.0.0")).toThrow("Invalid version");
  });
});

describe("checkForUpdate", () => {
  beforeEach(() => getVersionMock.mockResolvedValue("0.3.0"));

  it("returns signed native updater metadata for a newer stable release", async () => {
    const checker = vi.fn().mockResolvedValue({
      currentVersion: "0.3.0",
      version: "0.4.0",
      body: "Signed update",
      date: "2026-08-18T00:00:00Z",
    });
    await expect(checkForUpdate({ checker, timeoutMs: 2500 })).resolves.toEqual({
      status: "available",
      info: {
        currentVersion: "0.3.0",
        availableVersion: "0.4.0",
        name: "MetaClean v0.4.0",
        notes: "Signed update",
        publishedAt: "2026-08-18T00:00:00Z",
        releaseUrl: "https://github.com/Moresyl/metaclean/releases/tag/v0.4.0",
      },
    });
    expect(checker).toHaveBeenCalledWith({ timeout: 2500 });
  });

  it("reports current versions and uses the installed version when no update exists", async () => {
    const checker = vi.fn().mockResolvedValue(null);
    await expect(checkForUpdate({ checker })).resolves.toEqual({ status: "current", currentVersion: "0.3.0" });
    expect(getVersionMock).toHaveBeenCalled();
  });

  it("refuses prereleases and stale updater payloads", async () => {
    const prerelease = vi.fn().mockResolvedValue({ currentVersion: "0.3.0", version: "0.4.0-beta.1" });
    await expect(checkForUpdate({ checker: prerelease })).rejects.toThrow("prerelease");
    const stale = vi.fn().mockResolvedValue({ currentVersion: "0.4.0", version: "0.3.0" });
    await expect(checkForUpdate({ checker: stale })).resolves.toEqual({ status: "current", currentVersion: "0.4.0" });
  });
});

describe("native update commands", () => {
  it("reads the runtime support boundary from Rust", async () => {
    const invoker = vi.fn().mockResolvedValue({ selfUpdateSupported: false, portable: true });
    await expect(getUpdateRuntime(invoker)).resolves.toEqual({ selfUpdateSupported: false, portable: true });
    expect(invoker).toHaveBeenCalledWith("get_update_runtime");
  });

  it("forwards progress and always removes the event listener", async () => {
    const unlisten = vi.fn();
    const progress = vi.fn();
    const listener = vi.fn().mockImplementation(async (_event, handler) => {
      handler({ payload: { stage: "downloading", downloaded: 40, total: 100 } });
      return unlisten;
    });
    const invoker = vi.fn().mockResolvedValue(true);
    await expect(installAvailableUpdate({ listener, invoker, onProgress: progress })).resolves.toBe(true);
    expect(progress).toHaveBeenCalledWith({ stage: "downloading", downloaded: 40, total: 100 });
    expect(invoker).toHaveBeenCalledWith("install_update_and_restart");
    expect(unlisten).toHaveBeenCalled();
  });
});
