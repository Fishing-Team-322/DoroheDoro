"use client";

import type { FormEvent } from "react";
import { useEffect, useState } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useI18n } from "@/src/shared/lib/i18n";
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

const severityOptions = [
  { value: "", label: "Any severity" },
  { value: "debug", label: "debug" },
  { value: "info", label: "info" },
  { value: "warn", label: "warn" },
  { value: "error", label: "error" },
  { value: "fatal", label: "fatal" },
];

type SearchState = {
  query: string;
  host: string;
  service: string;
  agentId: string;
  severity: string;
};

export function LogsPage({ embedded = false }: { embedded?: boolean } = {}) {
  const { dictionary } = useI18n();
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
      setError(loadError instanceof Error ? loadError.message : "Failed to search logs");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    const nextState = {
      query: urlQuery,
      host: urlHost,
      service: urlService,
      agentId: urlAgentId,
      severity: urlSeverity,
    };

    setQuery(urlQuery);
    setHost(urlHost);
    setService(urlService);
    setAgentId(urlAgentId);
    setSeverity(urlSeverity);
    void runSearch(nextState);
  }, [urlAgentId, urlHost, urlQuery, urlService, urlSeverity]);

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
          title="Logs"
          description="Historical log search with quick host, service, agent, and severity filters plus anomaly markers."
          breadcrumbs={[
            { label: dictionary.common.dashboard, href: "#" },
            { label: "Logs" },
          ]}
        />
      ) : null}

      <Card>
        <form className="space-y-4" onSubmit={handleSubmit}>
          <div className="grid gap-4 xl:grid-cols-5">
            <Input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              label="Search query"
              placeholder="service:error OR host:web-1"
            />
            <Input
              value={host}
              onChange={(event) => setHost(event.target.value)}
              label="Host"
              placeholder="web-1"
            />
            <Input
              value={service}
              onChange={(event) => setService(event.target.value)}
              label="Service"
              placeholder="api"
            />
            <Input
              value={agentId}
              onChange={(event) => setAgentId(event.target.value)}
              label="Agent"
              placeholder="agent-123"
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
              Search
            </Button>
            <Button
              type="button"
              variant="ghost"
              onClick={handleReset}
            >
              Reset filters
            </Button>
          </div>
        </form>
      </Card>

      {loading ? <LoadingCard label="Searching logs..." /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error ? (
        <section className="grid gap-4 xl:grid-cols-[minmax(0,1.45fr)_minmax(0,0.95fr)]">
          <Card>
            <div className="space-y-3">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                  Log history
                </h2>
                <p className="text-sm text-[color:var(--muted-foreground)]">
                  {items.length} event(s)
                </p>
              </div>

              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Timestamp</TableHead>
                    <TableHead>Host</TableHead>
                    <TableHead>Agent</TableHead>
                    <TableHead>Service</TableHead>
                    <TableHead>Severity</TableHead>
                    <TableHead>Message</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {items.length === 0 ? (
                    <TableRow>
                      <TableCell colSpan={6}>
                        <EmptyState
                          variant="flush"
                          title="No matching logs"
                          description="Adjust the filters or ingest new events."
                        />
                      </TableCell>
                    </TableRow>
                  ) : (
                    items.map((item) => (
                      <TableRow key={item.id}>
                        <TableCell>{item.timestamp}</TableCell>
                        <TableCell>{item.host}</TableCell>
                        <TableCell>{item.agent_id || "n/a"}</TableCell>
                        <TableCell>{item.service}</TableCell>
                        <TableCell>{item.severity}</TableCell>
                        <TableCell className="max-w-[420px] truncate">
                          {item.message}
                        </TableCell>
                      </TableRow>
                    ))
                  )}
                </TableBody>
              </Table>
            </div>
          </Card>

          <Card>
            <div className="space-y-3">
              <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                Anomaly markers
              </h2>
              {anomalies.length === 0 ? (
                <EmptyState
                  variant="flush"
                  title="No anomaly markers"
                  description="Triggered log-origin anomalies matching the current filters will appear here."
                />
              ) : (
                anomalies.map((item) => (
                  <div
                    key={item.alert_instance_id}
                    className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3"
                  >
                    <p className="text-sm font-medium text-[color:var(--foreground)]">
                      {item.title}
                    </p>
                    <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                      {item.host} / {item.service} / {item.severity}
                    </p>
                    <p className="mt-1 text-xs uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                      {item.status} / {item.triggered_at}
                    </p>
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
