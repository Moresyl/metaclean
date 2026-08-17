import { describe, expect, it } from "vitest";
import { SOURCE_STRINGS, hasStaticTranslation, htmlLanguage, initialLocale, isLocale, translate, type Locale } from "./locales";

describe("locales", () => {
  it("accepts only published locale codes", () => {
    expect(isLocale("zh-TW")).toBe(true);
    expect(isLocale("ko")).toBe(true);
    expect(isLocale("unknown")).toBe(false);
  });

  it("prefers a saved locale and otherwise detects the browser language", () => {
    expect(initialLocale("ja", "de-DE")).toBe("ja");
    expect(initialLocale(null, "zh-Hant-HK")).toBe("zh-TW");
    expect(initialLocale(null, "ko-KR")).toBe("ko");
    expect(initialLocale(null, "pt-BR")).toBe("pt-BR");
    expect(initialLocale(null, "xx-YY")).toBe("en");
  });

  it("has a complete translation for every published non-source catalog", () => {
    for (const locale of ["zh-TW", "cs", "da", "de", "es", "fr", "id", "it", "ja", "ko", "ms", "nb", "nl", "pl", "pt", "pt-BR", "ro", "ru", "sk", "sv", "tr", "uk"] satisfies Locale[]) {
      for (const source of SOURCE_STRINGS) {
        expect(hasStaticTranslation(locale, source), `${locale}: ${source}`).toBe(true);
      }
    }
  });

  it("uses native source strings and correct HTML language tags", () => {
    expect(translate("zh", "设置", "Settings")).toBe("设置");
    expect(translate("en", "设置", "Settings")).toBe("Settings");
    expect(htmlLanguage("zh-TW")).toBe("zh-TW");
    expect(htmlLanguage("invalid" as Locale)).toBe("en");
    expect(hasStaticTranslation("zh", "Settings")).toBe(true);
    expect(hasStaticTranslation("en", "Settings")).toBe(true);
    expect(hasStaticTranslation("fr", "missing")).toBe(false);
    expect(translate("fr", "", "missing")).toBe("missing");
  });

  it("localizes dynamic values without altering user or version data", () => {
    expect(translate("ja", "", "3 trace(s) found")).toBe("3件の痕跡が見つかりました");
    expect(translate("ko", "", "Remove report.pdf")).toBe("report.pdf 제거");
    expect(translate("zh-TW", "", "Version 0.2.0 is installed; 0.3.0 is available.")).toBe("目前為 0.2.0，可更新至 0.3.0。");
  });

  it("covers every dynamic message in every translated locale", () => {
    const messages = [
      "Added 2 file(s); skipped 1. unreadable: sample.bin",
      "Could not expand the selected paths: denied",
      "Scan complete: 3 trace(s) await confirmation.",
      "Scan failed: unavailable",
      "2 file(s) cleaned; 1 failed. Output: result.pdf",
      "2 file(s) cleaned.",
      "Cleanup failed: denied",
      "Version 0.3.0 is available",
      "3 trace(s) found",
      "Remove report.pdf",
      "Version 0.2.0 is installed; 0.3.0 is available.",
      "Version 0.2.0 is up to date.",
      "Update check failed: offline",
    ];
    for (const locale of ["zh-TW", "cs", "da", "de", "es", "fr", "id", "it", "ja", "ko", "ms", "nb", "nl", "pl", "pt", "pt-BR", "ro", "ru", "sk", "sv", "tr", "uk"] satisfies Locale[]) {
      for (const message of messages) {
        expect(translate(locale, "", message), `${locale}: ${message}`).not.toBe(message);
      }
    }
  });
});
