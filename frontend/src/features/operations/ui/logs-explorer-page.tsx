"use client";

import { startTransition, useMemo, useState } from "react";
import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import {
  Button,
  EmptyState,
  Input,
  Select,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/src/shared/ui";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
import { DateTimePicker } from "@/src/shared/ui/date-picker";
import {
  getLogsHistogram,
  getLogsSeverity,
  getLogsTopHosts,
  getLogsTopServices,
  searchLogs,
  type LogSearchFilters,
} from "../api";
import { useApiQuery } from "../model";
import {
  ErrorState,
  LoadingState,
  PageStack,
  RequestMetaLine,
  formatDateTime,
  fromDatetimeLocalValue,
  toDatetimeLocalValue,
} from "./operations-ui";

const SEVERITY_OPTIONS = [
  { value: "", label: "Любая severity" },
  { value: "debug", label: "debug" },
  { value: "info", label: "info" },
  { value: "warn", label: "warn" },
  { value: "error", label: "error" },
  { value: "fatal", label: "fatal" },
];

type CursorHistoryState = {
  key: string;
  values: string[];
};

function getSeverityTone(
  value?: string
): "success" | "warning" | "error" | "neutral" {
  const normalized = value?.toLowerCase();

  if (normalized === "debug" || normalized === "info") return "neutral";
  if (normalized === "warn") return "warning";
  if (normalized === "error" || normalized === "fatal") return "error";

  return "neutral";
}

function formatMetaValue(value?: string | null) {
  return value && value.trim().length > 0 ? value : "n/a";
}

export function LogsExplorerPage() {
  const { locale } = useI18n();
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();

  const urlFilters = useMemo<LogSearchFilters>(() => {
    return {
      query: searchParams.get("query") ?? undefined,
      from: searchParams.get("from") ?? undefined,
      to: searchParams.get("to") ?? undefined,
      host: searchParams.get("host") ?? undefined,
      service: searchParams.get("service") ?? undefined,
      severity: searchParams.get("severity") ?? undefined,
      agentId: searchParams.get("agent_id") ?? undefined,
      cursor: searchParams.get("cursor") ?? undefined,
      limit: 50,
    };
  }, [searchParams]);

  const filterKey = JSON.stringify({
    query: urlFilters.query,
    from: urlFilters.from,
    to: urlFilters.to,
    host: urlFilters.host,
    service: urlFilters.service,
    severity: urlFilters.severity,
    agentId: urlFilters.agentId,
  });

  const [cursorHistoryState, setCursorHistoryState] =
    useState<CursorHistoryState>({
      key: filterKey,
      values: [],
    });

  const cursorHistory =
    cursorHistoryState.key === filterKey ? cursorHistoryState.values : [];

  const resultsQuery = useApiQuery({
    queryFn: (signal) => searchLogs(urlFilters, signal),
    deps: [filterKey, urlFilters.cursor],
  });

  const histogramQuery = useApiQuery({
    queryFn: (signal) => getLogsHistogram(urlFilters, signal, "15m"),
    deps: [filterKey],
  });

  const severityQuery = useApiQuery({
    queryFn: (signal) => getLogsSeverity(urlFilters, signal, 5),
    deps: [filterKey],
  });

  const topHostsQuery = useApiQuery({
    queryFn: (signal) => getLogsTopHosts(urlFilters, signal, 5),
    deps: [filterKey],
  });

  const topServicesQuery = useApiQuery({
    queryFn: (signal) => getLogsTopServices(urlFilters, signal, 5),
    deps: [filterKey],
  });

  const replaceFilters = (
    nextFilters: Partial<LogSearchFilters>,
    options?: { resetCursor?: boolean }
  ) => {
    const merged: LogSearchFilters = {
      query: nextFilters.query ?? urlFilters.query,
      from: nextFilters.from ?? urlFilters.from,
      to: nextFilters.to ?? urlFilters.to,
      host: nextFilters.host ?? urlFilters.host,
      service: nextFilters.service ?? urlFilters.service,
      severity: nextFilters.severity ?? urlFilters.severity,
      agentId: nextFilters.agentId ?? urlFilters.agentId,
      cursor: options?.resetCursor
        ? undefined
        : (nextFilters.cursor ?? urlFilters.cursor),
      limit: 50,
    };

    const params = new URLSearchParams();

    if (merged.query) params.set("query", merged.query);
    if (merged.from) params.set("from", merged.from);
    if (merged.to) params.set("to", merged.to);
    if (merged.host) params.set("host", merged.host);
    if (merged.service) params.set("service", merged.service);
    if (merged.severity) params.set("severity", merged.severity);
    if (merged.agentId) params.set("agent_id", merged.agentId);
    if (merged.cursor) params.set("cursor", merged.cursor);

    startTransition(() => {
      router.replace(
        params.toString() ? `${pathname}?${params.toString()}` : pathname,
        { scroll: false }
      );
    });
  };

  return (
    <PageStack>
      <div className="rounded-[28px] border border-[color:var(--border)] bg-[color:var(--surface)] p-8 md:p-10">
        <div className="space-y-8">
          <div className="space-y-3">
            <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
              <div className="space-y-3">
                <h1 className="text-3xl font-semibold tracking-tight text-[color:var(--foreground)] md:text-5xl">
                  логи
                </h1>

                <p className="max-w-3xl text-base leading-7 text-[color:var(--muted-foreground)] md:text-lg">
                  Поиск по журналам с фильтрами в URL, быстрым обзором по
                  severity, host и service, а также постраничной навигацией по
                  результатам.
                </p>
              </div>

              <Link
                href={withLocalePath(locale, "/logs/live")}
                className="text-sm text-[color:var(--muted-foreground)] transition-colors hover:text-[color:var(--foreground)]"
              >
                Открыть поток логов
              </Link>
            </div>
          </div>

          <div className="border-t border-[color:var(--border)] pt-8">
            <LogsFiltersForm
              key={`logs-filters-${filterKey}`}
              filters={urlFilters}
              onApply={(draft) => {
                setCursorHistoryState({ key: filterKey, values: [] });
                replaceFilters(draft, { resetCursor: true });
              }}
              onReset={() => {
                setCursorHistoryState({ key: filterKey, values: [] });
                startTransition(() => {
                  router.replace(pathname, { scroll: false });
                });
              }}
            />
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 gap-6 xl:grid-cols-2">
        <DashboardBlock
          title="Гистограмма"
          subtitle="GET /api/v1/logs/histogram"
          meta={histogramQuery.meta}
        >
          {histogramQuery.isLoading && !histogramQuery.data ? (
            <LoadingState compact label="Загрузка гистограммы..." />
          ) : histogramQuery.error && !histogramQuery.data ? (
            <ErrorState
              error={histogramQuery.error}
              retry={() => void histogramQuery.refetch()}
            />
          ) : (
            <NamedStatList
              emptyLabel="Нет данных по временным бакетам."
              items={(histogramQuery.data?.items ?? []).map((item) => ({
                label: formatDateTime(item.ts),
                value: item.count,
              }))}
            />
          )}
        </DashboardBlock>

        <DashboardBlock
          title="Severity"
          subtitle="GET /api/v1/logs/severity"
          meta={severityQuery.meta}
        >
          {severityQuery.isLoading && !severityQuery.data ? (
            <LoadingState compact label="Загрузка распределения severity..." />
          ) : severityQuery.error && !severityQuery.data ? (
            <ErrorState
              error={severityQuery.error}
              retry={() => void severityQuery.refetch()}
            />
          ) : (
            <NamedStatList
              emptyLabel="Нет данных по severity."
              items={(severityQuery.data?.items ?? []).map((item) => ({
                label: item.name,
                value: item.count,
                tone: getSeverityTone(item.name),
              }))}
            />
          )}
        </DashboardBlock>
      </div>

      <div className="grid grid-cols-1 gap-6 xl:grid-cols-2">
        <DashboardBlock
          title="Топ host"
          subtitle="GET /api/v1/logs/top-hosts"
          meta={topHostsQuery.meta}
        >
          {topHostsQuery.isLoading && !topHostsQuery.data ? (
            <LoadingState compact label="Загрузка top hosts..." />
          ) : topHostsQuery.error && !topHostsQuery.data ? (
            <ErrorState
              error={topHostsQuery.error}
              retry={() => void topHostsQuery.refetch()}
            />
          ) : (
            <NamedStatList
              emptyLabel="Нет данных по host."
              items={(topHostsQuery.data?.items ?? []).map((item) => ({
                label: item.name,
                value: item.count,
                onClick: () => {
                  setCursorHistoryState({ key: filterKey, values: [] });
                  replaceFilters({ host: item.name }, { resetCursor: true });
                },
              }))}
            />
          )}
        </DashboardBlock>

        <DashboardBlock
          title="Топ service"
          subtitle="GET /api/v1/logs/top-services"
          meta={topServicesQuery.meta}
        >
          {topServicesQuery.isLoading && !topServicesQuery.data ? (
            <LoadingState compact label="Загрузка top services..." />
          ) : topServicesQuery.error && !topServicesQuery.data ? (
            <ErrorState
              error={topServicesQuery.error}
              retry={() => void topServicesQuery.refetch()}
            />
          ) : (
            <NamedStatList
              emptyLabel="Нет данных по service."
              items={(topServicesQuery.data?.items ?? []).map((item) => ({
                label: item.name,
                value: item.count,
                onClick: () => {
                  setCursorHistoryState({ key: filterKey, values: [] });
                  replaceFilters({ service: item.name }, { resetCursor: true });
                },
              }))}
            />
          )}
        </DashboardBlock>
      </div>

      <div className="rounded-[28px] border border-[color:var(--border)] bg-[color:var(--surface)] p-8 md:p-10">
        <div className="space-y-8">
          <div className="space-y-3">
            <h2 className="text-2xl font-semibold tracking-tight text-[color:var(--foreground)] md:text-4xl">
              результаты
            </h2>
            <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
              POST /api/v1/logs/search
            </p>
          </div>

          <div className="border-t border-[color:var(--border)] pt-8">
            {resultsQuery.isLoading && !resultsQuery.data ? (
              <LoadingState label="Поиск логов..." />
            ) : resultsQuery.error && !resultsQuery.data ? (
              <ErrorState
                error={resultsQuery.error}
                retry={() => void resultsQuery.refetch()}
              />
            ) : (
              <div className="space-y-6">
                <div className="overflow-auto rounded-2xl">
                  <Table>
                    <TableHeader>
                      <TableRow className="border-b-0 hover:bg-transparent">
                        <TableHead>время</TableHead>
                        <TableHead>severity</TableHead>
                        <TableHead>host</TableHead>
                        <TableHead>service</TableHead>
                        <TableHead>agent id</TableHead>
                        <TableHead>сообщение</TableHead>
                      </TableRow>
                    </TableHeader>

                    <TableBody>
                      {(resultsQuery.data?.items ?? []).map((log) => {
                        const severityTone = getSeverityTone(log.severity);

                        return (
                          <TableRow key={`${log.timestamp}-${log.message}`}>
                            <TableCell className="whitespace-nowrap">
                              {formatDateTime(log.timestamp)}
                            </TableCell>

                            <TableCell>
                              <span
                                className={`font-semibold ${
                                  severityTone === "error"
                                    ? "text-red-400"
                                    : severityTone === "warning"
                                      ? "text-amber-400"
                                      : severityTone === "success"
                                        ? "text-emerald-400"
                                        : "text-[color:var(--foreground)]"
                                }`}
                              >
                                {(log.severity ?? "n/a").toUpperCase()}
                              </span>
                            </TableCell>

                            <TableCell>{formatMetaValue(log.host)}</TableCell>

                            <TableCell>
                              {formatMetaValue(log.service)}
                            </TableCell>

                            <TableCell className="font-mono text-xs text-[color:var(--muted-foreground)]">
                              {formatMetaValue(log.agentId)}
                            </TableCell>

                            <TableCell className="max-w-3xl whitespace-normal break-words text-[color:var(--foreground)]">
                              {log.message}
                            </TableCell>
                          </TableRow>
                        );
                      })}

                      {(resultsQuery.data?.items.length ?? 0) === 0 ? (
                        <TableRow>
                          <TableCell colSpan={6}>
                            <EmptyState
                              variant="flush"
                              title="Ничего не найдено"
                              description="Расширьте временной диапазон, измените запрос или ослабьте фильтры."
                            />
                          </TableCell>
                        </TableRow>
                      ) : null}
                    </TableBody>
                  </Table>
                </div>

                <div className="flex flex-col gap-4 border-t border-[color:var(--border)] pt-6 lg:flex-row lg:items-center lg:justify-between">
                  <RequestMetaLine meta={resultsQuery.meta} />

                  <div className="flex flex-wrap gap-3">
                    <Button
                      variant="outline"
                      size="sm"
                      className="h-11 px-5"
                      disabled={cursorHistory.length === 0}
                      onClick={() => {
                        const nextHistory = cursorHistory.slice(0, -1);
                        setCursorHistoryState({
                          key: filterKey,
                          values: nextHistory,
                        });

                        replaceFilters(
                          {
                            cursor:
                              nextHistory[nextHistory.length - 1] || undefined,
                          },
                          { resetCursor: false }
                        );
                      }}
                    >
                      Назад
                    </Button>

                    <Button
                      size="sm"
                      className="h-11 px-5"
                      disabled={!resultsQuery.data?.nextCursor}
                      onClick={() => {
                        const nextCursor = resultsQuery.data?.nextCursor;

                        if (!nextCursor) return;

                        setCursorHistoryState({
                          key: filterKey,
                          values: [...cursorHistory, urlFilters.cursor ?? ""],
                        });

                        replaceFilters(
                          { cursor: nextCursor },
                          { resetCursor: false }
                        );
                      }}
                    >
                      Дальше
                    </Button>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </PageStack>
  );
}

function LogsFiltersForm({
  filters,
  onApply,
  onReset,
}: {
  filters: LogSearchFilters;
  onApply: (filters: LogSearchFilters) => void;
  onReset: () => void;
}) {
  const [query, setQuery] = useState(filters.query ?? "");
  const [from, setFrom] = useState(toDatetimeLocalValue(filters.from));
  const [to, setTo] = useState(toDatetimeLocalValue(filters.to));
  const [host, setHost] = useState(filters.host ?? "");
  const [service, setService] = useState(filters.service ?? "");
  const [severity, setSeverity] = useState(filters.severity ?? "");
  const [agentId, setAgentId] = useState(filters.agentId ?? "");

  return (
    <div className="space-y-5">
      <div className="grid grid-cols-1 gap-4 xl:grid-cols-3">
        <Input
          label="Поиск"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="timeout OR error"
          inputSize="md"
        />

        <Input
          label="Host"
          value={host}
          onChange={(event) => setHost(event.target.value)}
          inputSize="md"
        />

        <Input
          label="Service"
          value={service}
          onChange={(event) => setService(event.target.value)}
          inputSize="md"
        />
      </div>

      <div className="grid grid-cols-1 gap-4 xl:grid-cols-4">
        <DateTimePicker label="От" value={from} onChange={setFrom} />

        <DateTimePicker label="До" value={to} onChange={setTo} />

        <div className="w-full">
          <Select
            value={severity}
            onChange={(event) => setSeverity(event.target.value)}
            options={SEVERITY_OPTIONS}
            placeholder="Любая severity"
            selectSize="md"
            triggerClassName="h-14 px-4 text-sm border border-[var(--input-border)] hover:border-[var(--input-border-hover)] hover:bg-[var(--input-background-hover)] focus-visible:border-[var(--ring)] focus-visible:bg-[var(--input-background-focus)] focus-visible:shadow-[0_0_0_1px_var(--ring),0_0_0_2px_rgba(113,113,122,0.08)]"
          />
        </div>

        <Input
          label="Agent ID"
          value={agentId}
          onChange={(event) => setAgentId(event.target.value)}
          inputSize="md"
        />
      </div>

      <div className="flex flex-wrap gap-3">
        <Button
          size="sm"
          className="h-11 px-5"
          onClick={() =>
            onApply({
              query: query || undefined,
              from: fromDatetimeLocalValue(from),
              to: fromDatetimeLocalValue(to),
              host: host || undefined,
              service: service || undefined,
              severity: severity || undefined,
              agentId: agentId || undefined,
              limit: 50,
            })
          }
        >
          Применить
        </Button>
        <Button
          variant="outline"
          size="sm"
          className="h-11 px-5"
          onClick={onReset}
        >
          Сбросить
        </Button>
      </div>
    </div>
  );
}

function DashboardBlock({
  title,
  subtitle,
  meta,
  children,
}: {
  title: string;
  subtitle: string;
  meta?: unknown;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-[28px] border border-[color:var(--border)] bg-[color:var(--surface)] p-8 md:p-10">
      <div className="space-y-8">
        <div className="space-y-3">
          <h2 className="text-2xl font-semibold tracking-tight text-[color:var(--foreground)] md:text-4xl">
            {title}
          </h2>
          <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
            {subtitle}
          </p>
        </div>

        <div className="border-t border-[color:var(--border)] pt-8">
          <div className="space-y-6">
            {children}
            <RequestMetaLine meta={meta} />
          </div>
        </div>
      </div>
    </div>
  );
}

function NamedStatList({
  items,
  emptyLabel,
}: {
  items: Array<{
    label: string;
    value: number;
    tone?: "success" | "warning" | "error" | "neutral";
    onClick?: () => void;
  }>;
  emptyLabel: string;
}) {
  if (items.length === 0) {
    return (
      <EmptyState variant="flush" title="Нет данных" description={emptyLabel} />
    );
  }

  return (
    <div className="overflow-hidden rounded-2xl">
      <Table>
        <TableHeader>
          <TableRow className="border-b-0 hover:bg-transparent">
            <TableHead>параметр</TableHead>
            <TableHead>значение</TableHead>
          </TableRow>
        </TableHeader>

        <TableBody>
          {items.map((item) => (
            <TableRow
              key={`${item.label}-${item.value}`}
              className={item.onClick ? "cursor-pointer" : undefined}
              onClick={item.onClick}
            >
              <TableCell className="text-[color:var(--foreground)]">
                {item.label}
              </TableCell>

              <TableCell>
                <span
                  className={`font-semibold ${
                    item.tone === "error"
                      ? "text-red-400"
                      : item.tone === "warning"
                        ? "text-amber-400"
                        : item.tone === "success"
                          ? "text-emerald-400"
                          : "text-[color:var(--foreground)]"
                  }`}
                >
                  {item.value}
                </span>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
