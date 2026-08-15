import { createContext, useContext, useMemo, useState, type ReactNode } from "react";

export type Locale = "zh" | "en";

interface I18nValue {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  text: (zh: string, en: string) => string;
}

const I18nContext = createContext<I18nValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(() => localStorage.getItem("metaclean.locale") === "en" ? "en" : "zh");
  const value = useMemo<I18nValue>(() => ({
    locale,
    setLocale: (next) => { localStorage.setItem("metaclean.locale", next); setLocaleState(next); },
    text: (zh, en) => locale === "zh" ? zh : en,
  }), [locale]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nValue {
  const value = useContext(I18nContext);
  if (!value) throw new Error("useI18n must be used inside I18nProvider");
  return value;
}
