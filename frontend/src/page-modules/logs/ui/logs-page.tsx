"use client";

import type { FormEvent } from "react";
import { useEffect, useMemo, useState } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { translateValueLabel, useI18n } from "@/src/shared/lib/i18n";
import { formatDateTime } from "@/src/features/operations/ui/operations-ui";
import {
  listLogAnomalies,
  searchLogs,
  type LogAnomalyItem,
  type LogEventItem,
} from "@/src/shared/lib/runtime-api";
import {
  Button,
  Card,
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
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { ErrorCard, LoadingCard } from "@/src/page-modules/common/ui/runtime-state";

const copyByLocale = {
  en: {
    title: "Logs",
    description:
      "Historical log search with quick host, service, agent, and severity filters plus anomaly markers.",
    filters: {
      query: "Search query",
      queryPlaceholder: "service:error OR host:web-1",
      host: "Host",
      hostPlaceholder: "web-1",
      service: "Service",
      servicePlaceholder: "api",
      agent: "Agent",
      agentPlaceholder: "agent-123",
      search: "Search",
      reset: "Reset filters",
      severityAny: "Any severity",
    },
    loading: "Searching logs...",
    error: "Failed to search logs",
    history: {
      title: "Log history",
      countSuffix: "event(s)",
      columns: {
        timestamp: "Timestamp",
        host: "Host",
        agent: "Agent",
        service: "Service",
        severity: "Severity",
        message: "Message",
      },
      emptyTitle: "No matching logs",
      emptyDescription: "Adjust the filters or ingest new events.",
    },
    anomalies: {
      title: "Anomaly markers",
      emptyTitle: "No anomaly markers",
      emptyDescription:
        "Triggered log-origin anomalies matching the current filters will appear here.",
      triggeredAt: "triggered at",
    },
  },
  ru: {
    title: "Логи",
    description:
      "Исторический поиск по логам с быстрыми фильтрами по host, service, agent и severity, плюс маркеры аномалий.",
    filters: {
      query: "Поисковый запрос",
      queryPlaceholder: "service:error OR host:web-1",
      host: "Хост",
      hostPlaceholder: "web-1",
      service: "Сервис",
      servicePlaceholder: "api",
      agent: "Агент",
      agentPlaceholder: "agent-123",
      search: "Искать",
      reset: "Сбросить фильтры",
      severityAny: "Любая severity",
    },
    loading: "Поиск логов...",
    error: "Не удалось выполнить поиск по логам",
    history: {
      title: "История логов",
      countSuffix: "событ.",
      columns: {
        timestamp: "Время",
        host: "Хост",
        agent: "Агент",
        service: "Сервис",
        severity: "Severity",
        message: "Сообщение",
      },
      emptyTitle: "Подходящих логов нет",
      emptyDescription: "Ослабьте фильтры или дождитесь новых событий.",
    },
    anomalies: {
      title: "Маркеры аномалий",
      emptyTitle: "Маркеров аномалий нет",
      emptyDescription:
        "Здесь появятся логовые аномалии, совпадающие с текущими фильтрами.",
      triggeredAt: "сработало в",
    },
  },
} as const;

type SearchState = {
  query: string;
  host: string;
  service: string;
  agentId: string;
  severity: string;
};

function getSeverityClasses(severity: LogEventItem["severity"] | LogAnomalyItem["severity"]) {
  switch (severity) {
    case "debug":
      return "bg-slate-400/8 text-slate-500 border-slate-400/10";
    case "info":
      return "bg-sky-400/8 text-sky-500 border-sky-400/10";
    case "warn":
      return "bg-amber-400/8 text-amber-500 border-amber-400/10";
    case "error":
      return "bg-rose-400/8 text-rose-500 border-rose-400/10";
    case "fatal":
      return "bg-red-400/8 text-red-500 border-red-400/10";
    default:
      return "bg-slate-400/8 text-slate-500 border-slate-400/10";
  }
}

function getStatusClasses(status: LogAnomalyItem["status"]) {
  switch (status) {
    case "open":
      return "bg-rose-400/8 text-rose-500 border-rose-400/10";
    case "acknowledged":
      return "bg-amber-400/8 text-amber-500 border-amber-400/10";
    case "resolved":
      return "bg-emerald-400/8 text-emerald-500 border-emerald-400/10";
    default:
      return "bg-slate-400/8 text-slate-500 border-slate-400/10";
  }
}

function SeverityBadge({
  locale,
  severity,
}: {
  locale: "ru" | "en";
  severity: LogEventItem["severity"] | LogAnomalyItem["severity"];
}) {
  return (
    <span
      className={[
        "inline-flex items-center rounded-md border px-2 py-1 text-xs font-normal capitalize",
        getSeverityClasses(severity),
      ].join(" ")}
    >
      {translateValueLabel(severity, locale)}
    </span>
  );
}

function StatusBadge({
  locale,
  status,
}: {
  locale: "ru" | "en";
  status: LogAnomalyItem["status"];
}) {
  return (
    <span
      className={[
        "inline-flex items-center rounded-md border px-2 py-1 text-xs font-normal capitalize",
        getStatusClasses(status),
      ].join(" ")}
    >
      {translateValueLabel(status, locale)}
    </span>
  );
}

export function LogsPage({ embedded = false }: { embedded?: boolean } = {}) {
  const { dictionary, locale } = useI18n();
  const copy = copyByLocale[locale];
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();

  const urlQuery = searchParams.get("query") ?? "";
  const urlHost = searchParams.get("host") ?? "";
  const urlService = searchParams.get("service") ?? "";
  const urlAgentId = searchParams.get("agentId") ?? "";
  const urlSeverity = searchParams.get("severity") ?? "";

  const [query, setQuery] = useState(urlQuery);
  const [host, setHost] = useState(urlHost);
  const [service, setService] = useState(urlService);
  const [agentId, setAgentId] = useState(urlAgentId);
  const [severity, setSeverity] = useState(urlSeverity);

  const [items, setItems] = useState<LogEventItem[]>([]);
  const [anomalies, setAnomalies] = useState<LogAnomalyItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const severityOptions = useMemo(
    () => [
      { value: "", label: copy.filters.severityAny },
      { value: "debug", label: "debug" },
      { value: "info", label: "info" },
      { value: "warn", label: "warn" },
      { value: "error", label: "error" },
      { value: "fatal", label: "fatal" },
    ],
    [copy.filters.severityAny]
  );

  const currentState = useMemo<SearchState>(
    () => ({
      query: urlQuery,
      host: urlHost,
      service: urlService,
      agentId: urlAgentId,
      severity: urlSeverity,
    }),
    [urlAgentId, urlHost, urlQuery, urlService, urlSeverity],
  );

  async function runSearch(state: SearchState) {
    setLoading(true);
    setError(null);

    try {
      const [logsResponse, anomaliesResponse] = await Promise.all([
        searchLogs({
          query: state.query,
          host: state.host,
          service: state.service,
          severity: state.severity,
          agentId: state.agentId,
          limit: 50,
          offset: 0,
        }),
        listLogAnomalies({
          host: state.host,
          service: state.service,
          severity: state.severity,
          limit: 10,
          offset: 0,
        }),
      ]);

      setItems(logsResponse.items);
      setAnomalies(anomaliesResponse.items);
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : copy.error);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    setQuery(urlQuery);
    setHost(urlHost);
    setService(urlService);
    setAgentId(urlAgentId);
    setSeverity(urlSeverity);

    void runSearch(currentState);
  }, [copy.error, currentState, urlAgentId, urlHost, urlQuery, urlService, urlSeverity]);

  function updateUrl(state: SearchState) {
    const nextParams = new URLSearchParams(searchParams.toString());
    const tab = searchParams.get("tab");

    if (tab) {
      nextParams.set("tab", tab);
    }

    [
      ["query", state.query],
      ["host", state.host],
      ["service", state.service],
      ["agentId", state.agentId],
      ["severity", state.severity],
    ].forEach(([key, value]) => {
      if (value) {
        nextParams.set(key, value);
      } else {
        nextParams.delete(key);
      }
    });

    const serialized = nextParams.toString();

    router.replace(serialized ? `${pathname}?${serialized}` : pathname, {
      scroll: false,
    });
  }

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    updateUrl({
      query: query.trim(),
      host: host.trim(),
      service: service.trim(),
      agentId: agentId.trim(),
      severity,
    });
  }

  function handleReset() {
    updateUrl({
      query: "",
      host: "",
      service: "",
      agentId: "",
      severity: "",
    });
  }

  return (
    <div className={embedded ? "space-y-4" : "space-y-6"}>
      {!embedded ? (
        <PageHeader
          title={copy.title}
          description={copy.description}
          breadcrumbs={[
            { label: dictionary.common.dashboard, href: "#" },
            { label: copy.title },
          ]}
        />
      ) : null}

      <Card>
        <form className="space-y-4" onSubmit={handleSubmit}>
          <div className="grid gap-4 xl:grid-cols-5">
            <Input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              label={copy.filters.query}
              placeholder={copy.filters.queryPlaceholder}
            />
            <Input
              value={host}
              onChange={(event) => setHost(event.target.value)}
              label={copy.filters.host}
              placeholder={copy.filters.hostPlaceholder}
            />
            <Input
              value={service}
              onChange={(event) => setService(event.target.value)}
              label={copy.filters.service}
              placeholder={copy.filters.servicePlaceholder}
            />
            <Input
              value={agentId}
              onChange={(event) => setAgentId(event.target.value)}
              label={copy.filters.agent}
              placeholder={copy.filters.agentPlaceholder}
            />
            <div className="space-y-2">
              <Select
                value={severity}
                onChange={(event) => setSeverity(event.target.value)}
                options={severityOptions}
              />
            </div>
          </div>

          <div className="flex flex-wrap gap-3">
            <Button type="submit" className="h-10 px-4">
              {copy.filters.search}
            </Button>
            <Button type="button" variant="ghost" onClick={handleReset}>
              {copy.filters.reset}
            </Button>
          </div>
        </form>
      </Card>

      {loading ? <LoadingCard label={copy.loading} /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error ? (
        <section className="grid gap-4 xl:grid-cols-[minmax(0,1.45fr)_minmax(0,0.95fr)]">
          <Card className="min-h-0">
            <div className="flex h-full min-h-0 flex-col space-y-3">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                  {copy.history.title}
                </h2>
                <p className="text-sm text-[color:var(--muted-foreground)]">
                  {items.length} {copy.history.countSuffix}
                </p>
              </div>

              <div className="min-h-0 flex-1 overflow-auto rounded-md border border-[color:var(--border)]">
                <Table>
                  <TableHeader className="sticky top-0 z-10 bg-[color:var(--background)]">
                    <TableRow>
                      <TableHead>{copy.history.columns.timestamp}</TableHead>
                      <TableHead>{copy.history.columns.host}</TableHead>
                      <TableHead>{copy.history.columns.agent}</TableHead>
                      <TableHead>{copy.history.columns.service}</TableHead>
                      <TableHead>{copy.history.columns.severity}</TableHead>
                      <TableHead>{copy.history.columns.message}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {items.length === 0 ? (
                      <TableRow>
                        <TableCell colSpan={6}>
                          <EmptyState
                            variant="flush"
                            title={copy.history.emptyTitle}
                            description={copy.history.emptyDescription}
                          />
                        </TableCell>
                      </TableRow>
                    ) : (
                      items.map((item) => (
                        <TableRow key={item.id}>
                          <TableCell className="whitespace-nowrap text-xs text-[color:var(--muted-foreground)]">
                            {formatDateTime(item.timestamp, locale)}
                          </TableCell>
                          <TableCell className="whitespace-nowrap font-medium">
                            {item.host}
                          </TableCell>
                          <TableCell className="whitespace-nowrap text-[color:var(--muted-foreground)]">
                            {item.agent_id || "n/a"}
                          </TableCell>
                          <TableCell className="whitespace-nowrap">
                            {item.service}
                          </TableCell>
                          <TableCell>
                            <SeverityBadge locale={locale} severity={item.severity} />
                          </TableCell>
                          <TableCell className="max-w-[460px]">
                            <div className="line-clamp-2 text-sm text-[color:var(--foreground)]">
                              {item.message}
                            </div>
                          </TableCell>
                        </TableRow>
                      ))
                    )}
                  </TableBody>
                </Table>
              </div>
            </div>
          </Card>

          <Card>
            <div className="space-y-3">
              <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                {copy.anomalies.title}
              </h2>

              {anomalies.length === 0 ? (
                <EmptyState
                  variant="flush"
                  title={copy.anomalies.emptyTitle}
                  description={copy.anomalies.emptyDescription}
                />
              ) : (
                anomalies.map((item) => (
                  <div
                    key={item.alert_instance_id}
                    className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-4"
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0">
                        <p className="text-sm font-semibold text-[color:var(--foreground)]">
                          {item.title}
                        </p>
                        <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                          {item.host} / {item.service}
                        </p>
                      </div>

                      <SeverityBadge locale={locale} severity={item.severity} />
                    </div>

                    <div className="mt-3 flex flex-wrap items-center gap-2">
                      <StatusBadge locale={locale} status={item.status} />
                      <span className="text-xs uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                        {copy.anomalies.triggeredAt}{" "}
                        {formatDateTime(item.triggered_at, locale)}
                      </span>
                    </div>
                  </div>
                ))
              )}
            </div>
          </Card>
        </section>
      ) : null}
    </div>
  );
}
