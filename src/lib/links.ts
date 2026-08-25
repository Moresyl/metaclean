export const REPOSITORY_URL = "https://github.com/Moresyl/metaclean";
export const ISSUES_URL = `${REPOSITORY_URL}/issues`;

export async function openProjectUrl(url: typeof REPOSITORY_URL | typeof ISSUES_URL): Promise<void> {
  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(url);
}
