import { getSiteCopy } from "@/src/shared/lib/i18n/site-copy";

const MINUTE_MS = 60 * 1000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;
const WEEK_MS = 7 * DAY_MS;

export function formatRelativeLabel(
  value: string | Date | null | undefined,
  locale: string
): string {
  const resolvedLocale = locale === "ru" ? "ru" : "en";

  if (!value) {
    return getSiteCopy(resolvedLocale).common.na;
  }

  const date = value instanceof Date ? value : new Date(value);
  if (Number.isNaN(date.getTime())) {
    return typeof value === "string" ? value : getSiteCopy(resolvedLocale).common.na;
  }

  const deltaMs = date.getTime() - Date.now();
  const absoluteDeltaMs = Math.abs(deltaMs);
  const formatter = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });

  if (absoluteDeltaMs < HOUR_MS) {
    return formatter.format(Math.round(deltaMs / MINUTE_MS), "minute");
  }

  if (absoluteDeltaMs < DAY_MS) {
    return formatter.format(Math.round(deltaMs / HOUR_MS), "hour");
  }

  if (absoluteDeltaMs < WEEK_MS) {
    return formatter.format(Math.round(deltaMs / DAY_MS), "day");
  }

  return date.toLocaleString(locale);
}
