export interface AboutInfo {
  version: string;
  platform: string;
  arch: string;
  appDataDir?: string;
  executableDir?: string;
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
