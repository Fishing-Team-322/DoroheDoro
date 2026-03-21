"use client";

import type { ReactNode } from "react";
import { Badge } from "@/src/shared/ui/badge";
import { badgeToneClassNames } from "@/src/shared/constants/dashboard";
import { cn } from "@/src/shared/lib/cn";
import { getHealthMeta, getHostStatusMeta } from "@/src/shared/lib/dashboard";
import { useI18n } from "@/src/shared/lib/i18n";
import type { BadgeTone, HealthState, HostStatus } from "@/src/shared/types/dashboard";

type ToneBadgeProps = {
  tone: BadgeTone;
  children: ReactNode;
  className?: string;
};

export function ToneBadge({ tone, children, className }: ToneBadgeProps) {
  return (
    <Badge
      className={cn(
        "rounded-full border px-2.5 py-1 text-[11px] font-semibold tracking-wide",
        badgeToneClassNames[tone],
        className
      )}
    >
      {children}
    </Badge>
  );
}

export function StatusBadge({ status }: { status: HostStatus }) {
  const { dictionary } = useI18n();
  const meta = getHostStatusMeta(status, dictionary.statuses.host);

  return <ToneBadge tone={meta.tone}>{meta.label}</ToneBadge>;
}

export function HealthBadge({ health }: { health: HealthState }) {
  const { dictionary } = useI18n();
  const meta = getHealthMeta(health, dictionary.statuses.health);

  return <ToneBadge tone={meta.tone}>{meta.label}</ToneBadge>;
}

export function KeyValueList({
  items,
}: {
  items: Array<{ label: string; value: ReactNode }>;
}) {
  return (
    <dl className="space-y-3">
      {items.map((item) => (
        <div
          key={item.label}
          className="flex items-start justify-between gap-4 border-b border-dashed border-[color:var(--border)] pb-3 last:border-b-0 last:pb-0"
        >
          <dt className="text-sm text-[color:var(--muted-foreground)]">{item.label}</dt>
          <dd className="text-right text-sm font-medium text-[color:var(--foreground)]">
            {item.value}
          </dd>
        </div>
      ))}
    </dl>
  );
}

export function SectionHeader({
  title,
  description,
  action,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
      <div className="space-y-1">
        <h2 className="text-lg font-semibold tracking-tight text-[color:var(--foreground)]">
          {title}
        </h2>
        {description ? (
          <p className="text-sm text-[color:var(--muted-foreground)]">
            {description}
          </p>
        ) : null}
      </div>
      {action ? <div className="flex items-center gap-2">{action}</div> : null}
    </div>
  );
}
