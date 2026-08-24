import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach } from "vitest";

// jsdom performs no layout and so implements no scrolling. Anything that keeps
// a highlighted row in view calls this, and a missing method is a crash.
Element.prototype.scrollIntoView ??= () => {};

beforeEach(() => {
  localStorage.setItem("metaclean.locale", "zh");
});

afterEach(() => {
  cleanup();
  localStorage.clear();
});
