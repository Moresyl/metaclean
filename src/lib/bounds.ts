export const MAX_PATH_BYTES = 32 * 1024;
export const MAX_BATCH_PATH_BYTES = 64 * 1024 * 1024;
export const MAX_BATCH_FILES = 10_000;
export const MAX_DIAGNOSTIC_BYTES = 8 * 1024;
export const MAX_LABEL_BYTES = 256;
export const MAX_NAME_BYTES = 256;
export const UTF8_ENCODER = new TextEncoder();

export function isBoundedText(value: unknown, maxBytes: number, allowEmpty = false): value is string {
  return typeof value === "string"
    && (allowEmpty || value.length > 0)
    && value.length <= maxBytes
    && UTF8_ENCODER.encode(value).byteLength <= maxBytes;
}
