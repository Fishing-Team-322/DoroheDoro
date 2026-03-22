"use client";

import type { ReactNode, TextareaHTMLAttributes } from "react";
import type { ApiError, ApiResponseMeta } from "@/src/shared/lib/api";
import { cn } from "@/src/shared/lib/cn";
import {
  Badge,
  Button,
  Card,
  EmptyState,
  FormLabel,
  Input,
  SearchInput,
  Select,
  Spinner,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/src/shared/ui";
import { getDeploymentStatusCategory } from "../api";

export function PageStack({ children }: { children: ReactNode }) {
  return <div className="space-y-6">{children}</div>;
}

export function SectionCard({
  title,
  description,
  action,
  children,
  className,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <Card className={cn("space-y-5 p-5 sm:p-6", className)}>
      <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
        <div className="space-y-1">
          <h2 className="text-xl font-semibold text-[color:var(--foreground)]">
            {title}
          </h2>
          {description ? (
            <p className="max-w-3xl text-lg leading-6 text-[color:var(--muted-foreground)]">
              {description}
            </p>
          ) : null}
        </div>

        {action ? <div className="flex flex-wrap gap-2">{action}</div> : null}
      </div>

      {children}
    </Card>
  );
}

export function LoadingState({
  label = "Loading data...",
  compact = false,
}: {
  label?: string;
  compact?: boolean;
}) {
  return (
    <div
      className={cn(
        "flex items-center justify-center gap-3 rounded-xl border border-[color:var(--border)] bg-[color:var(--surface-subtle)] text-sm text-[color:var(--muted-foreground)]",
        compact ? "px-4 py-5" : "px-5 py-8"
      )}
    >
      <Spinner size="sm" />
      <span>{label}</span>
    </div>
  );
}

export function ErrorState({
  error,
  retry,
  title = "Request failed",
}: {
  error?: ApiError;
  retry?: () => void;
  title?: string;
}) {
  return (
    <div className="rounded-xl border border-[color:var(--status-danger-border)] bg-[color:var(--status-danger-bg)]/70 p-4">
      <div className="space-y-2">
        <div className="flex flex-wrap items-center gap-2">
          <p className="text-sm font-semibold text-[color:var(--status-danger-fg)]">
            {title}
          </p>
          {error?.code ? (
            <Badge variant="danger" className="uppercase">
              {error.code}
            </Badge>
          ) : null}
        </div>

        <p className="text-sm leading-6 text-[color:var(--status-danger-fg)]/90">
          {error?.message ?? "The backend returned an unexpected error."}
        </p>

        {error?.requestId || error?.status || error?.natsSubject ? (
          <div className="flex flex-wrap gap-3 text-xs text-[color:var(--status-danger-fg)]/80">
            {error.status != null ? <span>Status: {error.status}</span> : null}
            {error.requestId ? <span>Request ID: {error.requestId}</span> : null}
            {error.natsSubject ? <span>Subject: {error.natsSubject}</span> : null}
          </div>
        ) : null}

        {retry ? (
          <Button variant="outline" size="sm" className="h-9 px-3" onClick={retry}>
            Retry
          </Button>
        ) : null}
      </div>
    </div>
  );
}

export function UnavailableState({
  title = "Unavailable",
  description,
  action,
}: {
  title?: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <EmptyState title={title} description={description} action={action} />
  );
}

export function NoticeBanner({
  title,
  description,
}: {
  title: string;
  description: string;
}) {
  return (
    <div className="rounded-xl border border-[color:var(--status-info-border)] bg-[color:var(--status-info-bg)]/80 px-4 py-3">
      <p className="text-Lg font-semibold text-[color:var(--status-info-fg)]">
        {title}
      </p>
      <p className="mt-1 text-base leading-6 text-[color:var(--status-info-fg)]/85">
        {description}
      </p>
    </div>
  );
}

export function StatusBadge({ value }: { value?: string }) {
  const normalized = (value ?? "unknown").trim() || "unknown";
  const category = getDeploymentStatusCategory(normalized);

  return (
    <Badge
      variant={
        category === "success"
          ? "success"
          : category === "warning"
            ? "warning"
            : category === "danger"
              ? "danger"
              : "default"
      }
      className="uppercase"
    >
      {normalized}
    </Badge>
  );
}

export function MetricCard({
  label,
  value,
  hint,
  status,
}: {
  label: string;
  value: ReactNode;
  hint?: string;
  status?: string;
}) {
  return (
    <Card className="space-y-3 p-4">
      <div className="flex items-start justify-between gap-3">
        <p className="text-sm text-[color:var(--muted-foreground)]">{label}</p>
        {status ? <StatusBadge value={status} /> : null}
      </div>
      <div className="text-2xl font-semibold tracking-tight text-[color:var(--foreground)]">
        {value}
      </div>
      {hint ? (
        <p className="text-xs text-[color:var(--muted-foreground)]">{hint}</p>
      ) : null}
    </Card>
  );
}

export function DetailGrid({
  items,
}: {
  items: Array<{ label: string; value: ReactNode }>;
}) {
  return (
    <dl className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
      {items.map((item) => (
        <div
          key={item.label}
          className="rounded-xl border border-[color:var(--border)] bg-[color:var(--surface)] px-4 py-3"
        >
          <dt className="text-xs uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
            {item.label}
          </dt>
          <dd className="mt-2 text-sm leading-6 text-[color:var(--foreground)]">
            {item.value}
          </dd>
        </div>
      ))}
    </dl>
  );
}

export function LabelMap({
  labels,
  emptyLabel = "No labels",
}: {
  labels?: Record<string, string>;
  emptyLabel?: string;
}) {
  const entries = Object.entries(labels ?? {});

  if (entries.length === 0) {
    return <span className="text-[color:var(--muted-foreground)]">{emptyLabel}</span>;
  }

  return (
    <div className="flex flex-wrap gap-2">
      {entries.map(([key, value]) => (
        <Badge key={`${key}-${value}`} variant="default">
          {key}: {value}
        </Badge>
      ))}
    </div>
  );
}

export function TokenList({
  items,
  emptyLabel = "No items",
}: {
  items: string[];
  emptyLabel?: string;
}) {
  if (items.length === 0) {
    return (
      <span className="text-[color:var(--muted-foreground)]">{emptyLabel}</span>
    );
  }

  return (
    <div className="flex flex-wrap gap-2">
      {items.map((item) => (
        <Badge key={item} variant="default">
          {item}
        </Badge>
      ))}
    </div>
  );
}

export function JsonPreview({
  value,
  emptyLabel = "No JSON payload available.",
}: {
  value: unknown;
  emptyLabel?: string;
}) {
  if (value == null) {
    return (
      <p className="text-sm text-[color:var(--muted-foreground)]">{emptyLabel}</p>
    );
  }

  return (
    <pre className="max-h-[420px] overflow-auto rounded-xl border border-[color:var(--border)] bg-black/30 p-4 text-xs leading-6 text-[color:var(--foreground)]">
      {JSON.stringify(value, null, 2)}
    </pre>
  );
}

export function RequestMetaLine({ meta }: { meta?: ApiResponseMeta }) {
  if (!meta) {
    return null;
  }

  return (
    <div className="flex flex-wrap gap-3 text-xs text-[color:var(--muted-foreground)]">
      <span>HTTP {meta.status}</span>
      {meta.requestId ? <span>Request ID: {meta.requestId}</span> : null}
      {meta.natsSubject ? <span>Subject: {meta.natsSubject}</span> : null}
    </div>
  );
}

export function TableCard({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <Card className={cn("overflow-hidden p-0", className)}>
      <div className="overflow-x-auto">{children}</div>
    </Card>
  );
}

export function DataTable({
  columns,
  rows,
  isEmpty,
  emptyTitle,
  emptyDescription,
}: {
  columns: string[];
  rows: ReactNode;
  isEmpty?: boolean;
  emptyTitle?: string;
  emptyDescription?: string;
}) {
  return (
    <Table>
      <TableHeader>
        <TableRow>
          {columns.map((column) => (
            <TableHead key={column}>{column}</TableHead>
          ))}
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows}
        {isEmpty && emptyTitle ? (
          <TableRow>
            <TableCell colSpan={columns.length}>
              <EmptyState
                variant="flush"
                title={emptyTitle}
                description={emptyDescription}
              />
            </TableCell>
          </TableRow>
        ) : null}
      </TableBody>
    </Table>
  );
}

export function CursorPagination({
  hasPrevious,
  hasNext,
  onPrevious,
  onNext,
  previousLabel = "Previous",
  nextLabel = "Next",
}: {
  hasPrevious: boolean;
  hasNext: boolean;
  onPrevious: () => void;
  onNext: () => void;
  previousLabel?: string;
  nextLabel?: string;
}) {
  return (
    <div className="flex flex-wrap items-center justify-end gap-2">
      <Button
        variant="outline"
        size="sm"
        className="h-9 px-3"
        disabled={!hasPrevious}
        onClick={onPrevious}
      >
        {previousLabel}
      </Button>
      <Button
        variant="outline"
        size="sm"
        className="h-9 px-3"
        disabled={!hasNext}
        onClick={onNext}
      >
        {nextLabel}
      </Button>
    </div>
  );
}

export function TextAreaField({
  label,
  helperText,
  error,
  className,
  ...props
}: TextareaHTMLAttributes<HTMLTextAreaElement> & {
  label: string;
  helperText?: string;
  error?: string;
}) {
  return (
    <div className="space-y-2">
      <FormLabel htmlFor={props.id}>{label}</FormLabel>
      <textarea
        {...props}
        className={cn(
          "min-h-28 w-full rounded-md border border-[color:var(--input-border)] bg-[color:var(--input-background)] px-3 py-3 text-sm text-[color:var(--foreground)] outline-none transition-colors placeholder:text-[color:var(--muted-foreground)] focus:border-[color:var(--ring)] focus:bg-[color:var(--input-background-focus)] focus:shadow-[0_0_0_1px_var(--ring)]",
          error &&
            "border-[color:var(--status-danger-border)] focus:border-[color:var(--status-danger-border)] focus:shadow-[0_0_0_1px_var(--status-danger-border)]",
          className
        )}
      />
      {error ? (
        <p className="text-sm text-[color:var(--status-danger-fg)]">{error}</p>
      ) : helperText ? (
        <p className="text-sm text-[color:var(--muted-foreground)]">
          {helperText}
        </p>
      ) : null}
    </div>
  );
}

export function FilterGrid({ children }: { children: ReactNode }) {
  return <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">{children}</div>;
}

export function FilterField({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="space-y-2">
      <FormLabel>{label}</FormLabel>
      {children}
    </div>
  );
}

export function NamedCountList({
  items,
  emptyLabel = "No data available.",
  onSelect,
}: {
  items: Array<{ name: string; count: number }>;
  emptyLabel?: string;
  onSelect?: (value: string) => void;
}) {
  if (items.length === 0) {
    return <EmptyState variant="flush" title={emptyLabel} />;
  }

  const max = Math.max(...items.map((item) => item.count), 1);

  return (
    <div className="space-y-3">
      {items.map((item) => (
        <button
          key={`${item.name}-${item.count}`}
          type="button"
          onClick={() => onSelect?.(item.name)}
          className={cn(
            "w-full rounded-lg text-left",
            onSelect && "transition-colors hover:bg-[color:var(--surface)]"
          )}
          disabled={!onSelect}
        >
          <div className="flex items-center justify-between gap-3">
            <span className="truncate text-sm text-[color:var(--foreground)]">
              {item.name}
            </span>
            <span className="text-sm text-[color:var(--muted-foreground)]">
              {formatNumber(item.count)}
            </span>
          </div>
          <div className="mt-2 h-2 rounded-full bg-[color:var(--surface)]">
            <div
              className="h-2 rounded-full bg-white/75"
              style={{ width: `${Math.max(8, (item.count / max) * 100)}%` }}
            />
          </div>
        </button>
      ))}
    </div>
  );
}

export function HistogramBars({
  items,
  emptyLabel = "No histogram data available.",
}: {
  items: Array<{ ts: string; count: number }>;
  emptyLabel?: string;
}) {
  if (items.length === 0) {
    return <EmptyState variant="flush" title={emptyLabel} />;
  }

  const max = Math.max(...items.map((item) => item.count), 1);

  return (
    <div className="space-y-3">
      <div className="flex h-36 items-end gap-2">
        {items.map((item) => (
          <div key={`${item.ts}-${item.count}`} className="flex min-w-0 flex-1 flex-col items-center gap-2">
            <div
              className="w-full rounded-t-md bg-white/80 transition-opacity"
              style={{
                height: `${Math.max(8, (item.count / max) * 100)}%`,
              }}
              title={`${item.count} at ${formatDateTime(item.ts)}`}
            />
          </div>
        ))}
      </div>

      <div className="grid grid-cols-2 gap-2 text-xs text-[color:var(--muted-foreground)] sm:grid-cols-4">
        {items.map((item) => (
          <div key={`${item.ts}-legend`} className="truncate">
            {formatShortDateTime(item.ts)}: {formatNumber(item.count)}
          </div>
        ))}
      </div>
    </div>
  );
}

export function SearchField({
  label,
  value,
  onChange,
  placeholder,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}) {
  return (
    <FilterField label={label}>
      <SearchInput
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
      />
    </FilterField>
  );
}

export function TextField({
  label,
  value,
  onChange,
  placeholder,
  type = "text",
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  type?: "text" | "datetime-local";
}) {
  return (
    <FilterField label={label}>
      <Input
        type={type}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        inputSize="sm"
      />
    </FilterField>
  );
}

export function SelectField({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  options: Array<{ value: string; label: string }>;
}) {
  return (
    <FilterField label={label}>
      <Select
        value={value}
        onChange={(event) => onChange(event.target.value)}
        options={options}
        selectSize="sm"
      />
    </FilterField>
  );
}

export function formatDateTime(value?: string): string {
  if (!value) {
    return "n/a";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

export function formatShortDateTime(value?: string): string {
  if (!value) {
    return "n/a";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

export function formatRelativeTime(value?: string): string {
  if (!value) {
    return "n/a";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  const deltaSeconds = Math.round((date.getTime() - Date.now()) / 1000);
  const absSeconds = Math.abs(deltaSeconds);
  const formatter = new Intl.RelativeTimeFormat(undefined, {
    numeric: "auto",
  });

  if (absSeconds < 60) {
    return formatter.format(deltaSeconds, "second");
  }

  const deltaMinutes = Math.round(deltaSeconds / 60);
  if (Math.abs(deltaMinutes) < 60) {
    return formatter.format(deltaMinutes, "minute");
  }

  const deltaHours = Math.round(deltaMinutes / 60);
  if (Math.abs(deltaHours) < 24) {
    return formatter.format(deltaHours, "hour");
  }

  const deltaDays = Math.round(deltaHours / 24);
  return formatter.format(deltaDays, "day");
}

export function formatNumber(value?: number): string {
  if (value == null || Number.isNaN(value)) {
    return "0";
  }

  return new Intl.NumberFormat().format(value);
}

export function formatParamsSummary(
  value?: Record<string, unknown>
): string {
  const entries = Object.entries(value ?? {});
  if (entries.length === 0) {
    return "No params";
  }

  return entries
    .slice(0, 3)
    .map(([key, item]) => `${key}=${String(item)}`)
    .join(", ");
}

export function formatMaybeValue(value?: string | number | null): string {
  if (value == null || value === "") {
    return "n/a";
  }

  return String(value);
}

export function toDatetimeLocalValue(value?: string): string {
  if (!value) {
    return "";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "";
  }

  const pad = (part: number) => String(part).padStart(2, "0");
  const year = date.getFullYear();
  const month = pad(date.getMonth() + 1);
  const day = pad(date.getDate());
  const hours = pad(date.getHours());
  const minutes = pad(date.getMinutes());
  return `${year}-${month}-${day}T${hours}:${minutes}`;
}

export function fromDatetimeLocalValue(value: string): string | undefined {
  if (!value) {
    return undefined;
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return undefined;
  }

  return date.toISOString();
}
