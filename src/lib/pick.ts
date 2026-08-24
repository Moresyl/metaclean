/**
 * The system file picker.
 *
 * Imported on demand so the module still loads under a plain browser, where
 * there is no dialog plugin and the caller falls back to a file input.
 */
export async function pickPaths(directory: boolean): Promise<string[] | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const chosen = await open({ multiple: !directory, directory });
  if (!chosen) return null;
  return Array.isArray(chosen) ? chosen : [chosen];
}
