import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { htmlLanguage, initialLocale, textDirection, translate, type Locale } from "./locales";
import { readStorage, writeStorage } from "./storage";

export type { Locale } from "./locales";

interface I18nValue {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  text: (zh: string, en: string) => string;
}

const I18nContext = createContext<I18nValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(() => {
    return initialLocale(readStorage("metaclean.locale") ?? null, navigator.language);
  });
  useEffect(() => {
    document.documentElement.lang = htmlLanguage(locale);
    document.documentElement.dir = textDirection(locale);
  }, [locale]);
  const value = useMemo<I18nValue>(() => ({
    locale,
    setLocale: (next) => { writeStorage("metaclean.locale", next); setLocaleState(next); },
    text: (zh, en) => translate(locale, zh, en),
  }), [locale]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nValue {
  const value = useContext(I18nContext);
  if (!value) throw new Error("useI18n must be used inside I18nProvider");
  return value;
}
