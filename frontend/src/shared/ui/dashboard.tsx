"use client";

import type { ReactNode } from "react";
import { Badge } from "@/src/shared/ui/badge";
import { Button } from "@/src/shared/ui/button";
import { EmptyState } from "@/src/shared/ui/empty-state";
import { ErrorState } from "@/src/shared/ui/error-state";
import { SearchInput } from "@/src/shared/ui/search-input";
import { Select } from "@/src/shared/ui/select";
import { Skeleton } from "@/src/shared/ui/skeleton";
import { badgeToneClassNames, timeRangeOptions } from "@/src/shared/constants/dashboard";
import { cn } from "@/src/shared/lib/cn";
import {
  formatRelativeLabel,
  getAlertStatusMeta,
  getHealthMeta,
  getHostStatusMeta,
  getSeverityMeta,
} from "@/src/shared/lib/dashboard";
import type {
  AlertStatus,
  BadgeTone,
  HealthState,
  HostStatus,
  Severity,
} from "@/src/shared/types/dashboard";

type ToneBadgeProps = {
  tone: BadgeTone;
  children: ReactNode;
  className?: string;
};

function DashboardPanelShell({
  children,
  className,
  tone = "default",
  chrome = "inset",
}: {
  children: ReactNode;
  className?: string;
  tone?: "default" | "muted";
  chrome?: "inset" | "ghost";
}) {
  return (
    <div
      className={cn(
        chrome === "inset" && "rounded-lg border border-[color:var(--border)]",
        chrome === "inset" &&
          (tone === "muted"
            ? "bg-[color:var(--surface-muted)]"
            : "bg-[color:var(--surface-subtle)]"),
        className
      )}
    >
      {children}
    </div>
  );
}

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

export function StatusBadge({ status }: { status: HostStatus | AlertStatus }) {
  const resolvedMeta =
    status === "online" ||
    status === "offline" ||
    status === "degraded" ||
    status === "enrolling"
      ? getHostStatusMeta(status)
      : getAlertStatusMeta(status as AlertStatus);

  return <ToneBadge tone={resolvedMeta.tone}>{resolvedMeta.label}</ToneBadge>;
}

export function SeverityBadge({ severity }: { severity: Severity }) {
  const meta = getSeverityMeta(severity);
  return <ToneBadge tone={meta.tone}>{meta.label}</ToneBadge>;
}

export function HealthBadge({ health }: { health: HealthState }) {
  const meta = getHealthMeta(health);
  return <ToneBadge tone={meta.tone}>{meta.label}</ToneBadge>;
}

type StatCardProps = {
  title: string;
  value: string;
  description?: string;
  eyebrow?: string;
  footer?: ReactNode;
  className?: string;
  chrome?: "inset" | "ghost";
};

export function StatCard({
  title,
  value,
  description,
  eyebrow,
  footer,
  className,
  chrome = "inset",
}: StatCardProps) {
  return (
    <DashboardPanelShell
      chrome={chrome}
      className={cn("flex min-h-32 flex-col justify-between gap-4 px-4 py-4", className)}
    >
      <div className="space-y-2">
        {eyebrow ? (
          <p className="text-[11px] font-semibold uppercase tracking-[0.2em] text-[color:var(--muted-foreground)]">
            {eyebrow}
          </p>
        ) : null}
        <p className="text-sm font-medium text-[color:var(--muted-foreground)]">{title}</p>
        <p className="text-3xl font-semibold tracking-tight text-[color:var(--foreground)]">{value}</p>
        {description ? <p className="text-sm text-[color:var(--muted-foreground)]">{description}</p> : null}
      </div>
      {footer ? <div>{footer}</div> : null}
    </DashboardPanelShell>
  );
}

type MetricCardProps = {
  label: string;
  value: string;
  change: number;
  trend: "up" | "down" | "flat";
  description?: string;
  className?: string;
  chrome?: "inset" | "ghost";
};

export function MetricCard({
  label,
  value,
  change,
  trend,
  description,
  className,
  chrome = "inset",
}: MetricCardProps) {
  const tone =
    trend === "up" ? "positive" : trend === "down" ? "danger" : "neutral";
  const sign = change > 0 ? "+" : "";

  return (
    <StatCard
      title={label}
      value={value}
      description={description}
      className={className}
      chrome={chrome}
      footer={
        <div className="flex items-center justify-between gap-3">
          <ToneBadge tone={tone}>{`${sign}${change.toFixed(1)}%`}</ToneBadge>
          <span className="text-xs text-[color:var(--muted-foreground)]">к предыдущему окну</span>
        </div>
      }
    />
  );
}

export function LoadingState({
  title = "Загрузка данных панели",
  lines = 3,
}: {
  title?: string;
  lines?: number;
}) {
  return (
    <DashboardPanelShell className="space-y-4 px-4 py-4">
      <div>
        <p className="text-sm font-medium text-[color:var(--muted-foreground)]">{title}</p>
      </div>
      <div className="space-y-2">
        {Array.from({ length: lines }).map((_, index) => (
          <Skeleton key={index} className="h-4 w-full rounded-full" />
        ))}
      </div>
    </DashboardPanelShell>
  );
}

type TableToolbarProps = {
  title?: string;
  description?: string;
  children?: ReactNode;
  actions?: ReactNode;
};

export function TableToolbar({
  title,
  description,
  children,
  actions,
}: TableToolbarProps) {
  return (
    <div className="flex flex-col gap-4 border-b border-[color:var(--border)] pb-4">
      {(title || description || actions) && (
        <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
          <div className="space-y-1">
            {title ? (
              <h3 className="text-sm font-semibold tracking-tight text-[color:var(--foreground)]">
                {title}
              </h3>
            ) : null}

            {description ? (
              <p className="text-sm text-[color:var(--muted-foreground)]">{description}</p>
            ) : null}
          </div>

          {actions ? <div className="flex flex-wrap items-center gap-2">{actions}</div> : null}
        </div>
      )}

      {children}
    </div>
  );
}

export function FilterBar({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between",
        className
      )}
    >
      {children}
    </div>
  );
}

export function TimeRangePicker({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <Select
      value={value}
      onChange={(event) => onChange(event.target.value)}
      options={timeRangeOptions}
      placeholder="Выберите период"
      selectSize="md"
      className="min-w-36"
    />
  );
}

export function DetailsDrawer({
  title,
  description,
  open,
  onClose,
  children,
}: {
  title: string;
  description?: string;
  open: boolean;
  onClose?: () => void;
  children: ReactNode;
}) {
  return (
    <aside
      className={cn(
        "h-full min-h-[24rem] bg-[color:var(--background)] transition-all",
        open ? "translate-x-0 opacity-100" : "pointer-events-none translate-x-3 opacity-0 lg:pointer-events-auto lg:translate-x-0 lg:opacity-100"
      )}
    >
      <div className="flex items-start justify-between gap-4 border-b border-[color:var(--border)] p-4">
        <div className="space-y-1">
          <h3 className="text-base font-semibold text-[color:var(--foreground)]">{title}</h3>
          {description ? <p className="text-sm text-[color:var(--muted-foreground)]">{description}</p> : null}
        </div>
        {onClose ? (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={onClose}
            className="lg:hidden"
          >
            Закрыть
          </Button>
        ) : null}
      </div>
      <div className="space-y-5 p-4">{children}</div>
    </aside>
  );
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
          <dd className="text-right text-sm font-medium text-[color:var(--foreground)]">{item.value}</dd>
        </div>
      ))}
    </dl>
  );
}

export function CodeBlock({
  code,
  language = "json",
}: {
  code: string;
  language?: string;
}) {
  return (
    <div className="overflow-hidden rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)]">
      <div className="border-b border-[color:var(--border)] px-4 py-2 text-xs uppercase tracking-[0.2em] text-[color:var(--muted-foreground)]">
        {language}
      </div>
      <pre className="overflow-x-auto p-4 text-xs leading-6 text-[color:var(--foreground)]">
        <code>{code}</code>
      </pre>
    </div>
  );
}

export function ChartCard({
  title,
  description,
  points,
  className,
  chrome = "inset",
}: {
  title: string;
  description?: string;
  points: number[];
  className?: string;
  chrome?: "inset" | "ghost";
}) {
  const max = Math.max(...points, 1);

  return (
    <DashboardPanelShell
      chrome={chrome}
      className={cn("space-y-5 px-4 py-4", className)}
    >
      <div className="space-y-1">
        <h3 className="text-base font-semibold text-[color:var(--foreground)]">{title}</h3>
        {description ? <p className="text-sm text-[color:var(--muted-foreground)]">{description}</p> : null}
      </div>
      <div className="flex h-40 items-end gap-2">
        {points.map((point, index) => (
          <div key={index} className="flex flex-1 items-end">
            <div
              className="w-full rounded-t-xl bg-gradient-to-t from-sky-500 to-cyan-300"
              style={{ height: `${Math.max((point / max) * 100, 12)}%` }}
            />
          </div>
        ))}
      </div>
    </DashboardPanelShell>
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
        <h2 className="text-lg font-semibold tracking-tight text-[color:var(--foreground)]">{title}</h2>
        {description ? <p className="text-sm text-[color:var(--muted-foreground)]">{description}</p> : null}
      </div>
      {action ? <div className="flex items-center gap-2">{action}</div> : null}
    </div>
  );
}

export function EntityMetaCard({
  title,
  subtitle,
  status,
  meta,
}: {
  title: string;
  subtitle?: string;
  status?: ReactNode;
  meta: Array<{ label: string; value: ReactNode }>;
}) {
  return (
    <DashboardPanelShell className="space-y-4 px-4 py-4">
      <div className="flex items-start justify-between gap-4">
        <div className="space-y-1">
          <h3 className="text-base font-semibold text-[color:var(--foreground)]">{title}</h3>
          {subtitle ? <p className="text-sm text-[color:var(--muted-foreground)]">{subtitle}</p> : null}
        </div>
        {status}
      </div>
      <KeyValueList items={meta} />
    </DashboardPanelShell>
  );
}

export function ActivityFeed({
  items,
  className,
  chrome = "inset",
}: {
  items: Array<{ id: string; title: string; description: string; timestamp: string }>;
  className?: string;
  chrome?: "inset" | "ghost";
}) {
  return (
    <DashboardPanelShell
      chrome={chrome}
      className={cn("space-y-4 px-4 py-4", className)}
    >
      <SectionHeader title="Последняя активность" description="Свежие операционные события по платформе." />
      <div className="space-y-4">
        {items.map((item) => (
          <div key={item.id} className="flex gap-3">
            <div className="mt-2 h-2.5 w-2.5 rounded-full bg-sky-500" />
            <div className="space-y-1">
              <p className="text-sm font-medium text-[color:var(--foreground)]">{item.title}</p>
              <p className="text-sm text-[color:var(--muted-foreground)]">{item.description}</p>
              <p className="text-xs text-[color:var(--muted-foreground)]">{formatRelativeLabel(item.timestamp)}</p>
            </div>
          </div>
        ))}
      </div>
    </DashboardPanelShell>
  );
}

export { EmptyState, ErrorState, SearchInput, Skeleton };

