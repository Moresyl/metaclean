export const RELEASES_PAGE_URL = "https://github.com/Moresyl/metaclean/releases/latest";

interface NativeUpdate {
  version: string;
  currentVersion?: string;
  body?: string;
  date?: string;
}

type UpdateChecker = (options: { timeout: number }) => Promise<NativeUpdate | null>;
type InvokeLike = <T>(command: string) => Promise<T>;
type ListenLike = (
  event: string,
  handler: (event: { payload: UpdateProgress }) => void,
) => Promise<() => void>;

export interface UpdateInfo {
  currentVersion: string;
  availableVersion: string;
  name: string;
  notes?: string;
  publishedAt?: string;
  releaseUrl: string;
}

export interface UpdateRuntime {
  selfUpdateSupported: boolean;
  portable: boolean;
}

export interface UpdateProgress {
  stage: "downloading" | "installing";
  downloaded: number;
  total?: number;
}

export type UpdateCheckResult =
  | { status: "current"; currentVersion: string }
  | { status: "available"; info: UpdateInfo };

interface ParsedVersion {
  core: number[];
  prerelease: string[];
}

function parseVersion(value: string): ParsedVersion {
  const normalized = value.trim().replace(/^v/iu, "").split("+", 1)[0];
  const [coreValue, prereleaseValue = ""] = normalized.split("-", 2);
  if (!/^\d+(?:\.\d+)*$/u.test(coreValue)) throw new Error(`Invalid version: ${value}`);
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
    const aNumeric = /^\d+$/u.test(aPart);
    const bNumeric = /^\d+$/u.test(bPart);
    if (aNumeric && bNumeric) return Math.sign(Number(aPart) - Number(bPart));
    if (aNumeric) return -1;
    if (bNumeric) return 1;
    return aPart.localeCompare(bPart) < 0 ? -1 : 1;
  }
  return 0;
}

function releaseUrlForVersion(version: string): string {
  return `https://github.com/Moresyl/metaclean/releases/tag/v${encodeURIComponent(version)}`;
}

export async function getInstalledVersion(): Promise<string> {
  try {
    const { getVersion } = await import("@tauri-apps/api/app");
    return await getVersion();
  } catch {
    return "0.0.0";
  }
}

async function nativeCheck(options: { timeout: number }): Promise<NativeUpdate | null> {
  const { check } = await import("@tauri-apps/plugin-updater");
  return await check(options);
}

export async function checkForUpdate(options: {
  currentVersion?: string;
  checker?: UpdateChecker;
  timeoutMs?: number;
} = {}): Promise<UpdateCheckResult> {
  const update = await (options.checker ?? nativeCheck)({ timeout: options.timeoutMs ?? 10_000 });
  const currentVersion = options.currentVersion ?? update?.currentVersion ?? await getInstalledVersion();
  if (!update) return { status: "current", currentVersion };

  const availableVersion = update.version.replace(/^v/iu, "");
  const parsed = parseVersion(availableVersion);
  if (parsed.prerelease.length) throw new Error("Update service returned a prerelease version");
  if (compareVersions(availableVersion, currentVersion) <= 0) {
    return { status: "current", currentVersion };
  }
  return {
    status: "available",
    info: {
      currentVersion,
      availableVersion,
      name: `MetaClean v${availableVersion}`,
      notes: typeof update.body === "string" && update.body.trim() ? update.body : undefined,
      publishedAt: typeof update.date === "string" && update.date.trim() ? update.date : undefined,
      releaseUrl: releaseUrlForVersion(availableVersion),
    },
  };
}

export async function getUpdateRuntime(invoker?: InvokeLike): Promise<UpdateRuntime> {
  const invoke = invoker ?? (async <T,>(command: string) => {
    const core = await import("@tauri-apps/api/core");
    return await core.invoke<T>(command);
  });
  return await invoke<UpdateRuntime>("get_update_runtime");
}

export async function installAvailableUpdate(options: {
  onProgress?: (progress: UpdateProgress) => void;
  invoker?: InvokeLike;
  listener?: ListenLike;
} = {}): Promise<boolean> {
  const invoke = options.invoker ?? (async <T,>(command: string) => {
    const core = await import("@tauri-apps/api/core");
    return await core.invoke<T>(command);
  });
  const listen = options.listener ?? (async (event, handler) => {
    const events = await import("@tauri-apps/api/event");
    return await events.listen<UpdateProgress>(event, handler);
  });
  const unlisten = await listen("update-progress", (event) => options.onProgress?.(event.payload));
  try {
    return await invoke<boolean>("install_update_and_restart");
  } finally {
    unlisten();
  }
}
