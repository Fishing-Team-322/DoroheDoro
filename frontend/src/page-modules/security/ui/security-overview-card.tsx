"use client";

import { translateToneLabel, useI18n } from "@/src/shared/lib/i18n";
import { Badge, Card } from "@/src/shared/ui";
import type { SecuritySummaryItem } from "@/src/shared/lib/operations-workbench";

function toBadgeVariant(tone: SecuritySummaryItem["tone"]) {
  if (tone === "danger") {
    return "danger";
  }
  if (tone === "warning") {
    return "warning";
  }
  if (tone === "success") {
    return "success";
  }
  return "default";
}

export function SecurityOverviewCard({ item }: { item: SecuritySummaryItem }) {
  const { locale } = useI18n();

  return (
    <Card className="space-y-3 p-4">
      <div className="flex items-start justify-between gap-3">
        <p className="text-sm font-medium text-[color:var(--muted-foreground)]">
          {item.label}
        </p>
        <Badge variant={toBadgeVariant(item.tone)}>
          {translateToneLabel(item.tone, locale)}
        </Badge>
      </div>

      <p className="text-3xl font-semibold tracking-tight text-[color:var(--foreground)]">
        {item.value}
      </p>

      <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
        {item.helperText}
      </p>
    </Card>
  );
}
