import type { Page } from "../types";
import { MAX_DIAGNOSTIC_BYTES, isBoundedText } from "./bounds";

const PAGES: ReadonlySet<string> = new Set(["clean", "history", "privacy", "settings", "about"]);

/** Validate navigation events emitted by the native menu bridge. */
export function normalizeNavigationPage(value: unknown): Page | undefined {
  return typeof value === "string" && PAGES.has(value) ? value as Page : undefined;
}

/** Validate close-blocked diagnostics before they reach visible UI state. */
export function normalizeCloseBlocked(value: unknown): string | undefined {
  return isBoundedText(value, MAX_DIAGNOSTIC_BYTES) ? value : undefined;
}
