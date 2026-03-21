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

export function replacePathLocale(path: string, locale: Locale): string {
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;

  try {
    const url = new URL(normalizedPath, "http://localhost");
    const segments = url.pathname.split("/").filter(Boolean);

    if (segments.length > 0 && isLocale(segments[0])) {
      segments[0] = locale;
    } else {
      segments.unshift(locale);
    }

    const pathname = `/${segments.join("/")}`;

    return `${pathname}${url.search}${url.hash}`;
  } catch {
    return withLocalePath(locale, normalizedPath);
  }
}
