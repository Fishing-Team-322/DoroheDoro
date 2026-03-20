"use client";

import { useMemo } from "react";
import type { Locale } from "@/src/shared/config";
import { withLocalePath } from "./routing";

export function useLocaleHref(locale: Locale, path = "/") {
  return useMemo(() => withLocalePath(locale, path), [locale, path]);
}
