/**
 * Release and updater tooling must use the same stable three-part version
 * contract as the desktop client. Returning the normalized value keeps the
 * callers from accidentally building asset names from a rejected spelling.
 */
export function stableVersion(value) {
  if (typeof value !== "string") return undefined;
  const normalized = value.trim().replace(/^v/iu, "");
  if (normalized.length > 128) return undefined;
  const parts = normalized.split(".");
  if (parts.length !== 3 || !parts.every((part) => /^(?:0|[1-9]\d*)$/u.test(part) && Number.isSafeInteger(Number(part)))) {
    return undefined;
  }
  return normalized;
}
