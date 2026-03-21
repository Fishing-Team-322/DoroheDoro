"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import type { Locale } from "@/src/shared/config";
import { defaultLocale } from "@/src/shared/config";
import { getDefaultAuthenticatedPath } from "@/src/shared/lib/auth";
import { useI18n } from "@/src/shared/lib/i18n";
import { Spinner } from "@/src/shared/ui";
import { useAuth } from "../model/use-auth";

export function LocaleEntryRedirect({ locale }: { locale: Locale }) {
  const router = useRouter();
  const { dictionary } = useI18n();
  const { status } = useAuth();

  useEffect(() => {
    if (status === "authenticated") {
      router.replace(getDefaultAuthenticatedPath(locale));
      return;
    }

    if (status === "unauthenticated") {
      router.replace(`/${defaultLocale}/login`);
    }
  }, [locale, router, status]);

  const message =
    status === "authenticated"
      ? dictionary.auth.entry.redirectingAuthenticated
      : status === "unauthenticated"
        ? dictionary.auth.entry.redirectingUnauthenticated
        : dictionary.auth.entry.checkingSession;

  return (
    <main className="flex min-h-screen items-center justify-center bg-[color:var(--background)] px-6">
      <div className="inline-flex items-center gap-3 rounded-full border border-[color:var(--border)] bg-[color:var(--surface-elevated)] px-5 py-3 text-sm text-[color:var(--muted-foreground)]">
        <Spinner size="sm" />
        {message}
      </div>
    </main>
  );
}
