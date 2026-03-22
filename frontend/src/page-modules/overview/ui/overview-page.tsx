"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import type { Locale } from "@/src/shared/config";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
import {
  getDashboardOverview,
  listAgents,
  listAlerts,
  listDeployments,
  listLogAnomalies,
  type AgentItem,
  type AlertInstanceItem,
  type DashboardOverviewResponse,
  type DeploymentJobItem,
  type LogAnomalyItem,
} from "@/src/shared/lib/runtime-api";
import { Badge, Button, Card, EmptyState } from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import {
  ErrorCard,
  LoadingCard,
} from "@/src/page-modules/common/ui/runtime-state";

type OverviewState = {
  dashboard: DashboardOverviewResponse;
  alerts: AlertInstanceItem[];
  anomalies: LogAnomalyItem[];
  deployments: DeploymentJobItem[];
  agents: AgentItem[];
};

function isOpenAlertStatus(value?: string) {
  const normalized = value?.trim().toLowerCase() ?? "";
  return !["resolved", "closed", "delivered"].includes(normalized);
}

function isHealthyAgentStatus(value?: string) {
  const normalized = value?.trim().toLowerCase() ?? "";
  return ["online", "healthy", "ready"].includes(normalized);
}

function toBadgeVariant(value?: string) {
  const normalized = value?.trim().toLowerCase() ?? "";

  if (
    ["critical", "fatal", "high", "error", "failed", "offline"].includes(
      normalized
    )
  ) {
    return "danger" as const;
  }

  if (["warning", "warn", "running", "degraded"].includes(normalized)) {
    return "warning" as const;
  }

  if (
    ["healthy", "online", "ready", "resolved", "success", "succeeded"].includes(
      normalized
    )
  ) {
    return "success" as const;
  }

  return "default" as const;
}

export function OverviewPage() {
  const { dictionary, locale } = useI18n();
  const [data, setData] = useState<OverviewState | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);

      try {
        const [
          dashboardResponse,
          alertsResponse,
          anomaliesResponse,
          deploymentsResponse,
          agentsResponse,
        ] = await Promise.all([
          getDashboardOverview(),
          listAlerts({ limit: 20, offset: 0 }),
          listLogAnomalies({ limit: 10, offset: 0 }),
          listDeployments(),
          listAgents(),
        ]);

        if (!cancelled) {
          setData({
            dashboard: dashboardResponse,
            alerts: alertsResponse.items,
            anomalies: anomaliesResponse.items,
            deployments: deploymentsResponse.items,
            agents: agentsResponse.items,
          });
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

  const openAlerts = data?.alerts.filter((item) => isOpenAlertStatus(item.status)) ?? [];
  const criticalAlerts = openAlerts
    .filter((item) =>
      ["critical", "high", "error", "fatal"].includes(item.severity.toLowerCase())
    )
    .slice(0, 5);
  const healthyAgents =
    data?.agents.filter((item) => isHealthyAgentStatus(item.status)) ?? [];
  const degradedAgents =
    data?.agents.filter((item) => !isHealthyAgentStatus(item.status)) ?? [];

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="space-y-6">
          <div className="border-b border-[color:var(--border)] pb-6">
            <h2 className="text-5xl font-semibold text-[color:var(--foreground)]">
              overview workspace
            </h2>
          </div>

          {loading ? <LoadingCard label="Loading overview..." /> : null}
          {!loading && error ? <ErrorCard message={error} /> : null}

          {!loading && !error && data ? (
            <>
              <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-5">
                <OverviewMetricCard
                  label="Open alerts"
                  value={String(openAlerts.length)}
                  hint="Security pressure right now"
                />
                <OverviewMetricCard
                  label="Recent anomalies"
                  value={String(data.anomalies.length)}
                  hint="Latest correlated detections"
                />
                <OverviewMetricCard
                  label="Deployments"
                  value={String(data.deployments.length)}
                  hint="Known rollout jobs"
                />
                <OverviewMetricCard
                  label="Agent health"
                  value={`${healthyAgents.length}/${data.agents.length}`}
                  hint="Healthy coverage"
                />
                <OverviewMetricCard
                  label="Ingested events"
                  value={String(data.dashboard.ingested_events)}
                  hint={`Active hosts: ${data.dashboard.active_hosts}`}
                />
              </section>

              <section className="grid gap-4 xl:grid-cols-[minmax(0,1.05fr)_minmax(0,0.95fr)]">
                <section className="space-y-4 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <div>
                      <h2 className="text-xl font-semibold text-[color:var(--foreground)]">
                        Critical alerts
                      </h2>
                      <p className="text-base text-[color:var(--muted-foreground)]">
                        Open high-severity signals that usually need the fastest
                        response.
                      </p>
                    </div>

                    <Link href={withLocalePath(locale, "/security?tab=alerts")}>
                      <Button variant="outline" size="sm" className="h-10 px-4">
                        Open Security
                      </Button>
                    </Link>
                  </div>

                  {criticalAlerts.length === 0 ? (
                    <EmptyState
                      variant="flush"
                      title="No critical alerts"
                      description="High-severity open alerts are not currently piling up."
                    />
                  ) : (
                    <div className="space-y-3">
                      {criticalAlerts.map((alert) => (
                        <div
                          key={alert.alert_instance_id}
                          className="rounded-lg border border-[color:var(--border)] bg-[color:var(--background)] p-4"
                        >
                          <div className="flex flex-wrap items-center gap-2">
                            <Badge variant={toBadgeVariant(alert.severity)}>
                              {alert.severity}
                            </Badge>
                            <Badge>{alert.status}</Badge>
                          </div>
                          <p className="mt-3 text-base font-semibold text-[color:var(--foreground)]">
                            {alert.title}
                          </p>
                          <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                            {alert.host || "n/a"} / {alert.service || "n/a"} /{" "}
                            {alert.triggered_at}
                          </p>
                        </div>
                      ))}
                    </div>
                  )}
                </section>

                <QuickLinksPanel locale={locale} />
              </section>

              <section className="grid gap-4 xl:grid-cols-3">
                <section className="space-y-4 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
                  <div>
                    <h2 className="text-xl font-semibold text-[color:var(--foreground)]">
                      Anomalies summary
                    </h2>
                    <p className="text-base text-[color:var(--muted-foreground)]">
                      Recent anomaly instances and their current state.
                    </p>
                  </div>

                  {data.anomalies.length === 0 ? (
                    <EmptyState
                      variant="flush"
                      title="No anomalies"
                      description="No recent anomaly instances were returned."
                    />
                  ) : (
                    <div className="space-y-3">
                      {data.anomalies.slice(0, 5).map((item) => (
                        <div
                          key={item.alert_instance_id}
                          className="rounded-lg border border-[color:var(--border)] bg-[color:var(--background)] p-4"
                        >
                          <div className="flex flex-wrap items-center gap-2">
                            <Badge variant={toBadgeVariant(item.severity)}>
                              {item.severity}
                            </Badge>
                            <Badge>{item.status}</Badge>
                          </div>
                          <p className="mt-3 text-sm font-semibold text-[color:var(--foreground)]">
                            {item.title}
                          </p>
                          <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                            {item.host} / {item.service}
                          </p>
                        </div>
                      ))}
                    </div>
                  )}
                </section>

                <section className="space-y-4 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
                  <div>
                    <h2 className="text-xl font-semibold text-[color:var(--foreground)]">
                      Deployments summary
                    </h2>
                    <p className="text-base text-[color:var(--muted-foreground)]">
                      Latest rollout jobs and their current phases.
                    </p>
                  </div>

                  {data.deployments.length === 0 ? (
                    <EmptyState
                      variant="flush"
                      title="No deployments"
                      description="No deployment jobs were returned."
                    />
                  ) : (
                    <div className="space-y-3">
                      {data.deployments.slice(0, 5).map((job) => (
                        <div
                          key={job.job_id}
                          className="rounded-lg border border-[color:var(--border)] bg-[color:var(--background)] p-4"
                        >
                          <div className="flex flex-wrap items-center gap-2">
                            <Badge variant={toBadgeVariant(job.status)}>
                              {job.status}
                            </Badge>
                            <Badge>{job.current_phase || "phase:n/a"}</Badge>
                          </div>
                          <p className="mt-3 text-sm font-semibold text-[color:var(--foreground)]">
                            {job.job_type}
                          </p>
                          <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                            Targets: {job.total_targets} / Executor:{" "}
                            {job.executor_kind}
                          </p>
                        </div>
                      ))}
                    </div>
                  )}
                </section>

                <section className="space-y-4 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
                  <div>
                    <h2 className="text-xl font-semibold text-[color:var(--foreground)]">
                      Agent health
                    </h2>
                    <p className="text-base text-[color:var(--muted-foreground)]">
                      Degraded or stale agents that may reduce visibility.
                    </p>
                  </div>

                  {degradedAgents.length === 0 ? (
                    <EmptyState
                      variant="flush"
                      title="Agent fleet looks healthy"
                      description="No degraded agents were detected in the latest registry snapshot."
                    />
                  ) : (
                    <div className="space-y-3">
                      {degradedAgents.slice(0, 5).map((agent) => (
                        <div
                          key={agent.agent_id}
                          className="rounded-lg border border-[color:var(--border)] bg-[color:var(--background)] p-4"
                        >
                          <div className="flex flex-wrap items-center gap-2">
                            <Badge variant={toBadgeVariant(agent.status)}>
                              {agent.status}
                            </Badge>
                            <Badge>{agent.version || "unknown"}</Badge>
                          </div>
                          <p className="mt-3 text-sm font-semibold text-[color:var(--foreground)]">
                            {agent.hostname}
                          </p>
                          <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                            Last seen {agent.last_seen_at}
                          </p>
                        </div>
                      ))}
                    </div>
                  )}
                </section>
              </section>
            </>
          ) : null}
        </div>
      </Card>
    </div>
  );
}

function OverviewMetricCard({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint: string;
}) {
  return (
    <section className="space-y-2 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
      <p className="text-sm text-[color:var(--muted-foreground)]">{label}</p>
      <p className="text-3xl font-semibold text-[color:var(--foreground)]">
        {value}
      </p>
      <p className="text-sm text-[color:var(--muted-foreground)]">{hint}</p>
    </section>
  );
}

function QuickLinksPanel({ locale }: { locale: Locale }) {
  const links = [
    {
      title: "Infrastructure",
      description: "Resources, agents, and access",
      href: withLocalePath(locale, "/infrastructure"),
    },
    {
      title: "Security",
      description: "Alerts, findings, policies, anomalies",
      href: withLocalePath(locale, "/security"),
    },
    {
      title: "Operations",
      description: "Deployments and log history",
      href: withLocalePath(locale, "/operations"),
    },
    {
      title: "Live Logs",
      description: "Open the streaming log view",
      href: withLocalePath(locale, "/logs/live"),
    },
    {
      title: "Integrations",
      description: "Telegram instances and bindings",
      href: withLocalePath(locale, "/integrations"),
    },
    {
      title: "Audit",
      description: "Recent state-changing events",
      href: withLocalePath(locale, "/audit"),
    },
  ];

  return (
    <section className="space-y-4 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
      <div>
        <h2 className="text-xl font-semibold text-[color:var(--foreground)]">
          Quick links
        </h2>
        <p className="text-base text-[color:var(--muted-foreground)]">
          Shortcuts into the new larger sections.
        </p>
      </div>

      <div className="grid gap-3 sm:grid-cols-2">
        {links.map((item) => (
          <div
            key={item.title}
            className="space-y-3 rounded-lg border border-[color:var(--border)] bg-[color:var(--background)] p-4"
          >
            <div className="space-y-1">
              <p className="text-base font-semibold text-[color:var(--foreground)]">
                {item.title}
              </p>
              <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
                {item.description}
              </p>
            </div>
            <Link href={item.href}>
              <Button variant="outline" size="sm" className="h-10 px-4">
                Open
              </Button>
            </Link>
          </div>
        ))}
      </div>
    </section>
  );
}