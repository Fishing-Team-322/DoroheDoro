"use client";

import type { ReactNode } from "react";
import { useEffect } from "react";
import { usePathname, useRouter } from "next/navigation";
import type { Locale } from "@/src/shared/config";
import { buildLoginPath } from "@/src/shared/lib/auth";
import { useI18n } from "@/src/shared/lib/i18n";
import { Spinner } from "@/src/shared/ui";
import { DashboardLayout } from "@/src/widgets/dashboard-layout";
import { useAuth } from "../model/use-auth";

type ProtectedDashboardLayoutProps = {
  locale: Locale;
  children: ReactNode;
};

export function ProtectedDashboardLayout({
  locale,
  children,
}: ProtectedDashboardLayoutProps) {
  const router = useRouter();
  const pathname = usePathname();
  const { dictionary } = useI18n();
  const { status } = useAuth();

  useEffect(() => {
    if (status === "unauthenticated") {
      router.replace(buildLoginPath(locale, pathname));
    }
  }, [locale, pathname, router, status]);

  if (status !== "authenticated") {
    return (
      <div className="flex min-h-screen items-center justify-center bg-[color:var(--background)] px-6">
        <div className="inline-flex items-center gap-3 rounded-full border border-[color:var(--border)] bg-[color:var(--surface-elevated)] px-5 py-3 text-sm text-[color:var(--muted-foreground)]">
          <Spinner size="sm" />
          {status === "loading"
            ? dictionary.auth.guard.checkingSession
            : dictionary.auth.guard.redirectingToLogin}
        </div>
      </div>
    );
  }

  return <DashboardLayout locale={locale}>{children}</DashboardLayout>;
}
