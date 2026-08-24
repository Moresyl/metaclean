/**
 * Accelerator rendering.
 *
 * A shortcut printed as "Ctrl+K" on a Mac is the kind of detail that reads as
 * carelessness, so the modifier is spelled the way the platform spells it.
 */
export function isApple(): boolean {
  return /mac|iphone|ipad|ipod/i.test(navigator.platform || navigator.userAgent);
}

/** The prefix a shortcut label starts with — "⌘" carries its own separator. */
export function commandKeyLabel(): string {
  return isApple() ? "⌘" : "Ctrl+";
}

/** True when an event carries the platform's primary accelerator modifier. */
export function hasCommandModifier(event: Pick<KeyboardEvent, "ctrlKey" | "metaKey">): boolean {
  return isApple() ? event.metaKey : event.ctrlKey;
}
