import Link from "next/link";
import { LanguageSwitcher } from "@/src/features/locale-switcher";
import { getDictionary, withLocalePath } from "@/src/shared/lib/i18n";
import type { Locale } from "@/src/shared/config";

const demoLinks = [
  { href: "/login", label: "Sign in" },
  { href: "/overview", label: "Overview" },
  { href: "/profile", label: "Profile" },
  { href: "/demo", label: "Demo" },
  { href: "/ui-kit", label: "UI kit" },
  { href: "/forms", label: "Forms" },
  { href: "/api-demo", label: "API demo" },
  { href: "/query-demo", label: "Query params demo" },
  { href: "/table", label: "Table demo" },
] as const;

export function HomePage({ locale }: { locale: Locale }) {
  const dict = getDictionary(locale);

  return (
    <main className="mx-auto flex min-h-screen max-w-3xl flex-col items-start justify-center gap-6 p-8">
      <LanguageSwitcher currentLocale={locale} label={dict.switcherLabel} />
      <h1 className="text-3xl font-semibold">{dict.title}</h1>
      <p className="text-zinc-600">{dict.description}</p>
      <p className="text-sm text-zinc-500">
        {dict.currentLocale}: <strong>{locale}</strong>
      </p>

      <section className="w-full space-y-3">
        <h2 className="text-sm font-semibold uppercase tracking-wide text-zinc-500">
          Demo pages
        </h2>
        <div className="grid w-full gap-3 sm:grid-cols-2">
          {demoLinks.map((link) => (
            <Link
              key={link.href}
              href={withLocalePath(locale, link.href)}
              className="inline-flex h-10 items-center justify-center rounded-md border border-zinc-800 bg-zinc-950 px-4 text-sm font-medium text-zinc-100 transition-colors hover:bg-zinc-900"
            >
              {link.label}
            </Link>
          ))}
        </div>
      </section>
    </main>
  );
}
