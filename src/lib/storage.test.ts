import { describe, expect, it, vi } from "vitest";
import { readStorage, removeStorage, writeStorage } from "./storage";

describe("safe local storage", () => {
  it("reads, writes, and removes values", () => {
    expect(writeStorage("test.key", "value")).toBe(true);
    expect(readStorage("test.key")).toBe("value");
    expect(removeStorage("test.key")).toBe(true);
    expect(readStorage("test.key")).toBeUndefined();
  });

  it("degrades gracefully when storage is unavailable", () => {
    const get = vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => { throw new DOMException("denied"); });
    const set = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => { throw new DOMException("quota"); });
    const remove = vi.spyOn(Storage.prototype, "removeItem").mockImplementation(() => { throw new DOMException("denied"); });

    expect(readStorage("test.key")).toBeUndefined();
    expect(writeStorage("test.key", "value")).toBe(false);
    expect(removeStorage("test.key")).toBe(false);

    get.mockRestore();
    set.mockRestore();
    remove.mockRestore();
  });
});
