import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { initializeTheme, ThemeProvider, useTheme } from "./ThemeContext";

let darkPreference = true;
let mediaListener: (() => void) | undefined;

beforeEach(() => {
  localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.style.colorScheme = "";
  darkPreference = true;
  mediaListener = undefined;
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: vi.fn(() => ({
      get matches() { return darkPreference; },
      media: "(prefers-color-scheme: dark)",
      onchange: null,
      addEventListener: (_event: string, listener: () => void) => { mediaListener = listener; },
      removeEventListener: (_event: string, listener: () => void) => {
        if (mediaListener === listener) mediaListener = undefined;
      },
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
});

function ThemeConsumer() {
  const theme = useTheme();
  return (
    <>
      <span data-testid="theme-mode">{theme.mode}</span>
      <button type="button" onClick={() => theme.setMode("light")}>light</button>
      <button type="button" onClick={() => theme.setMode("system")}>system</button>
    </>
  );
}

describe("ThemeProvider", () => {
  it("applies the stored theme before React renders", () => {
    localStorage.setItem("metaclean.theme", "dark");
    expect(initializeTheme()).toBe("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(document.documentElement.style.colorScheme).toBe("dark");
  });

  it("tracks system changes and persists explicit choices", async () => {
    render(<ThemeProvider initialMode="system"><ThemeConsumer /></ThemeProvider>);
    expect(document.documentElement.dataset.theme).toBe("dark");
    darkPreference = false;
    mediaListener?.();
    expect(document.documentElement.dataset.theme).toBe("light");

    fireEvent.click(screen.getByRole("button", { name: "light" }));
    await waitFor(() => expect(screen.getByTestId("theme-mode")).toHaveTextContent("light"));
    expect(localStorage.getItem("metaclean.theme")).toBe("light");
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("rejects theme usage outside its provider", () => {
    function InvalidConsumer() { useTheme(); return null; }
    expect(() => render(<InvalidConsumer />)).toThrow("useTheme must be used inside ThemeProvider");
  });
});
