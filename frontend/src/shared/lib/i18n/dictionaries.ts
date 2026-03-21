import type { Locale } from "@/src/shared/config";
import en from "@/src/shared/i18n/locales/en/common.json";
import ru from "@/src/shared/i18n/locales/ru/common.json";

const dictionaries = {
  en,
  ru,
} as const;

export type Dictionary = (typeof dictionaries)[Locale];

export function getDictionary(locale: Locale): Dictionary {
  return dictionaries[locale];
}
