import { beforeEach, describe, expect, it, vi } from "vitest";
import { checkForUpdate, compareVersions, getInstalledVersion, getUpdateRuntime, installAvailableUpdate } from "./update";

const getVersionMock = vi.hoisted(() => vi.fn());
const invokeMock = vi.hoisted(() => vi.fn());
const listenMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/app", () => ({ getVersion: getVersionMock }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));

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
    const close = vi.fn().mockResolvedValue(undefined);
    const checker = vi.fn().mockResolvedValue({
      currentVersion: "0.3.0",
      version: "0.4.0",
      body: "Signed update",
      date: "2026-08-18T00:00:00Z",
      close,
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
    expect(close).toHaveBeenCalledOnce();
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

  it("closes native update resources on invalid metadata and explains network failures", async () => {
    const close = vi.fn().mockResolvedValue(undefined);
    const malformed = vi.fn().mockResolvedValue({ currentVersion: "0.3.0", version: "latest", close });
    await expect(checkForUpdate({ checker: malformed })).rejects.toThrow("Invalid version");
    expect(close).toHaveBeenCalledOnce();

    const offline = vi.fn().mockRejectedValue(new Error("error sending request for url"));
    await expect(checkForUpdate({ checker: offline })).rejects.toThrow(/无法连接已签名更新源.*HTTPS_PROXY.*error sending request for url/su);
    expect(offline).toHaveBeenCalledWith({ timeout: 15_000 });
  });
});

describe("getInstalledVersion", () => {
  it("reads the local application version without an update request", async () => {
    getVersionMock.mockResolvedValue("0.4.1");
    await expect(getInstalledVersion()).resolves.toBe("0.4.1");
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
    await expect(installAvailableUpdate({ expectedVersion: "0.4.0", listener, invoker, onProgress: progress })).resolves.toBe(true);
    expect(progress).toHaveBeenCalledWith({ stage: "downloading", downloaded: 40, total: 100 });
    expect(invoker).toHaveBeenCalledWith("install_update_and_restart", { expectedVersion: "0.4.0" });
    expect(unlisten).toHaveBeenCalled();
  });

  it("forwards the reviewed version through the default Tauri adapter", async () => {
    const unlisten = vi.fn();
    invokeMock.mockResolvedValue(true);
    listenMock.mockResolvedValue(unlisten);

    await expect(installAvailableUpdate({ expectedVersion: "v0.7.1" })).resolves.toBe(true);

    expect(invokeMock).toHaveBeenCalledWith("install_update_and_restart", { expectedVersion: "0.7.1" });
    expect(listenMock).toHaveBeenCalledWith("update-progress", expect.any(Function));
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("refuses an unreviewed install version before invoking Rust", async () => {
    const unlisten = vi.fn();
    const listener = vi.fn().mockResolvedValue(unlisten);
    const invoker = vi.fn();
    await expect(installAvailableUpdate({ expectedVersion: "latest", listener, invoker })).rejects.toThrow(/发生了变化/u);
    expect(invoker).not.toHaveBeenCalled();
    expect(unlisten).toHaveBeenCalledOnce();
  });
});
