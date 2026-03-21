"use client";

import { useEffect, useState } from "react";
import { useI18n } from "@/src/shared/lib/i18n";
import { formatRelativeLabel } from "@/src/shared/lib/dashboard";
import { getDashboardOverview, type DashboardOverviewResponse } from "@/src/shared/lib/runtime-api";
import { Card, EmptyState, Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { ErrorCard, LoadingCard } from "@/src/page-modules/common/ui/runtime-state";

export function OverviewPage() {
  const { dictionary, locale } = useI18n();
  const [data, setData] = useState<DashboardOverviewResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);
      try {
        const response = await getDashboardOverview();
        if (!cancelled) {
          setData(response);
        }
      } catch (loadError) {
        if (!cancelled) {
          setError(
            loadError instanceof Error ? loadError.message : "Failed to load overview"
          );
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void load();
    return () => {
      cancelled = true;
    };
  }, []);

  const histogramMax = Math.max(
    ...(data?.log_histogram.map((item) => item.count) ?? [0]),
    1
  );

  return (
    <div className="space-y-6">
      <PageHeader
        title="Overview"
        description="Live operational summary backed by dashboard, ingest, alert and audit runtime data."
        breadcrumbs={[
          { label: dictionary.common.dashboard, href: "#" },
          { label: "Overview" },
        ]}
      />

      {loading ? <LoadingCard label="Loading overview..." /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error && data ? (
        <>
          <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
            {data.metrics.map((metric) => (
              <Card key={metric.key}>
                <p className="text-sm font-medium text-[color:var(--muted-foreground)]">
                  {metric.label}
                </p>
                <div className="mt-2 text-3xl font-semibold tracking-tight text-[color:var(--foreground)]">
                  {metric.value}
                </div>
                <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                  {metric.description || metric.key}
                </p>
              </Card>
            ))}
          </section>

          <section className="grid gap-4 xl:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)]">
            <Card>
              <div className="space-y-3">
                <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                  Log histogram
                </h2>
                {data.log_histogram.length === 0 ? (
                  <EmptyState
                    variant="flush"
                    title="No log activity yet"
                    description="Run the agent ingest path to populate analytics."
                  />
                ) : (
                  <>
                    <div className="flex h-44 items-end gap-2">
                      {data.log_histogram.map((point) => (
                        <div
                          key={point.bucket}
                          className="flex min-w-0 flex-1 items-end"
                        >
                          <div
                            className="w-full rounded-[2px] bg-gradient-to-t from-emerald-500 to-cyan-300"
                            style={{
                              height: `${Math.max(
                                (point.count / histogramMax) * 100,
                                10
                              )}%`,
                            }}
                            title={`${point.bucket}: ${point.count}`}
                          />
                        </div>
                      ))}
                    </div>

                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>Bucket</TableHead>
                          <TableHead>Count</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {data.log_histogram.slice(-6).map((item) => (
                          <TableRow key={item.bucket}>
                            <TableCell>{item.bucket}</TableCell>
                            <TableCell>{item.count}</TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  </>
                )}
              </div>
            </Card>

            <Card>
              <div className="space-y-3">
                <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                  Recent activity
                </h2>
                {data.recent_activity.length === 0 ? (
                  <EmptyState
                    variant="flush"
                    title="No audit-backed activity yet"
                    description="Recent control, deployment and alert events will appear here."
                  />
                ) : (
                  <div className="space-y-3">
                    {data.recent_activity.map((item, index) => (
                      <div
                        key={`${item.kind}-${item.timestamp}-${index}`}
                        className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3"
                      >
                        <p className="text-sm font-medium text-[color:var(--foreground)]">
                          {item.title}
                        </p>
                        <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                          {item.description}
                        </p>
                        <p className="mt-2 text-xs uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                          {formatRelativeLabel(item.timestamp, locale)}
                        </p>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </Card>
          </section>

          <section className="grid gap-4 lg:grid-cols-2">
            <Card>
              <div className="space-y-3">
                <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                  Top services
                </h2>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Service</TableHead>
                      <TableHead>Count</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {data.top_services.length === 0 ? (
                      <TableRow>
                        <TableCell colSpan={2}>
                          <EmptyState
                            variant="flush"
                            title="No services yet"
                            description="Services will appear after ingest begins."
                          />
                        </TableCell>
                      </TableRow>
                    ) : (
                      data.top_services.map((item) => (
                        <TableRow key={item.key}>
                          <TableCell>{item.key}</TableCell>
                          <TableCell>{item.count}</TableCell>
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
                  Top hosts
                </h2>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Host</TableHead>
                      <TableHead>Count</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {data.top_hosts.length === 0 ? (
                      <TableRow>
                        <TableCell colSpan={2}>
                          <EmptyState
                            variant="flush"
                            title="No hosts yet"
                            description="Hosts will appear after agents start sending logs."
                          />
                        </TableCell>
                      </TableRow>
                    ) : (
                      data.top_hosts.map((item) => (
                        <TableRow key={item.key}>
                          <TableCell>{item.key}</TableCell>
                          <TableCell>{item.count}</TableCell>
                        </TableRow>
                      ))
                    )}
                  </TableBody>
                </Table>
              </div>
            </Card>
          </section>
        </>
      ) : null}
    </div>
  );
}
