import type { ReactNode } from "react";
import { notFound } from "next/navigation";
import { AuthProvider } from "@/src/features/auth";
import { isLocale } from "@/src/shared/config";

type LocaleLayoutProps = {
  children: ReactNode;
  params: Promise<{ locale: string }>;
};

export default async function LocaleLayout({
  children,
  params,
}: LocaleLayoutProps) {
  const { locale } = await params;

  if (!isLocale(locale)) {
    notFound();
  }

  return <AuthProvider>{children}</AuthProvider>;
}
