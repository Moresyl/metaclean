type ZoomKeyEvent = Pick<KeyboardEvent, "altKey" | "ctrlKey" | "key" | "metaKey">;

const ZOOM_KEYS = new Set(["+", "-", "=", "0", "add", "subtract"]);

export function isZoomShortcut(event: ZoomKeyEvent): boolean {
  if (event.altKey || !(event.ctrlKey || event.metaKey)) return false;
  return ZOOM_KEYS.has(event.key.toLowerCase());
}

export function installZoomLock(target: Window = window): () => void {
  const preventKeyboardZoom = (event: KeyboardEvent) => {
    if (isZoomShortcut(event)) event.preventDefault();
  };
  const preventWheelZoom = (event: WheelEvent) => {
    if (event.ctrlKey || event.metaKey) event.preventDefault();
  };

  target.addEventListener("keydown", preventKeyboardZoom, { capture: true });
  target.addEventListener("wheel", preventWheelZoom, { capture: true, passive: false });
  return () => {
    target.removeEventListener("keydown", preventKeyboardZoom, true);
    target.removeEventListener("wheel", preventWheelZoom, true);
  };
}

/**
 * The window commands the title bar draws buttons for.
 *
 * Imported on demand and allowed to fail, so the same interface still renders
 * under a plain browser — which is where the tests run it.
 */
async function command(action: "minimize" | "close" | "startDragging"): Promise<void> {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow()[action]();
  } catch {
    /* No desktop window to command. */
  }
}

export const minimizeWindow = () => command("minimize");
export const closeWindow = () => command("close");
export const startWindowDragging = () => command("startDragging");

/** Interactive caption controls must never become accidental drag handles. */
export function isWindowDragTarget(target: EventTarget | null): boolean {
  return target instanceof Element
    && !target.closest("button, a, input, select, textarea, [role='button'], [data-no-drag]");
}

/**
 * Puts text on the clipboard.
 *
 * The async clipboard needs a secure context and a permission the webview does
 * not always grant, so a detached textarea and the legacy command stand behind
 * it. One of the two always works, and copying a file path is not worth a
 * dependency.
 */
export async function copyText(value: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(value);
    return true;
  } catch {
    /* Fall through to the selection-based copy. */
  }
  const carrier = document.createElement("textarea");
  carrier.value = value;
  carrier.setAttribute("readonly", "");
  carrier.className = "sr-only";
  document.body.append(carrier);
  try {
    carrier.select();
    return document.execCommand("copy");
  } catch {
    return false;
  } finally {
    carrier.remove();
  }
}
