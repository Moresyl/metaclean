import { beforeEach, describe, expect, it, vi } from "vitest";
import { checkForUpdate, compareVersions, RELEASES_PAGE_URL } from "./update";

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
  beforeEach(() => getVersionMock.mockResolvedValue("0.1.0"));
  it("returns a validated newer stable release", async () => {
    const fetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify({
      tag_name: "v0.2.0",
      name: "MetaClean v0.2.0",
      body: "Safer cleaning",
      published_at: "2026-08-17T00:00:00Z",
      html_url: "https://github.com/Moresyl/metaclean/releases/tag/v0.2.0",
      draft: false,
      prerelease: false,
    }), { status: 200 }));
    await expect(checkForUpdate({ currentVersion: "0.1.0", fetcher })).resolves.toEqual({
      status: "available",
      info: expect.objectContaining({ availableVersion: "0.2.0", releaseUrl: "https://github.com/Moresyl/metaclean/releases/tag/v0.2.0" }),
    });
  });

  it("reports current versions and refuses untrusted release links", async () => {
    const currentFetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify({ tag_name: "v0.1.0", draft: false, prerelease: false }), { status: 200 }));
    await expect(checkForUpdate({ currentVersion: "0.1.0", fetcher: currentFetcher })).resolves.toEqual({ status: "current", currentVersion: "0.1.0" });

    const untrustedFetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify({ tag_name: "v0.2.0", html_url: "https://evil.example/update", draft: false, prerelease: false }), { status: 200 }));
    const result = await checkForUpdate({ currentVersion: "0.1.0", fetcher: untrustedFetcher });
    expect(result.status === "available" && result.info.releaseUrl).toBe(RELEASES_PAGE_URL);
  });

  it("rejects API failures and non-stable payloads", async () => {
    const failed = vi.fn().mockResolvedValue(new Response("rate limited", { status: 403 }));
    await expect(checkForUpdate({ currentVersion: "0.1.0", fetcher: failed })).rejects.toThrow("HTTP 403");
    const prerelease = vi.fn().mockResolvedValue(new Response(JSON.stringify({ tag_name: "v0.2.0-beta.1", prerelease: true }), { status: 200 }));
    await expect(checkForUpdate({ currentVersion: "0.1.0", fetcher: prerelease })).rejects.toThrow("invalid stable release");
  });

  it("uses the installed app version and safe fallback release fields", async () => {
    const fetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify({
      tag_name: "v0.2.0",
      name: "",
      body: 123,
      published_at: null,
      html_url: "not a URL",
      draft: false,
      prerelease: false,
    }), { status: 200 }));
    const result = await checkForUpdate({ fetcher, timeoutMs: 100 });
    expect(getVersionMock).toHaveBeenCalled();
    expect(result).toEqual({
      status: "available",
      info: {
        currentVersion: "0.1.0",
        availableVersion: "0.2.0",
        name: "MetaClean v0.2.0",
        notes: undefined,
        publishedAt: undefined,
        releaseUrl: RELEASES_PAGE_URL,
      },
    });
  });
});
