import type { ReactNode } from "react";
import { ProtectedDashboardLayout } from "@/src/features/auth";
import { isLocale } from "@/src/shared/config";

export default async function DashboardRouteGroupLayout({
  children,
  params,
}: {
  children: ReactNode;
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;

  if (!isLocale(locale)) {
    return children;
  }

  return (
    <ProtectedDashboardLayout locale={locale}>
      {children}
    </ProtectedDashboardLayout>
  );
}
