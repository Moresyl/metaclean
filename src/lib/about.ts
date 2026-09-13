import { MAX_LABEL_BYTES, MAX_PATH_BYTES, isBoundedText } from "./bounds";

export interface AboutInfo {
  version: string;
  platform: string;
  arch: string;
  appDataDir?: string;
  executableDir?: string;
}

/** Validate runtime facts crossing the native/UI boundary. */
export function normalizeAboutInfo(value: unknown): AboutInfo | undefined {
  if (!value || typeof value !== "object") return undefined;
  const candidate = value as Partial<AboutInfo>;
  if (!isBoundedText(candidate.version, MAX_LABEL_BYTES)
    || !isBoundedText(candidate.platform, MAX_LABEL_BYTES)
    || !isBoundedText(candidate.arch, MAX_LABEL_BYTES)) return undefined;
  const optionalPath = (path: unknown) => path === undefined || path === null || isBoundedText(path, MAX_PATH_BYTES);
  if (!optionalPath(candidate.appDataDir) || !optionalPath(candidate.executableDir)) return undefined;
  return {
    version: candidate.version,
    platform: candidate.platform,
    arch: candidate.arch,
    appDataDir: typeof candidate.appDataDir === "string" ? candidate.appDataDir : undefined,
    executableDir: typeof candidate.executableDir === "string" ? candidate.executableDir : undefined,
  };
}

export function buildDiagnosticReport(
  about: AboutInfo,
  options: {
    locale: string;
    updateStatus: string;
    availableVersion?: string;
    portable: boolean;
    selfUpdateSupported: boolean;
  },
): string {
  return JSON.stringify({
    schemaVersion: 1,
    product: "MetaClean",
    version: about.version,
    platform: about.platform,
    arch: about.arch,
    locale: options.locale,
    runtime: {
      portable: options.portable,
      selfUpdateSupported: options.selfUpdateSupported,
    },
    update: {
      status: options.updateStatus,
      availableVersion: options.availableVersion,
    },
    paths: {
      appDataDirectory: about.appDataDir,
      executableDirectory: about.executableDir,
    },
    generatedAt: new Date().toISOString(),
  }, null, 2);
}
