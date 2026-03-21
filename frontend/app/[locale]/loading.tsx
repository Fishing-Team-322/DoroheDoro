"use client";

import { useOptionalI18n } from "@/src/shared/lib/i18n";
import { Spinner } from "@/src/shared/ui";

export default function LocaleLoading() {
  const i18n = useOptionalI18n();
  const message = i18n?.dictionary.app.loading ?? "Loading...";

  return (
    <main className="flex min-h-screen items-center justify-center p-6">
      <div className="flex w-full max-w-md items-center justify-center gap-3 rounded-lg border border-[color:var(--border)] bg-[color:var(--surface-elevated)] p-8 text-center shadow-sm">
        <Spinner size="sm" />
        <p className="text-sm text-[color:var(--muted-foreground)]">{message}</p>
      </div>
    </main>
  );
}
