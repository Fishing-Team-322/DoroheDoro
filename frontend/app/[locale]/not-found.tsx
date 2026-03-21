"use client";

import { useOptionalI18n } from "@/src/shared/lib/i18n";

export default function LocaleNotFound() {
  const i18n = useOptionalI18n();
  const title = i18n?.dictionary.app.notFound.title ?? "404";
  const description =
    i18n?.dictionary.app.notFound.description ?? "The page could not be found.";

  return (
    <main className="flex min-h-screen items-center justify-center p-6">
      <div className="w-full max-w-md rounded-lg border border-[color:var(--border)] bg-[color:var(--surface-elevated)] p-8 text-center shadow-sm">
        <h1 className="text-2xl font-semibold">{title}</h1>
        <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">{description}</p>
      </div>
    </main>
  );
}
