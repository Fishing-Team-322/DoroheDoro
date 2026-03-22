"use client";

import { useEffect, useState } from "react";
import { translateValueLabel, useI18n } from "@/src/shared/lib/i18n";
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
import { formatDateTime } from "@/src/features/operations/ui/operations-ui";
import { Badge, Card, EmptyState } from "@/src/shared/ui";
import {
  ErrorCard,
  LoadingCard,
} from "@/src/page-modules/common/ui/runtime-state";

const copyByLocale = {
  en: {
    loadError: "Failed to load overview",
    pageTitle: "Overview workspace",
    loading: "Loading overview...",
    metrics: {
      openAlerts: { label: "Open alerts", hint: "Security pressure right now" },
      anomalies: {
        label: "Recent anomalies",
        hint: "Latest correlated detections",
      },
      deployments: { label: "Deployments", hint: "Known rollout jobs" },
      agentHealth: { label: "Agent health", hint: "Healthy coverage" },
      ingestedEvents: { label: "Ingested events", hintPrefix: "Active hosts:" },
    },
    criticalAlerts: {
      title: "Critical alerts",
      description:
        "Open high-severity signals that usually need the fastest response.",
      emptyTitle: "No critical alerts",
      emptyDescription:
        "High-severity open alerts are not currently piling up.",
    },
    sections: {
      anomalies: {
        title: "Anomalies summary",
        description: "Recent anomaly instances and their current state.",
        emptyTitle: "No anomalies",
        emptyDescription: "No recent anomaly instances were returned.",
      },
      deployments: {
        title: "Deployments summary",
        description: "Latest rollout jobs and their current phases.",
        emptyTitle: "No deployments",
        emptyDescription: "No deployment jobs were returned.",
        phaseFallback: "phase:n/a",
        descriptionPrefix: "Targets",
        descriptionSeparator: "Executor",
      },
      agents: {
        title: "Agent health",
        description:
          "Degraded or stale agents that may reduce visibility.",
        emptyTitle: "Agent fleet looks healthy",
        emptyDescription:
          "No degraded agents were detected in the latest registry snapshot.",
        versionFallback: "unknown",
        lastSeenPrefix: "Last seen",
      },
    },
  },
  ru: {
    loadError: "Не удалось загрузить обзор",
    pageTitle: "обзор",
    loading: "Загрузка обзора...",
    metrics: {
      openAlerts: { label: "Открытые алерты", hint: "Текущее давление по безопасности" },
      anomalies: {
        label: "Недавние аномалии",
        hint: "Последние коррелированные детекты",
      },
      deployments: { label: "Раскатки", hint: "Известные задачи выката" },
      agentHealth: { label: "Состояние агентов", hint: "Покрытие healthy-агентами" },
      ingestedEvents: { label: "Ингестированные события", hintPrefix: "Активные хосты:" },
    },
    criticalAlerts: {
      title: "Критичные алерты",
      description:
        "Открытые high-severity сигналы, которым обычно нужен самый быстрый ответ.",
      emptyTitle: "Нет критичных алертов",
      emptyDescription:
        "Открытые high-severity алерты сейчас не накапливаются.",
    },
    sections: {
      anomalies: {
        title: "Сводка по аномалиям",
        description: "Недавние аномалии и их текущее состояние.",
        emptyTitle: "Нет аномалий",
        emptyDescription: "Недавние аномалии не были возвращены.",
      },
      deployments: {
        title: "Сводка по раскаткам",
        description: "Последние rollout-задачи и их текущие фазы.",
        emptyTitle: "Нет раскаток",
        emptyDescription: "Задачи deployment не были возвращены.",
        phaseFallback: "фаза:н/д",
        descriptionPrefix: "Таргеты",
        descriptionSeparator: "Исполнитель",
      },
      agents: {
        title: "Состояние агентов",
        description:
          "Деградированные или давно не выходившие на связь агенты, которые могут снижать видимость.",
        emptyTitle: "Флот агентов выглядит здоровым",
        emptyDescription:
          "В последнем снимке реестра не найдено деградированных агентов.",
        versionFallback: "неизвестно",
        lastSeenPrefix: "Последний сигнал",
      },
    },
  },
} as const;

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
  const { locale } = useI18n();
  const copy = copyByLocale[locale];
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
              : copy.loadError
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
  }, [copy.loadError]);

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
                {copy.pageTitle}
              </h2>
            </div>

          {loading ? <LoadingCard label={copy.loading} /> : null}
          {!loading && error ? <ErrorCard message={error} /> : null}

          {!loading && !error && data ? (
            <>
              <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-5">
                <OverviewMetricCard
                  label={copy.metrics.openAlerts.label}
                  value={String(openAlerts.length)}
                  hint={copy.metrics.openAlerts.hint}
                />
                <OverviewMetricCard
                  label={copy.metrics.anomalies.label}
                  value={String(data.anomalies.length)}
                  hint={copy.metrics.anomalies.hint}
                />
                <OverviewMetricCard
                  label={copy.metrics.deployments.label}
                  value={String(data.deployments.length)}
                  hint={copy.metrics.deployments.hint}
                />
                <OverviewMetricCard
                  label={copy.metrics.agentHealth.label}
                  value={`${healthyAgents.length}/${data.agents.length}`}
                  hint={copy.metrics.agentHealth.hint}
                />
                <OverviewMetricCard
                  label={copy.metrics.ingestedEvents.label}
                  value={String(data.dashboard.ingested_events)}
                  hint={`${copy.metrics.ingestedEvents.hintPrefix} ${data.dashboard.active_hosts}`}
                />
              </section>

              <section className="space-y-4 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
                <div className="space-y-1">
                  <h2 className="text-xl font-semibold text-[color:var(--foreground)]">
                    {copy.criticalAlerts.title}
                  </h2>
                  <p className="text-base text-[color:var(--muted-foreground)]">
                    {copy.criticalAlerts.description}
                  </p>
                </div>

                {criticalAlerts.length === 0 ? (
                  <EmptyState
                    variant="flush"
                    title={copy.criticalAlerts.emptyTitle}
                    description={copy.criticalAlerts.emptyDescription}
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
                            {translateValueLabel(alert.severity, locale)}
                          </Badge>
                          <Badge>{translateValueLabel(alert.status, locale)}</Badge>
                        </div>

                        <p className="mt-3 text-base font-semibold text-[color:var(--foreground)]">
                          {alert.title}
                        </p>

                        <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                          {alert.host || "n/a"} / {alert.service || "n/a"} /{" "}
                          {formatDateTime(alert.triggered_at, locale)}
                        </p>
                      </div>
                    ))}
                  </div>
                )}
              </section>

              <section className="grid gap-4 xl:grid-cols-3">
                <SummaryPanel
                  title={copy.sections.anomalies.title}
                  description={copy.sections.anomalies.description}
                  emptyTitle={copy.sections.anomalies.emptyTitle}
                  emptyDescription={copy.sections.anomalies.emptyDescription}
                >
                  {data.anomalies.length === 0 ? null : (
                    <div className="space-y-3">
                      {data.anomalies.slice(0, 5).map((item) => (
                        <SummaryItemCard
                          key={item.alert_instance_id}
                          badges={[
                            {
                              label: translateValueLabel(item.severity, locale),
                              variant: toBadgeVariant(item.severity),
                            },
                            { label: translateValueLabel(item.status, locale) },
                          ]}
                          title={item.title}
                          description={`${item.host} / ${item.service}`}
                        />
                      ))}
                    </div>
                  )}
                </SummaryPanel>

                <SummaryPanel
                  title={copy.sections.deployments.title}
                  description={copy.sections.deployments.description}
                  emptyTitle={copy.sections.deployments.emptyTitle}
                  emptyDescription={copy.sections.deployments.emptyDescription}
                >
                  {data.deployments.length === 0 ? null : (
                    <div className="space-y-3">
                      {data.deployments.slice(0, 5).map((job) => (
                        <SummaryItemCard
                          key={job.job_id}
                          badges={[
                            {
                              label: translateValueLabel(job.status, locale),
                              variant: toBadgeVariant(job.status),
                            },
                            {
                              label:
                                job.current_phase || copy.sections.deployments.phaseFallback,
                            },
                          ]}
                          title={job.job_type}
                          description={`${copy.sections.deployments.descriptionPrefix}: ${job.total_targets} / ${copy.sections.deployments.descriptionSeparator}: ${job.executor_kind}`}
                        />
                      ))}
                    </div>
                  )}
                </SummaryPanel>

                <SummaryPanel
                  title={copy.sections.agents.title}
                  description={copy.sections.agents.description}
                  emptyTitle={copy.sections.agents.emptyTitle}
                  emptyDescription={copy.sections.agents.emptyDescription}
                >
                  {degradedAgents.length === 0 ? null : (
                    <div className="space-y-3">
                      {degradedAgents.slice(0, 5).map((agent) => (
                        <SummaryItemCard
                          key={agent.agent_id}
                          badges={[
                            {
                              label: translateValueLabel(agent.status, locale),
                              variant: toBadgeVariant(agent.status),
                            },
                            {
                              label:
                                agent.version || copy.sections.agents.versionFallback,
                            },
                          ]}
                          title={agent.hostname}
                          description={`${copy.sections.agents.lastSeenPrefix} ${formatDateTime(
                            agent.last_seen_at,
                            locale
                          )}`}
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
