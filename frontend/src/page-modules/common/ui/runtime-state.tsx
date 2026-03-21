"use client";

import { Card, EmptyState, Spinner } from "@/src/shared/ui";

export function LoadingCard({ label = "Loading data..." }: { label?: string }) {
  return (
    <Card className="flex min-h-40 items-center justify-center">
      <div className="inline-flex items-center gap-3 text-sm text-[color:var(--muted-foreground)]">
        <Spinner size="sm" />
        {label}
      </div>
    </Card>
  );
}

export function ErrorCard({ message }: { message: string }) {
  return (
    <Card>
      <EmptyState
        variant="flush"
        title="Request failed"
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
