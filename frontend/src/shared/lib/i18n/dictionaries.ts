import type { Locale } from "@/src/shared/config";
import en from "@/src/shared/i18n/locales/en/common";
import ru from "@/src/shared/i18n/locales/ru/common";

const dictionaries = {
  en,
  ru,
} as const;

export type Dictionary = (typeof dictionaries)[Locale];

export function getDictionary(locale: Locale): Dictionary {
  return dictionaries[locale];
}
