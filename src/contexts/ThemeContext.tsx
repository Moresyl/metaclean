import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { readStorage, writeStorage } from "../lib/storage";

export type ThemeMode = "system" | "light" | "dark";

interface ThemeContextValue {
  mode: ThemeMode;
  setMode: (mode: ThemeMode) => void;
}

const THEME_STORAGE_KEY = "metaclean.theme";
const DARK_MEDIA_QUERY = "(prefers-color-scheme: dark)";
const ThemeContext = createContext<ThemeContextValue | null>(null);

function storedTheme(): ThemeMode {
  const value = readStorage(THEME_STORAGE_KEY);
  return value === "light" || value === "dark" ? value : "system";
}

function systemTheme(): Exclude<ThemeMode, "system"> {
  return window.matchMedia?.(DARK_MEDIA_QUERY).matches ? "dark" : "light";
}

function applyTheme(mode: ThemeMode) {
  const resolved = mode === "system" ? systemTheme() : mode;
  document.documentElement.dataset.theme = resolved;
  document.documentElement.style.colorScheme = resolved;
}

export function initializeTheme(): ThemeMode {
  const mode = storedTheme();
  applyTheme(mode);
  return mode;
}

export function ThemeProvider({ children, initialMode }: { children: ReactNode; initialMode?: ThemeMode }) {
  const [mode, setModeState] = useState<ThemeMode>(initialMode ?? storedTheme);

  const setMode = useCallback((next: ThemeMode) => {
    writeStorage(THEME_STORAGE_KEY, next);
    setModeState(next);
  }, []);

  useEffect(() => {
    applyTheme(mode);
    if (mode !== "system" || !window.matchMedia) return;
    const media = window.matchMedia(DARK_MEDIA_QUERY);
    const handleChange = () => applyTheme("system");
    media.addEventListener("change", handleChange);
    return () => media.removeEventListener("change", handleChange);
  }, [mode]);

  const value = useMemo<ThemeContextValue>(() => ({ mode, setMode }), [mode, setMode]);
  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeContextValue {
  const value = useContext(ThemeContext);
  if (!value) throw new Error("useTheme must be used inside ThemeProvider");
  return value;
}
