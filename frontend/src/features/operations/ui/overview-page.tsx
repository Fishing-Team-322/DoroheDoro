"use client";

import type { ReactNode } from "react";
import Link from "next/link";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
import { Card } from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import {
  getHealthStatus,
  getLogsHistogram,
  getLogsSeverity,
  getLogsTopHosts,
  getLogsTopServices,
  getReadinessStatus,
  listAgents,
  listDeployments,
  listPolicies,
} from "../api";
import { useApiQuery } from "../model";
import {
  ErrorState,
  HistogramBars,
  LoadingState,
  MetricCard,
  NamedCountList,
  PageStack,
  RequestMetaLine,
  SectionCard,
  StatusBadge,
  formatDateTime,
  formatNumber,
  formatParamsSummary,
} from "./operations-ui";

export function OverviewPage() {
  const { locale } = useI18n();
  const healthQuery = useApiQuery({ queryFn: getHealthStatus, deps: [] });
  const readinessQuery = useApiQuery({ queryFn: getReadinessStatus, deps: [] });
  const agentsQuery = useApiQuery({
    queryFn: (signal) => listAgents({ signal }),
    deps: [],
  });
  const policiesQuery = useApiQuery({
    queryFn: (signal) => listPolicies({ signal }),
    deps: [],
  });
  const deploymentsQuery = useApiQuery({
    queryFn: (signal) => listDeployments({ signal }),
    deps: [],
  });
  const histogramQuery = useApiQuery({
    queryFn: (signal) => getLogsHistogram({}, signal, "15m"),
    deps: [],
  });
  const severityQuery = useApiQuery({
    queryFn: (signal) => getLogsSeverity({}, signal, 5),
    deps: [],
  });
  const topHostsQuery = useApiQuery({
    queryFn: (signal) => getLogsTopHosts({}, signal, 5),
    deps: [],
  });
  const topServicesQuery = useApiQuery({
    queryFn: (signal) => getLogsTopServices({}, signal, 5),
    deps: [],
  });

  const onlineAgents =
    agentsQuery.data?.items.filter((item) =>
      ["online", "healthy"].includes(item.status.toLowerCase())
    ).length ?? 0;

  const recentDeployments = deploymentsQuery.data?.items.slice(0, 5) ?? [];

  return (
    <PageStack>
      <PageHeader
        title="Overview"
        description="Operational snapshot across health, readiness, agents, policies, deployments, and log analytics. Every block fetches independently so one backend error does not blank the whole page."
      />

      <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-5">
        <MetricSlot
          title="HTTP Status"
          query={healthQuery}
          render={() => (
            <MetricCard
              label="Public health probe"
              value={healthQuery.data?.status ?? "unknown"}
              status={healthQuery.data?.status}
              hint={
                healthQuery.meta?.requestId
                  ? `req ${healthQuery.meta.requestId}`
                  : undefined
              }
            />
          )}
        />
        <MetricSlot
          title="Bridge Readiness"
          query={readinessQuery}
          render={() => (
            <MetricCard
              label="Ready state"
              value={readinessQuery.data?.status ?? "unknown"}
              status={readinessQuery.data?.status}
              hint={
                readinessQuery.meta?.requestId
                  ? `req ${readinessQuery.meta.requestId}`
                  : undefined
              }
            />
          )}
        />
        <MetricSlot
          title="Agents"
          query={agentsQuery}
          render={() => (
            <MetricCard
              label="Known agents"
              value={formatNumber(agentsQuery.data?.items.length)}
              hint={`${formatNumber(onlineAgents)} online`}
            />
          )}
        />
        <MetricSlot
          title="Policies"
          query={policiesQuery}
          render={() => (
            <MetricCard
              label="Policies"
              value={formatNumber(policiesQuery.data?.items.length)}
            />
          )}
        />
        <MetricSlot
          title="Deployments"
          query={deploymentsQuery}
          render={() => (
            <MetricCard
              label="Recent jobs"
              value={formatNumber(deploymentsQuery.data?.items.length)}
            />
          )}
        />
      </div>

      <div className="grid grid-cols-1 gap-6 xl:grid-cols-2">
        <SectionCard
          title="Recent Deployments"
          description="Latest entries from `GET /api/v1/deployments`."
          action={
            <Link
              href={withLocalePath(locale, "/deployments")}
              className="text-sm text-[color:var(--muted-foreground)] transition-colors hover:text-[color:var(--foreground)]"
            >
              Open deployments
            </Link>
          }
        >
          {deploymentsQuery.isLoading && !deploymentsQuery.data ? (
            <LoadingState compact label="Loading deployments..." />
          ) : deploymentsQuery.error && !deploymentsQuery.data ? (
            <ErrorState
              error={deploymentsQuery.error}
              retry={() => void deploymentsQuery.refetch()}
            />
          ) : recentDeployments.length === 0 ? (
            <Card className="rounded-xl border border-dashed border-[color:var(--border)] bg-[color:var(--surface-subtle)] p-4 text-sm text-[color:var(--muted-foreground)]">
              No deployments have been returned yet.
            </Card>
          ) : (
            <div className="space-y-3">
              {recentDeployments.map((deployment) => (
                <Link
                  key={deployment.id}
                  href={withLocalePath(locale, `/deployments/${deployment.id}`)}
                  className="block rounded-xl border border-[color:var(--border)] bg-[color:var(--surface)] px-4 py-4 transition-colors hover:bg-[color:var(--surface-subtle)]"
                >
                  <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                    <div className="space-y-2">
                      <p className="font-medium text-[color:var(--foreground)]">
                        {deployment.id}
                      </p>
                      <p className="text-sm text-[color:var(--muted-foreground)]">
                        Policy: {deployment.policyId ?? "n/a"}
                      </p>
                      <p className="text-sm text-[color:var(--muted-foreground)]">
                        {formatParamsSummary(deployment.params)}
                      </p>
                    </div>
                    <div className="space-y-2 text-left sm:text-right">
                      <StatusBadge value={deployment.status} />
                      <p className="text-xs text-[color:var(--muted-foreground)]">
                        {formatDateTime(deployment.createdAt)}
                      </p>
                    </div>
                  </div>
                </Link>
              ))}
              <RequestMetaLine meta={deploymentsQuery.meta} />
            </div>
          )}
        </SectionCard>

        <SectionCard
          title="Logs Histogram"
          description="Volume trend from `GET /api/v1/logs/histogram`."
          action={
            <Link
              href={withLocalePath(locale, "/logs")}
              className="text-sm text-[color:var(--muted-foreground)] transition-colors hover:text-[color:var(--foreground)]"
            >
              Open logs
            </Link>
          }
        >
          {histogramQuery.isLoading && !histogramQuery.data ? (
            <LoadingState compact label="Loading histogram..." />
          ) : histogramQuery.error && !histogramQuery.data ? (
            <ErrorState
              error={histogramQuery.error}
              retry={() => void histogramQuery.refetch()}
            />
          ) : (
            <div className="space-y-4">
              <HistogramBars items={histogramQuery.data?.items ?? []} />
              <RequestMetaLine meta={histogramQuery.meta} />
            </div>
          )}
        </SectionCard>
      </div>

      <div className="grid grid-cols-1 gap-6 xl:grid-cols-3">
        <OverviewNamedCountCard
          title="Severity Breakdown"
          description="Severity aggregation from `GET /api/v1/logs/severity`."
          query={severityQuery}
        />
        <OverviewNamedCountCard
          title="Top Hosts"
          description="Host distribution from `GET /api/v1/logs/top-hosts`."
          query={topHostsQuery}
        />
        <OverviewNamedCountCard
          title="Top Services"
          description="Service distribution from `GET /api/v1/logs/top-services`."
          query={topServicesQuery}
        />
      </div>
    </PageStack>
  );
}

function MetricSlot({
  title,
  query,
  render,
}: {
  title: string;
  query: {
    data?: unknown;
    error?: unknown;
    isLoading: boolean;
    refetch: (options?: { silent?: boolean }) => Promise<void>;
  };
  render: () => ReactNode;
}) {
  if (query.isLoading && !query.data) {
    return (
      <Card className="p-4">
        <LoadingState compact label={`Loading ${title.toLowerCase()}...`} />
      </Card>
    );
  }

  if (query.error && !query.data) {
    return (
      <Card className="p-4">
        <ErrorState error={query.error as never} retry={() => void query.refetch()} />
      </Card>
    );
  }

  return render();
}

function OverviewNamedCountCard({
  title,
  description,
  query,
}: {
  title: string;
  description: string;
  query: {
    data?: { items: Array<{ name: string; count: number }> };
    error?: unknown;
    isLoading: boolean;
    meta?: unknown;
    refetch: (options?: { silent?: boolean }) => Promise<void>;
  };
}) {
  return (
    <SectionCard title={title} description={description}>
      {query.isLoading && !query.data ? (
        <LoadingState compact label={`Loading ${title.toLowerCase()}...`} />
      ) : query.error && !query.data ? (
        <ErrorState error={query.error as never} retry={() => void query.refetch()} />
      ) : (
        <div className="space-y-4">
          <NamedCountList items={query.data?.items ?? []} />
          <RequestMetaLine meta={query.meta as never} />
        </div>
      )}
    </SectionCard>
  );
}
