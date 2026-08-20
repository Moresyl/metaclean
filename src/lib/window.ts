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
