import { normalizePathList } from "./intake";

/** Only this error means a browser fallback is appropriate. */
export class PickerUnavailableError extends Error {
  constructor(cause?: unknown) {
    super("系统文件选择器不可用 / The system file picker is unavailable", { cause });
    this.name = "PickerUnavailableError";
  }
}

/**
 * The system file picker.
 *
 * Imported on demand so the module still loads under a plain browser, where
 * there is no dialog plugin and the caller falls back to a file input.
 */
export async function pickPaths(directory: boolean): Promise<string[] | null> {
  let open: (options: { multiple: boolean; directory: boolean }) => Promise<unknown>;
  try {
    ({ open } = await import("@tauri-apps/plugin-dialog"));
  } catch (cause) {
    throw new PickerUnavailableError(cause);
  }
  let chosen: unknown;
  try {
    chosen = await open({ multiple: !directory, directory });
  } catch (cause) {
    throw new PickerUnavailableError(cause);
  }
  if (!chosen) return null;
  const paths = normalizePathList(Array.isArray(chosen) ? chosen : [chosen]);
  if (!paths) throw new Error("系统选择器返回了无效或超限路径 / The system picker returned invalid or oversized paths");
  return paths;
}
