export const REPOSITORY_URL = "https://github.com/Moresyl/metaclean";
export const ISSUES_URL = `${REPOSITORY_URL}/issues`;
export const BUG_REPORT_URL = `${ISSUES_URL}/new?labels=bug`;
export const FEATURE_REQUEST_URL = `${ISSUES_URL}/new?labels=enhancement`;
export const RELEASES_URL = `${REPOSITORY_URL}/releases`;
export const LICENSE_URL = `${REPOSITORY_URL}/blob/master/LICENSE`;

export type ProjectUrl =
  | typeof REPOSITORY_URL
  | typeof ISSUES_URL
  | typeof BUG_REPORT_URL
  | typeof FEATURE_REQUEST_URL
  | typeof RELEASES_URL
  | typeof LICENSE_URL;

export async function openProjectUrl(url: ProjectUrl): Promise<void> {
  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(url);
}
