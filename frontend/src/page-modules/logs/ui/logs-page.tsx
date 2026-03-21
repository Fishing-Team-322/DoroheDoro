"use client";

import { FormEvent, useEffect, useState } from "react";
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
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { ErrorCard, LoadingCard } from "@/src/page-modules/common/ui/runtime-state";

export function LogsPage() {
  const { dictionary } = useI18n();
  const [query, setQuery] = useState("");
  const [items, setItems] = useState<LogEventItem[]>([]);
  const [anomalies, setAnomalies] = useState<LogAnomalyItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function runSearch(searchQuery: string) {
    setLoading(true);
    setError(null);
    try {
      const [logsResponse, anomaliesResponse] = await Promise.all([
        searchLogs({ query: searchQuery, limit: 20, offset: 0 }),
        listLogAnomalies({ limit: 10, offset: 0 }),
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
    void runSearch("");
  }, []);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    void runSearch(query.trim());
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title="Logs"
        description="Live search over normalized log events plus log-origin anomaly projections."
        breadcrumbs={[
          { label: dictionary.common.dashboard, href: "#" },
          { label: "Logs" },
        ]}
      />

      <Card>
        <form className="flex flex-col gap-3 md:flex-row" onSubmit={handleSubmit}>
          <Input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            label="Search query"
            placeholder="service:error OR host:web-1"
          />
          <div className="flex items-end">
            <Button type="submit" variant="outline">
              Search
            </Button>
          </div>
        </form>
      </Card>

      {loading ? <LoadingCard label="Searching logs..." /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error ? (
        <section className="grid gap-4 xl:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)]">
          <Card>
            <div className="space-y-3">
              <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                Search results
              </h2>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Timestamp</TableHead>
                    <TableHead>Host</TableHead>
                    <TableHead>Service</TableHead>
                    <TableHead>Severity</TableHead>
                    <TableHead>Message</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {items.length === 0 ? (
                    <TableRow>
                      <TableCell colSpan={5}>
                        <EmptyState
                          variant="flush"
                          title="No matching logs"
                          description="Adjust the query or ingest new events."
                        />
                      </TableCell>
                    </TableRow>
                  ) : (
                    items.map((item) => (
                      <TableRow key={item.id}>
                        <TableCell>{item.timestamp}</TableCell>
                        <TableCell>{item.host}</TableCell>
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
                Log-origin anomalies
              </h2>
              {anomalies.length === 0 ? (
                <EmptyState
                  variant="flush"
                  title="No anomalies"
                  description="Triggered alert instances backed by log rules will appear here."
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
                      {item.host} · {item.service} · {item.severity}
                    </p>
                    <p className="mt-1 text-xs uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                      {item.status} · {item.triggered_at}
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
