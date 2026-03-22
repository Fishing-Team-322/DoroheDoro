"use client";

import { useEffect, useState } from "react";
import { useI18n } from "@/src/shared/lib/i18n";
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
import { Badge, Card, EmptyState } from "@/src/shared/ui";
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
  useI18n();
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
            loadError instanceof Error
              ? loadError.message
              : "Failed to load overview"
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

  const openAlerts =
    data?.alerts.filter((item) => isOpenAlertStatus(item.status)) ?? [];
  const criticalAlerts = openAlerts
    .filter((item) =>
      ["critical", "high", "error", "fatal"].includes(
        item.severity.toLowerCase()
      )
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

              <section className="space-y-4 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
                <div className="space-y-1">
                  <h2 className="text-xl font-semibold text-[color:var(--foreground)]">
                    Critical alerts
                  </h2>
                  <p className="text-base text-[color:var(--muted-foreground)]">
                    Open high-severity signals that usually need the fastest
                    response.
                  </p>
                </div>

                {criticalAlerts.length === 0 ? (
                  <EmptyState
                    variant="flush"
                    title="No critical alerts"
                    description="High-severity open alerts are not currently piling up."
                  />
                ) : (
                  <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
                    {criticalAlerts.map((alert) => (
                      <div
                        key={alert.alert_instance_id}
                        className="rounded-xl border border-[color:var(--border)] bg-[color:var(--background)] p-4"
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

              <section className="grid gap-4 xl:grid-cols-3">
                <SummaryPanel
                  title="Anomalies summary"
                  description="Recent anomaly instances and their current state."
                  emptyTitle="No anomalies"
                  emptyDescription="No recent anomaly instances were returned."
                >
                  {data.anomalies.length === 0 ? null : (
                    <div className="space-y-3">
                      {data.anomalies.slice(0, 5).map((item) => (
                        <SummaryItemCard
                          key={item.alert_instance_id}
                          badges={[
                            {
                              label: item.severity,
                              variant: toBadgeVariant(item.severity),
                            },
                            { label: item.status },
                          ]}
                          title={item.title}
                          description={`${item.host} / ${item.service}`}
                        />
                      ))}
                    </div>
                  )}
                </SummaryPanel>

                <SummaryPanel
                  title="Deployments summary"
                  description="Latest rollout jobs and their current phases."
                  emptyTitle="No deployments"
                  emptyDescription="No deployment jobs were returned."
                >
                  {data.deployments.length === 0 ? null : (
                    <div className="space-y-3">
                      {data.deployments.slice(0, 5).map((job) => (
                        <SummaryItemCard
                          key={job.job_id}
                          badges={[
                            {
                              label: job.status,
                              variant: toBadgeVariant(job.status),
                            },
                            { label: job.current_phase || "phase:n/a" },
                          ]}
                          title={job.job_type}
                          description={`Targets: ${job.total_targets} / Executor: ${job.executor_kind}`}
                        />
                      ))}
                    </div>
                  )}
                </SummaryPanel>

                <SummaryPanel
                  title="Agent health"
                  description="Degraded or stale agents that may reduce visibility."
                  emptyTitle="Agent fleet looks healthy"
                  emptyDescription="No degraded agents were detected in the latest registry snapshot."
                >
                  {degradedAgents.length === 0 ? null : (
                    <div className="space-y-3">
                      {degradedAgents.slice(0, 5).map((agent) => (
                        <SummaryItemCard
                          key={agent.agent_id}
                          badges={[
                            {
                              label: agent.status,
                              variant: toBadgeVariant(agent.status),
                            },
                            { label: agent.version || "unknown" },
                          ]}
                          title={agent.hostname}
                          description={`Last seen ${agent.last_seen_at}`}
                        />
                      ))}
                    </div>
                  )}
                </SummaryPanel>
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

function SummaryPanel({
  title,
  description,
  emptyTitle,
  emptyDescription,
  children,
}: {
  title: string;
  description: string;
  emptyTitle: string;
  emptyDescription: string;
  children: React.ReactNode;
}) {
  const hasChildren = Boolean(children);

  return (
    <section className="space-y-4 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
      <div className="space-y-1">
        <h2 className="text-xl font-semibold text-[color:var(--foreground)]">
          {title}
        </h2>
        <p className="text-base text-[color:var(--muted-foreground)]">
          {description}
        </p>
      </div>

      {hasChildren ? (
        children
      ) : (
        <EmptyState
          variant="flush"
          title={emptyTitle}
          description={emptyDescription}
        />
      )}
    </section>
  );
}

function SummaryItemCard({
  badges,
  title,
  description,
}: {
  badges: Array<{
    label: string;
    variant?: "default" | "success" | "warning" | "danger";
  }>;
  title: string;
  description: string;
}) {
  return (
    <div className="rounded-xl border border-[color:var(--border)] bg-[color:var(--background)] p-4">
      <div className="flex flex-wrap items-center gap-2">
        {badges.map((badge) => (
          <Badge
            key={`${badge.label}-${badge.variant ?? "default"}`}
            variant={badge.variant}
          >
            {badge.label}
          </Badge>
        ))}
      </div>

      <p className="mt-3 text-sm font-semibold text-[color:var(--foreground)]">
        {title}
      </p>

      <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
        {description}
      </p>
    </div>
  );
}