"use client";

import { startTransition, useMemo, useState } from "react";
import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { PageHeader } from "@/src/widgets/dashboard-layout";
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
  CursorPagination,
  DataTable,
  ErrorState,
  FilterGrid,
  LoadingState,
  NamedCountList,
  PageStack,
  RequestMetaLine,
  SearchField,
  SectionCard,
  SelectField,
  StatusBadge,
  TextField,
  formatDateTime,
  fromDatetimeLocalValue,
  toDatetimeLocalValue,
} from "./operations-ui";
import { Button, TableCell, TableRow } from "@/src/shared/ui";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";

const SEVERITY_OPTIONS = [
  { value: "", label: "Any severity" },
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

  const [cursorHistoryState, setCursorHistoryState] = useState<CursorHistoryState>(
    { key: filterKey, values: [] }
  );

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
        : nextFilters.cursor ?? urlFilters.cursor,
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
      <PageHeader
        title="Logs Explorer"
        description="Search logs with URL-synced filters, inspect analytics blocks independently, and page through results via cursors when available."
        action={
          <Link
            href={withLocalePath(locale, "/logs/live")}
            className="text-sm text-[color:var(--muted-foreground)] transition-colors hover:text-[color:var(--foreground)]"
          >
            Open live logs
          </Link>
        }
      />

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

      <div className="grid grid-cols-1 gap-6 xl:grid-cols-2">
        <SectionCard
          title="Histogram"
          description="`GET /api/v1/logs/histogram`"
        >
          {histogramQuery.isLoading && !histogramQuery.data ? (
            <LoadingState compact label="Loading histogram..." />
          ) : histogramQuery.error && !histogramQuery.data ? (
            <ErrorState error={histogramQuery.error} retry={() => void histogramQuery.refetch()} />
          ) : (
            <div className="space-y-4">
              <RequestMetaLine meta={histogramQuery.meta} />
              <NamedCountList
                items={(histogramQuery.data?.items ?? []).map((item) => ({
                  name: formatDateTime(item.ts),
                  count: item.count,
                }))}
                emptyLabel="No histogram buckets were returned."
              />
            </div>
          )}
        </SectionCard>

        <SectionCard
          title="Severity"
          description="`GET /api/v1/logs/severity`"
        >
          {severityQuery.isLoading && !severityQuery.data ? (
            <LoadingState compact label="Loading severity breakdown..." />
          ) : severityQuery.error && !severityQuery.data ? (
            <ErrorState error={severityQuery.error} retry={() => void severityQuery.refetch()} />
          ) : (
            <div className="space-y-4">
              <NamedCountList items={severityQuery.data?.items ?? []} />
              <RequestMetaLine meta={severityQuery.meta} />
            </div>
          )}
        </SectionCard>
      </div>

      <div className="grid grid-cols-1 gap-6 xl:grid-cols-2">
        <SectionCard title="Top Hosts" description="`GET /api/v1/logs/top-hosts`">
          {topHostsQuery.isLoading && !topHostsQuery.data ? (
            <LoadingState compact label="Loading top hosts..." />
          ) : topHostsQuery.error && !topHostsQuery.data ? (
            <ErrorState error={topHostsQuery.error} retry={() => void topHostsQuery.refetch()} />
          ) : (
            <div className="space-y-4">
              <NamedCountList
                items={topHostsQuery.data?.items ?? []}
                onSelect={(value) => {
                  setCursorHistoryState({ key: filterKey, values: [] });
                  replaceFilters({ host: value }, { resetCursor: true });
                }}
              />
              <RequestMetaLine meta={topHostsQuery.meta} />
            </div>
          )}
        </SectionCard>

        <SectionCard
          title="Top Services"
          description="`GET /api/v1/logs/top-services`"
        >
          {topServicesQuery.isLoading && !topServicesQuery.data ? (
            <LoadingState compact label="Loading top services..." />
          ) : topServicesQuery.error && !topServicesQuery.data ? (
            <ErrorState error={topServicesQuery.error} retry={() => void topServicesQuery.refetch()} />
          ) : (
            <div className="space-y-4">
              <NamedCountList
                items={topServicesQuery.data?.items ?? []}
                onSelect={(value) => {
                  setCursorHistoryState({ key: filterKey, values: [] });
                  replaceFilters({ service: value }, { resetCursor: true });
                }}
              />
              <RequestMetaLine meta={topServicesQuery.meta} />
            </div>
          )}
        </SectionCard>
      </div>

      <SectionCard title="Results" description="`POST /api/v1/logs/search`">
        {resultsQuery.isLoading && !resultsQuery.data ? (
          <LoadingState label="Searching logs..." />
        ) : resultsQuery.error && !resultsQuery.data ? (
          <ErrorState error={resultsQuery.error} retry={() => void resultsQuery.refetch()} />
        ) : (
          <div className="space-y-4">
            <DataTable
              columns={[
                "Timestamp",
                "Severity",
                "Host",
                "Service",
                "Agent ID",
                "Message",
              ]}
              isEmpty={(resultsQuery.data?.items.length ?? 0) === 0}
              rows={(resultsQuery.data?.items ?? []).map((log) => (
                <TableRow key={`${log.timestamp}-${log.message}`}>
                  <TableCell>{formatDateTime(log.timestamp)}</TableCell>
                  <TableCell>
                    <StatusBadge value={log.severity} />
                  </TableCell>
                  <TableCell>{log.host ?? "n/a"}</TableCell>
                  <TableCell>{log.service ?? "n/a"}</TableCell>
                  <TableCell className="font-mono text-xs text-[color:var(--muted-foreground)]">
                    {log.agentId ?? "n/a"}
                  </TableCell>
                  <TableCell className="max-w-3xl text-[color:var(--foreground)]">
                    {log.message}
                  </TableCell>
                </TableRow>
              ))}
              emptyTitle="No log records match the current filters."
              emptyDescription="Broaden the search query or time range and try again."
            />

            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <RequestMetaLine meta={resultsQuery.meta} />
              <CursorPagination
                hasPrevious={cursorHistory.length > 0}
                hasNext={Boolean(resultsQuery.data?.nextCursor)}
                onPrevious={() => {
                  const nextHistory = cursorHistory.slice(0, -1);
                  setCursorHistoryState({ key: filterKey, values: nextHistory });
                  replaceFilters(
                    { cursor: nextHistory[nextHistory.length - 1] || undefined },
                    { resetCursor: false }
                  );
                }}
                onNext={() => {
                  const nextCursor = resultsQuery.data?.nextCursor;
                  if (!nextCursor) {
                    return;
                  }

                  setCursorHistoryState({
                    key: filterKey,
                    values: [...cursorHistory, urlFilters.cursor ?? ""],
                  });
                  replaceFilters({ cursor: nextCursor }, { resetCursor: false });
                }}
              />
            </div>
          </div>
        )}
      </SectionCard>
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
    <SectionCard
      title="Filters"
      description="All filters stay in the URL so the current search can be refreshed or shared."
      action={
        <div className="flex flex-wrap gap-2">
          <Button
            variant="outline"
            size="sm"
            className="h-10 px-4"
            onClick={onReset}
          >
            Reset
          </Button>
          <Button
            size="sm"
            className="h-10 px-4"
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
            Apply
          </Button>
        </div>
      }
    >
      <FilterGrid>
        <SearchField
          label="Query"
          value={query}
          onChange={setQuery}
          placeholder="timeout OR error"
        />
        <TextField
          label="From"
          value={from}
          onChange={setFrom}
          type="datetime-local"
        />
        <TextField
          label="To"
          value={to}
          onChange={setTo}
          type="datetime-local"
        />
        <TextField label="Host" value={host} onChange={setHost} />
        <TextField label="Service" value={service} onChange={setService} />
        <SelectField
          label="Severity"
          value={severity}
          onChange={setSeverity}
          options={SEVERITY_OPTIONS}
        />
        <TextField label="Agent ID" value={agentId} onChange={setAgentId} />
      </FilterGrid>
    </SectionCard>
  );
}
