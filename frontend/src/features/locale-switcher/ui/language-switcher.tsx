"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { locales, type Locale } from "@/src/shared/config";
import { useLocaleHref } from "@/src/shared/lib/i18n";

type LanguageSwitcherProps = {
  currentLocale: Locale;
  label: string;
};

export function LanguageSwitcher({
  currentLocale,
  label,
}: LanguageSwitcherProps) {
  const pathname = usePathname();

  const pathWithoutLocale = pathname.replace(/^\/(ru|en)(?=\/|$)/, "") || "/";

  return (
    <div className="inline-flex items-center gap-2 rounded-full border border-zinc-200 px-3 py-1 text-sm">
      <span className="text-zinc-600">{label}:</span>
      {locales.map((locale) => (
        <LocaleLink
          key={locale}
          locale={locale}
          currentLocale={currentLocale}
          path={pathWithoutLocale}
        />
      ))}
    </div>
  );
}

type LocaleLinkProps = {
  locale: Locale;
  currentLocale: Locale;
  path: string;
};

function LocaleLink({ locale, currentLocale, path }: LocaleLinkProps) {
  const href = useLocaleHref(locale, path);

  const isActive = locale === currentLocale;

  return (
    <Link
      href={href}
      className={
        isActive ? "font-semibold text-black" : "text-zinc-500 hover:text-black"
      }
    >
      {locale.toUpperCase()}
    </Link>
  );
}
