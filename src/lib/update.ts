const RELEASES_API_URL = "https://api.github.com/repos/Moresyl/metaclean/releases/latest";
export const RELEASES_PAGE_URL = "https://github.com/Moresyl/metaclean/releases/latest";

type FetchLike = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

interface GitHubRelease {
  tag_name?: unknown;
  name?: unknown;
  body?: unknown;
  published_at?: unknown;
  html_url?: unknown;
  draft?: unknown;
  prerelease?: unknown;
}

export interface UpdateInfo {
  currentVersion: string;
  availableVersion: string;
  name: string;
  notes?: string;
  publishedAt?: string;
  releaseUrl: string;
}

export type UpdateCheckResult =
  | { status: "current"; currentVersion: string }
  | { status: "available"; info: UpdateInfo };

interface ParsedVersion {
  core: number[];
  prerelease: string[];
}

function parseVersion(value: string): ParsedVersion {
  const normalized = value.trim().replace(/^v/i, "").split("+", 1)[0];
  const [coreValue, prereleaseValue = ""] = normalized.split("-", 2);
  if (!/^\d+(?:\.\d+)*$/.test(coreValue)) throw new Error(`Invalid version: ${value}`);
  return {
    core: coreValue.split(".").map(Number),
    prerelease: prereleaseValue ? prereleaseValue.split(".") : [],
  };
}

export function compareVersions(left: string, right: string): number {
  const a = parseVersion(left);
  const b = parseVersion(right);
  const coreLength = Math.max(a.core.length, b.core.length);
  for (let index = 0; index < coreLength; index += 1) {
    const difference = (a.core[index] ?? 0) - (b.core[index] ?? 0);
    if (difference !== 0) return Math.sign(difference);
  }
  if (!a.prerelease.length && !b.prerelease.length) return 0;
  if (!a.prerelease.length) return 1;
  if (!b.prerelease.length) return -1;
  const prereleaseLength = Math.max(a.prerelease.length, b.prerelease.length);
  for (let index = 0; index < prereleaseLength; index += 1) {
    const aPart = a.prerelease[index];
    const bPart = b.prerelease[index];
    if (aPart === undefined) return -1;
    if (bPart === undefined) return 1;
    if (aPart === bPart) continue;
    const aNumeric = /^\d+$/.test(aPart);
    const bNumeric = /^\d+$/.test(bPart);
    if (aNumeric && bNumeric) return Math.sign(Number(aPart) - Number(bPart));
    if (aNumeric) return -1;
    if (bNumeric) return 1;
    return aPart.localeCompare(bPart) < 0 ? -1 : 1;
  }
  return 0;
}

function validatedReleaseUrl(value: unknown): string {
  if (typeof value !== "string") return RELEASES_PAGE_URL;
  try {
    const url = new URL(value);
    return url.origin === "https://github.com" && url.pathname.startsWith("/Moresyl/metaclean/releases/")
      ? url.toString()
      : RELEASES_PAGE_URL;
  } catch {
    return RELEASES_PAGE_URL;
  }
}

async function installedVersion(): Promise<string> {
  try {
    const { getVersion } = await import("@tauri-apps/api/app");
    return await getVersion();
  } catch {
    return "0.0.0";
  }
}

export async function checkForUpdate(options: {
  currentVersion?: string;
  fetcher?: FetchLike;
  timeoutMs?: number;
} = {}): Promise<UpdateCheckResult> {
  const currentVersion = options.currentVersion ?? await installedVersion();
  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), options.timeoutMs ?? 10_000);
  try {
    const response = await (options.fetcher ?? fetch)(RELEASES_API_URL, {
      headers: {
        Accept: "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
      },
      signal: controller.signal,
    });
    if (!response.ok) throw new Error(`GitHub update service returned HTTP ${response.status}`);
    const release = await response.json() as GitHubRelease;
    if (release.draft || release.prerelease || typeof release.tag_name !== "string") {
      throw new Error("GitHub update service returned an invalid stable release");
    }
    const availableVersion = release.tag_name.replace(/^v/i, "");
    if (compareVersions(availableVersion, currentVersion) <= 0) {
      return { status: "current", currentVersion };
    }
    return {
      status: "available",
      info: {
        currentVersion,
        availableVersion,
        name: typeof release.name === "string" && release.name.trim() ? release.name : `MetaClean v${availableVersion}`,
        notes: typeof release.body === "string" ? release.body : undefined,
        publishedAt: typeof release.published_at === "string" ? release.published_at : undefined,
        releaseUrl: validatedReleaseUrl(release.html_url),
      },
    };
  } finally {
    window.clearTimeout(timeout);
  }
}
