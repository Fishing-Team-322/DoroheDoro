"use client";

import { useOptionalI18n } from "@/src/shared/lib/i18n";
import { Button } from "@/src/shared/ui";

type LocaleErrorPageProps = {
  error: Error & { digest?: string };
  reset: () => void;
};

export default function LocaleErrorPage({
  error,
  reset,
}: LocaleErrorPageProps) {
  const i18n = useOptionalI18n();
  const title = i18n?.dictionary.app.error.title ?? "Something went wrong";
  const description =
    i18n?.dictionary.app.error.description ?? "An unexpected error occurred.";
  const retry = i18n?.dictionary.app.error.retry ?? "Try again";

  return (
    <main className="flex min-h-screen items-center justify-center p-6">
      <div className="w-full max-w-md rounded-lg border border-[color:var(--border)] bg-[color:var(--surface-elevated)] p-8 text-center shadow-sm">
        <h1 className="text-2xl font-semibold">{title}</h1>
        <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
          {error.message || description}
        </p>
        <Button type="button" className="mt-6" onClick={() => reset()}>
          {retry}
        </Button>
      </div>
    </main>
  );
}
