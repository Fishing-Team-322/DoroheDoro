import { defaultLocale, isLocale, type Locale } from "@/src/shared/config";

export function getLocaleFromParams(params: { locale?: string }): Locale {
  if (params.locale && isLocale(params.locale)) {
    return params.locale;
  }

  return defaultLocale;
}

export function withLocalePath(locale: Locale, path = "/"): string {
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;

  if (normalizedPath === "/") {
    return `/${locale}`;
  }

  return `/${locale}${normalizedPath}`;
}
