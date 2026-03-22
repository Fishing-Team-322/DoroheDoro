"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import { useSearchParams } from "next/navigation";
import { InventoryPage } from "@/src/page-modules/inventory";
import { AgentsPage } from "@/src/page-modules/agents";
import { CredentialsPage } from "@/src/page-modules/credentials";
import { ErrorCard } from "@/src/page-modules/common/ui/runtime-state";
import {
  getHealthStatus,
  getReadinessStatus,
  getAuthContext,
} from "@/src/features/operations";
import {
  DetailGrid,
  LoadingState,
  MetricCard,
  SectionCard,
  StatusBadge,
  formatDateTime,
  formatMaybeValue,
} from "@/src/features/operations/ui/operations-ui";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
import {
  listAgents,
  listCredentials,
  listHostGroups,
  listHosts,
  type AgentItem,
} from "@/src/shared/lib/runtime-api";
import { Button, Card, EmptyState } from "@/src/shared/ui";

type InfrastructureTab = "overview" | "resources" | "agents" | "access";

type InfrastructureOverviewData = {
  health: string;
  readiness: string;
  subject: string;
  role: string;
  authMode: string;
  hosts: number;
  hostGroups: number;
  agentsTotal: number;
  agentsHealthy: number;
  credentials: number;
  unhealthyAgents: AgentItem[];
};

const validTabs: InfrastructureTab[] = [
  "overview",
  "resources",
  "agents",
  "access",
];

function isHealthyAgentStatus(value?: string) {
  const normalized = value?.trim().toLowerCase() ?? "";
  return ["online", "healthy", "ready"].includes(normalized);
}

function getActiveTab(value: string | null): InfrastructureTab {
  return validTabs.includes(value as InfrastructureTab)
    ? (value as InfrastructureTab)
    : "overview";
}

export function InfrastructurePage() {
  const { locale } = useI18n();
  const searchParams = useSearchParams();
  const activeTab = getActiveTab(searchParams.get("tab"));

  const tabs = useMemo(
    () => [
      {
        id: "overview" as const,
        label: "Overview",
        href: withLocalePath(locale, "/infrastructure?tab=overview"),
      },
      {
        id: "resources" as const,
        label: "Resources",
        href: withLocalePath(locale, "/infrastructure?tab=resources"),
      },
      {
        id: "agents" as const,
        label: "Agents",
        href: withLocalePath(locale, "/infrastructure?tab=agents"),
      },
      {
        id: "access" as const,
        label: "Access",
        href: withLocalePath(locale, "/infrastructure?tab=access"),
      },
    ],
    [locale]
  );

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="space-y-6">
          <div className="flex flex-col gap-4 border-b border-[color:var(--border)] pb-6 xl:flex-row xl:items-center xl:justify-between">
            <div className="space-y-2">
              <h2 className="text-5xl font-semibold text-[color:var(--foreground)]">
                infrastructure workspace
              </h2>
            </div>

            <div className="flex flex-wrap items-center gap-3">
              <div className="inline-flex rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-1 gap-1">
                {tabs.map((tab) => {
                  const isActive = tab.id === activeTab;

                  return (
                    <Link key={tab.id} href={tab.href}>
                      <Button
                        variant={isActive ? "default" : "ghost"}
                        size="sm"
                        className="h-9 px-4"
                      >
                        {tab.label}
                      </Button>
                    </Link>
                  );
                })}
              </div>
            </div>
          </div>

          <div>
            {activeTab === "overview" ? (
              <InfrastructureOverviewSection embedded />
            ) : null}
            {activeTab === "resources" ? (
              <InfrastructureResourcesSection embedded />
            ) : null}
            {activeTab === "agents" ? <AgentsPage embedded /> : null}
            {activeTab === "access" ? <CredentialsPage embedded /> : null}
          </div>
        </div>
      </Card>
    </div>
  );
}

function InfrastructureOverviewSection({
  embedded = false,
}: {
  embedded?: boolean;
}) {
  const { locale } = useI18n();
  const [data, setData] = useState<InfrastructureOverviewData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);

      try {
        const [
          healthResponse,
          readinessResponse,
          authResponse,
          hostsResponse,
          hostGroupsResponse,
          agentsResponse,
          credentialsResponse,
        ] = await Promise.all([
          getHealthStatus(),
          getReadinessStatus(),
          getAuthContext(),
          listHosts(),
          listHostGroups(),
          listAgents(),
          listCredentials(),
        ]);

        if (cancelled) {
          return;
        }

        const unhealthyAgents = agentsResponse.items.filter(
          (item) => !isHealthyAgentStatus(item.status)
        );

        setData({
          health: healthResponse.data.status,
          readiness: readinessResponse.data.status,
          subject: authResponse.data.user?.subject ?? "n/a",
          role: authResponse.data.user?.role ?? "n/a",
          authMode: authResponse.data.auth?.mode ?? "n/a",
          hosts: hostsResponse.items.length,
          hostGroups: hostGroupsResponse.items.length,
          agentsTotal: agentsResponse.items.length,
          agentsHealthy: agentsResponse.items.length - unhealthyAgents.length,
          credentials: credentialsResponse.items.length,
          unhealthyAgents,
        });
      } catch (loadError) {
        if (!cancelled) {
          setError(
            loadError instanceof Error
              ? loadError.message
              : "Failed to load infrastructure overview"
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

  const content = (
    <div className="space-y-6">
      {loading ? <LoadingState label="Loading infrastructure overview..." /> : null}

      {!loading && (error || !data) ? (
        <ErrorCard message={error ?? "Failed to load infrastructure overview"} />
      ) : null}

      {!loading && !error && data ? (
        <>
          <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-5">
            <MetricCard
              label="Public health"
              value={data.health}
              status={data.health}
              hint="System probe"
            />
            <MetricCard
              label="Backend readiness"
              value={data.readiness}
              status={data.readiness}
              hint="Bridge readiness"
            />
            <MetricCard
              label="Resources"
              value={String(data.hosts)}
              hint={`${data.hostGroups} host group(s)`}
            />
            <MetricCard
              label="Agents"
              value={`${data.agentsHealthy}/${data.agentsTotal}`}
              hint="Healthy coverage"
            />
            <MetricCard
              label="Credentials"
              value={String(data.credentials)}
              hint="Vault-backed metadata"
            />
          </section>

          <section className="grid gap-4 xl:grid-cols-[minmax(0,1.1fr)_minmax(0,0.9fr)]">
            <SectionCard
              title="Operator context"
              description="Fast summary of the current session plus the infrastructure surfaces that usually gate day-to-day work."
            >
              <DetailGrid
                items={[
                  { label: "Session subject", value: data.subject },
                  { label: "Role", value: data.role },
                  { label: "Auth mode", value: data.authMode },
                  { label: "Hosts", value: String(data.hosts) },
                  { label: "Host groups", value: String(data.hostGroups) },
                  { label: "Credential profiles", value: String(data.credentials) },
                ]}
              />
            </SectionCard>

            <SectionCard
              title="Quick links"
              description="Deep links keep related infrastructure work inside this section instead of sending operators back to a fragmented sidebar."
            >
              <div className="grid gap-3 sm:grid-cols-2">
                <QuickLinkCard
                  title="Open resources"
                  description="System and inventory in one place."
                  href={withLocalePath(locale, "/infrastructure?tab=resources")}
                />
                <QuickLinkCard
                  title="Open agents"
                  description="Agent registry, health, and shortcuts."
                  href={withLocalePath(locale, "/infrastructure?tab=agents")}
                />
                <QuickLinkCard
                  title="Open access"
                  description="Credential profiles and access metadata."
                  href={withLocalePath(locale, "/infrastructure?tab=access")}
                />
                <QuickLinkCard
                  title="Open live logs"
                  description="Jump straight into the log stream."
                  href={withLocalePath(locale, "/logs/live")}
                />
              </div>
            </SectionCard>
          </section>

          <SectionCard
            title="Agent health"
            description="Degraded agents stay visible here so infrastructure drift is obvious before operators jump into the full agents tab."
          >
            {data.unhealthyAgents.length === 0 ? (
              <EmptyState
                variant="flush"
                title="All tracked agents look healthy"
                description="No degraded or offline agents were detected in the current registry snapshot."
              />
            ) : (
              <div className="space-y-3">
                {data.unhealthyAgents.slice(0, 6).map((agent) => (
                  <Card key={agent.agent_id} className="space-y-2 p-4">
                    <div className="flex flex-wrap items-center justify-between gap-3">
                      <div>
                        <p className="text-base font-semibold text-[color:var(--foreground)]">
                          {agent.hostname}
                        </p>
                        <p className="text-sm text-[color:var(--muted-foreground)]">
                          {agent.agent_id}
                        </p>
                      </div>
                      <StatusBadge value={agent.status} />
                    </div>
                    <p className="text-sm text-[color:var(--muted-foreground)]">
                      Version {formatMaybeValue(agent.version)} / last seen{" "}
                      {formatDateTime(agent.last_seen_at)}
                    </p>
                  </Card>
                ))}
              </div>
            )}
          </SectionCard>
        </>
      ) : null}
    </div>
  );

  return embedded ? (
    <div className="space-y-4">{content}</div>
  ) : (
    <Card className="p-6">{content}</Card>
  );
}

function InfrastructureResourcesSection({
  embedded = false,
}: {
  embedded?: boolean;
}) {
  const content = (
    <div className="space-y-6">
      <InfrastructureSystemPanel />
      <InventoryPage embedded />
    </div>
  );

  return embedded ? (
    <div className="space-y-4">{content}</div>
  ) : (
    <Card className="p-6">{content}</Card>
  );
}

function InfrastructureSystemPanel() {
  const [state, setState] = useState<{
    health: string;
    readiness: string;
    subject: string;
    role: string;
    authMode: string;
  } | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);

      try {
        const [healthResponse, readinessResponse, authResponse] = await Promise.all([
          getHealthStatus(),
          getReadinessStatus(),
          getAuthContext(),
        ]);

        if (cancelled) {
          return;
        }

        setState({
          health: healthResponse.data.status,
          readiness: readinessResponse.data.status,
          subject: authResponse.data.user?.subject ?? "n/a",
          role: authResponse.data.user?.role ?? "n/a",
          authMode: authResponse.data.auth?.mode ?? "n/a",
        });
      } catch (loadError) {
        if (!cancelled) {
          setError(
            loadError instanceof Error ? loadError.message : "Failed to load system state"
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

  return (
    <SectionCard
      title="System"
      description="Runtime health, readiness, and current authentication context grouped into the infrastructure resources view."
    >
      {loading ? <LoadingState compact label="Loading system state..." /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}
      {!loading && !error && state ? (
        <div className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-5">
            <MetricCard label="Public health" value={state.health} status={state.health} />
            <MetricCard
              label="Backend readiness"
              value={state.readiness}
              status={state.readiness}
            />
            <MetricCard label="Session subject" value={state.subject} />
            <MetricCard label="Role" value={state.role} />
            <MetricCard label="Auth mode" value={state.authMode} />
          </div>

          <DetailGrid
            items={[
              { label: "Public probe", value: state.health },
              { label: "Readiness", value: state.readiness },
              { label: "Current subject", value: state.subject },
              { label: "Current role", value: state.role },
              { label: "Authentication mode", value: state.authMode },
            ]}
          />
        </div>
      ) : null}
    </SectionCard>
  );
}

function QuickLinkCard({
  title,
  description,
  href,
}: {
  title: string;
  description: string;
  href: string;
}) {
  return (
    <Card className="space-y-3 p-4">
      <div className="space-y-1">
        <p className="text-base font-semibold text-[color:var(--foreground)]">
          {title}
        </p>
        <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
          {description}
        </p>
      </div>

      <Link href={href}>
        <Button variant="outline" size="sm" className="h-10 px-4">
          Open
        </Button>
      </Link>
    </Card>
  );
}