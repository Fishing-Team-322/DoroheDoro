"use client";

import Link from "next/link";
import type { Locale } from "@/src/shared/config";
import { Badge } from "@/src/shared/ui";
import { withLocalePath } from "@/src/shared/lib/i18n";
import {
  formatDateTime,
} from "@/src/features/operations/ui/operations-ui";
import {
  getSeverityTone,
  type AnomalyTimelineEntry,
} from "@/src/shared/lib/operations-workbench";

function toBadgeVariant(severity?: string) {
  const tone = getSeverityTone(severity);
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

export function AnomalyTimeline({
  locale,
  items,
}: {
  locale: Locale;
  items: AnomalyTimelineEntry[];
}) {
  return (
    <div className="space-y-4">
      {items.map((item) => (
        <div key={item.id} className="flex gap-4">
          <div className="flex w-6 flex-col items-center">
            <span className="mt-1 h-3 w-3 rounded-full bg-[color:var(--foreground)]" />
            <span className="mt-2 h-full min-h-8 w-px bg-[color:var(--border)]" />
          </div>

          <div className="min-w-0 flex-1 rounded-xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
            <div className="flex flex-wrap items-center gap-2">
              <Badge>{item.kind}</Badge>
              <Badge variant={toBadgeVariant(item.severity)}>{item.severity}</Badge>
              <span className="text-xs uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                {formatDateTime(item.timestamp)}
              </span>
            </div>

            <p className="mt-3 text-base font-semibold text-[color:var(--foreground)]">
              {item.title}
            </p>
            <p className="mt-2 text-sm leading-6 text-[color:var(--muted-foreground)]">
              {item.detail}
            </p>

            {item.href ? (
              <div className="mt-3">
                <Link
                  href={withLocalePath(locale, item.href)}
                  className="text-sm font-medium text-[color:var(--foreground)] underline-offset-4 hover:underline"
                >
                  Open related alert
                </Link>
              </div>
            ) : null}
          </div>
        </div>
      ))}
    </div>
  );
}
