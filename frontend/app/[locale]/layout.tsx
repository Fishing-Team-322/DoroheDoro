import type { ReactNode } from "react";
import { notFound } from "next/navigation";
import { AuthProvider } from "@/src/features/auth";
import { isLocale, locales } from "@/src/shared/config";
import { getDictionary, I18nProvider } from "@/src/shared/lib/i18n";

type LocaleLayoutProps = {
  children: ReactNode;
  params: Promise<{ locale: string }>;
};

export const dynamicParams = false;

export function generateStaticParams() {
  return locales.map((locale) => ({ locale }));
}

export default async function LocaleLayout({
  children,
  params,
}: LocaleLayoutProps) {
  const { locale } = await params;

  if (!isLocale(locale)) {
    notFound();
  }

  const dictionary = getDictionary(locale);

  return (
    <I18nProvider locale={locale} dictionary={dictionary}>
      <AuthProvider>{children}</AuthProvider>
    </I18nProvider>
  );
}
