import Link from "next/link";
import { LanguageSwitcher } from "@/src/features/locale-switcher";
import { getDictionary, withLocalePath } from "@/src/shared/lib/i18n";
import type { Locale } from "@/src/shared/config";

export function DemoPage({ locale }: { locale: Locale }) {
  const dict = getDictionary(locale);

  return (
    <main className="mx-auto flex min-h-screen max-w-2xl flex-col items-start justify-center gap-6 p-8">
      <LanguageSwitcher currentLocale={locale} label={dict.switcherLabel} />
      <h1 className="text-3xl font-semibold">{dict.demo}</h1>
      <p className="text-zinc-600">{dict.description}</p>
      <Link className="text-sm underline" href={withLocalePath(locale, "/")}>
        {"<-"} {dict.home}
      </Link>
    </main>
  );
}
