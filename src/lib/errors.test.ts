import { describe, expect, it } from "vitest";
import { boundedErrorMessage } from "./errors";

describe("boundedErrorMessage", () => {
  it("keeps ordinary errors readable", () => {
    expect(boundedErrorMessage(new Error("engine unavailable"))).toBe("engine unavailable");
    expect(boundedErrorMessage("native failure")).toBe("native failure");
  });

  it("caps UTF-8 bytes without splitting a multibyte character", () => {
    const value = boundedErrorMessage("界".repeat(8 * 1024));
    expect(new TextEncoder().encode(value).byteLength).toBeLessThanOrEqual(8 * 1024);
    expect(value.endsWith("…")).toBe(true);
  });

  it("falls back when an unknown rejection cannot be stringified", () => {
    const reason = { toString: () => { throw new Error("broken coercion"); } };
    expect(boundedErrorMessage(reason)).toBe("未知错误 / Unknown error");
  });

  it("falls back when an Error message has an unsafe value", () => {
    const reason = new Error();
    Object.defineProperty(reason, "message", { get: () => ({ toString: () => { throw new Error("broken message"); } }) });
    expect(boundedErrorMessage(reason)).toBe("未知错误 / Unknown error");
  });
});
