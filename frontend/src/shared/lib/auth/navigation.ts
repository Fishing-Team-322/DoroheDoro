import type { Locale } from "@/src/shared/config";
import { withLocalePath } from "@/src/shared/lib/i18n";

export function getDefaultAuthenticatedPath(locale: Locale): string {
  return withLocalePath(locale, "/overview");
}

export function buildLoginPath(locale: Locale, nextPath?: string | null): string {
  const loginPath = withLocalePath(locale, "/login");
  const safeNextPath = normalizeRedirectPath(nextPath, locale);

  if (!safeNextPath) {
    return loginPath;
  }

  return `${loginPath}?next=${encodeURIComponent(safeNextPath)}`;
}

export function normalizeRedirectPath(
  value: string | null | undefined,
  locale: Locale
): string | null {
  if (!value || !value.startsWith("/") || value.startsWith("//")) {
    return null;
  }

  try {
    const url = new URL(value, "http://localhost");
    const normalized = `${url.pathname}${url.search}${url.hash}`;

    if (normalized === `/${locale}` || normalized.startsWith(`/${locale}/`)) {
      return normalized;
    }

    return null;
  } catch {
    return null;
  }
}
