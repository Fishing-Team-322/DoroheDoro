"use client";

import { getSiteCopy, useOptionalI18n } from "@/src/shared/lib/i18n";
import { Card, EmptyState, Spinner } from "@/src/shared/ui";

export function LoadingCard({ label }: { label?: string }) {
  const i18n = useOptionalI18n();
  const fallback =
    i18n != null
      ? getSiteCopy(i18n.locale).runtimeState.loadingData
      : "Loading data...";

  return (
    <Card className="flex min-h-40 items-center justify-center">
      <div className="inline-flex items-center gap-3 text-sm text-[color:var(--muted-foreground)]">
        <Spinner size="sm" />
        {label ?? fallback}
      </div>
    </Card>
  );
}

export function ErrorCard({ message }: { message: string }) {
  const i18n = useOptionalI18n();
  const title =
    i18n != null
      ? getSiteCopy(i18n.locale).runtimeState.requestFailed
      : "Request failed";

  return (
    <Card>
      <EmptyState
        variant="flush"
        title={title}
        description={message}
      />
    </Card>
  );
}

export function JsonValue({ value }: { value: unknown }) {
  return (
    <pre className="overflow-x-auto rounded-lg bg-[color:var(--surface)] p-3 text-xs leading-6 text-[color:var(--muted-foreground)]">
      {JSON.stringify(value ?? {}, null, 2)}
    </pre>
  );
}
