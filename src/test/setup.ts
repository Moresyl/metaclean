import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach } from "vitest";

beforeEach(() => {
  localStorage.setItem("metaclean.locale", "zh");
});

afterEach(() => {
  cleanup();
  localStorage.clear();
});
