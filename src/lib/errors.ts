const MAX_ERROR_MESSAGE_BYTES = 8 * 1024;
const TRUNCATION_MARKER = "…";
const UTF8_ENCODER = new TextEncoder();

function utf8ByteLength(value: string): number {
  return UTF8_ENCODER.encode(value).byteLength;
}

/**
 * Convert an unknown rejection into a bounded, user-visible diagnostic.
 * Browser APIs and plugin adapters are outside the native error boundary, so
 * their messages need the same UTF-8 byte budget before entering React state.
 */
export function boundedErrorMessage(reason: unknown): string {
  let value: string;
  try {
    const raw = reason instanceof Error ? reason.message : reason;
    value = typeof raw === "string" ? raw : String(raw);
  } catch {
    value = "未知错误 / Unknown error";
  }
  if (utf8ByteLength(value) <= MAX_ERROR_MESSAGE_BYTES) return value;

  const budget = MAX_ERROR_MESSAGE_BYTES - utf8ByteLength(TRUNCATION_MARKER);
  let bytes = 0;
  let output = "";
  for (const character of value) {
    const next = utf8ByteLength(character);
    if (bytes + next > budget) break;
    output += character;
    bytes += next;
  }
  return output + TRUNCATION_MARKER;
}
