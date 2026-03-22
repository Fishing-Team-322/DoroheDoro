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
import {
  translateValueLabel,
  useI18n,
  withLocalePath,
} from "@/src/shared/lib/i18n";
import {
  listAgents,
  listCredentials,
  listHostGroups,
  listHosts,
  type AgentItem,
} from "@/src/shared/lib/runtime-api";
import { Button, Card, EmptyState } from "@/src/shared/ui";

const copyByLocale = {
  en: {
    tabs: {
      overview: "Overview",
      resources: "Resources",
      agents: "Agents",
      access: "Access",
    },
    pageTitle: "Infrastructure workspace",
    loadOverviewError: "Failed to load infrastructure overview",
    loadingOverview: "Loading infrastructure overview...",
    overview: {
      metrics: {
        publicHealth: { label: "Public health", hint: "System probe" },
        backendReadiness: { label: "Backend readiness", hint: "Bridge readiness" },
        resources: { label: "Resources", hintSuffix: "host group(s)" },
        agents: { label: "Agents", hint: "Healthy coverage" },
        credentials: { label: "Credentials", hint: "Vault-backed metadata" },
      },
      operatorContext: {
        title: "Operator context",
        description:
          "Fast summary of the current session plus the infrastructure surfaces that usually gate day-to-day work.",
        fields: {
          sessionSubject: "Session subject",
          role: "Role",
          authMode: "Auth mode",
          hosts: "Hosts",
          hostGroups: "Host groups",
          credentialProfiles: "Credential profiles",
        },
      },
      quickLinks: {
        title: "Quick links",
        description:
          "Deep links keep related infrastructure work inside this section instead of sending operators back to a fragmented sidebar.",
        items: {
          resources: {
            title: "Open resources",
            description: "System and inventory in one place.",
          },
          agents: {
            title: "Open agents",
            description: "Agent registry, health, and shortcuts.",
          },
          access: {
            title: "Open access",
            description: "Credential profiles and access metadata.",
          },
          liveLogs: {
            title: "Open live logs",
            description: "Jump straight into the log stream.",
          },
        },
        action: "Open",
      },
      agentHealth: {
        title: "Agent health",
        description:
          "Degraded agents stay visible here so infrastructure drift is obvious before operators jump into the full agents tab.",
        emptyTitle: "All tracked agents look healthy",
        emptyDescription:
          "No degraded or offline agents were detected in the current registry snapshot.",
        versionPrefix: "Version",
        lastSeenPrefix: "last seen",
      },
    },
    system: {
      loadError: "Failed to load system state",
      title: "System",
      description:
        "Runtime health, readiness, and current authentication context grouped into the infrastructure resources view.",
      loading: "Loading system state...",
      metrics: {
        publicHealth: "Public health",
        backendReadiness: "Backend readiness",
        sessionSubject: "Session subject",
        role: "Role",
        authMode: "Auth mode",
      },
      details: {
        publicProbe: "Public probe",
        readiness: "Readiness",
        currentSubject: "Current subject",
        currentRole: "Current role",
        authenticationMode: "Authentication mode",
      },
    },
  },
  ru: {
    tabs: {
      overview: "Обзор",
      resources: "Ресурсы",
      agents: "Агенты",
      access: "Доступы",
    },
    pageTitle: "инфраструктура",
    loadOverviewError: "Не удалось загрузить обзор инфраструктуры",
    loadingOverview: "Загрузка обзора инфраструктуры...",
    overview: {
      metrics: {
        publicHealth: { label: "Публичное здоровье", hint: "Системная проба" },
        backendReadiness: { label: "Готовность backend", hint: "Готовность bridge" },
        resources: { label: "Ресурсы", hintSuffix: "групп хостов" },
        agents: { label: "Агенты", hint: "Healthy-покрытие" },
        credentials: { label: "Доступы", hint: "Метаданные из Vault" },
      },
      operatorContext: {
        title: "Контекст оператора",
        description:
          "Быстрая сводка по текущей сессии и инфраструктурным поверхностям, которые чаще всего ограничивают повседневную работу.",
        fields: {
          sessionSubject: "Субъект сессии",
          role: "Роль",
          authMode: "Режим авторизации",
          hosts: "Хосты",
          hostGroups: "Группы хостов",
          credentialProfiles: "Профили доступов",
        },
      },
      quickLinks: {
        title: "Быстрые ссылки",
        description:
          "Deep links удерживают связанную инфраструктурную работу внутри раздела, не возвращая оператора в разрозненный sidebar.",
        items: {
          resources: {
            title: "Открыть ресурсы",
            description: "System и inventory в одном месте.",
          },
          agents: {
            title: "Открыть агентов",
            description: "Реестр агентов, здоровье и быстрые действия.",
          },
          access: {
            title: "Открыть доступы",
            description: "Профили доступов и access metadata.",
          },
          liveLogs: {
            title: "Открыть live-логи",
            description: "Сразу перейти в поток логов.",
          },
        },
        action: "Открыть",
      },
      agentHealth: {
        title: "Состояние агентов",
        description:
          "Деградированные агенты остаются видимыми здесь, чтобы дрейф инфраструктуры был заметен до перехода на полную вкладку агентов.",
        emptyTitle: "Все отслеживаемые агенты выглядят здоровыми",
        emptyDescription:
          "В текущем снимке реестра не обнаружено деградированных или offline-агентов.",
        versionPrefix: "Версия",
        lastSeenPrefix: "последний сигнал",
      },
    },
    system: {
      loadError: "Не удалось загрузить состояние системы",
      title: "Система",
      description:
        "Runtime health, readiness и текущий контекст авторизации, собранные во view инфраструктурных ресурсов.",
      loading: "Загрузка состояния системы...",
      metrics: {
        publicHealth: "Публичное здоровье",
        backendReadiness: "Готовность backend",
        sessionSubject: "Субъект сессии",
        role: "Роль",
        authMode: "Режим авторизации",
      },
      details: {
        publicProbe: "Публичная проба",
        readiness: "Готовность",
        currentSubject: "Текущий субъект",
        currentRole: "Текущая роль",
        authenticationMode: "Режим аутентификации",
      },
    },
  },
} as const;

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
  const copy = copyByLocale[locale];
  const searchParams = useSearchParams();
  const activeTab = getActiveTab(searchParams.get("tab"));

  const tabs = useMemo(
    () => [
      {
        id: "overview" as const,
        label: copy.tabs.overview,
        href: withLocalePath(locale, "/infrastructure?tab=overview"),
      },
      {
        id: "resources" as const,
        label: copy.tabs.resources,
        href: withLocalePath(locale, "/infrastructure?tab=resources"),
      },
      {
        id: "agents" as const,
        label: copy.tabs.agents,
        href: withLocalePath(locale, "/infrastructure?tab=agents"),
      },
      {
        id: "access" as const,
        label: copy.tabs.access,
        href: withLocalePath(locale, "/infrastructure?tab=access"),
      },
    ],
    [copy.tabs.access, copy.tabs.agents, copy.tabs.overview, copy.tabs.resources, locale]
  );

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="space-y-6">
          <div className="flex flex-col gap-4 border-b border-[color:var(--border)] pb-6 xl:flex-row xl:items-center xl:justify-between">
            <div className="space-y-2">
              <h2 className="text-5xl font-semibold text-[color:var(--foreground)]">
                {copy.pageTitle}
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
  const copy = copyByLocale[locale];
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
              : copy.loadOverviewError
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
  }, [copy.loadOverviewError]);

  const content = (
    <div className="space-y-6">
      {loading ? <LoadingState label={copy.loadingOverview} /> : null}

      {!loading && (error || !data) ? (
        <ErrorCard message={error ?? copy.loadOverviewError} />
      ) : null}

      {!loading && !error && data ? (
        <>
          <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-5">
            <MetricCard
              label={copy.overview.metrics.publicHealth.label}
              value={translateValueLabel(data.health, locale)}
              status={data.health}
              hint={copy.overview.metrics.publicHealth.hint}
            />
            <MetricCard
              label={copy.overview.metrics.backendReadiness.label}
              value={translateValueLabel(data.readiness, locale)}
              status={data.readiness}
              hint={copy.overview.metrics.backendReadiness.hint}
            />
            <MetricCard
              label={copy.overview.metrics.resources.label}
              value={String(data.hosts)}
              hint={`${data.hostGroups} ${copy.overview.metrics.resources.hintSuffix}`}
            />
            <MetricCard
              label={copy.overview.metrics.agents.label}
              value={`${data.agentsHealthy}/${data.agentsTotal}`}
              hint={copy.overview.metrics.agents.hint}
            />
            <MetricCard
              label={copy.overview.metrics.credentials.label}
              value={String(data.credentials)}
              hint={copy.overview.metrics.credentials.hint}
            />
          </section>

          <section className="grid gap-4 xl:grid-cols-[minmax(0,1.1fr)_minmax(0,0.9fr)]">
            <SectionCard
              title={copy.overview.operatorContext.title}
              description={copy.overview.operatorContext.description}
            >
              <DetailGrid
                items={[
                  {
                    label: copy.overview.operatorContext.fields.sessionSubject,
                    value: data.subject,
                  },
                  { label: copy.overview.operatorContext.fields.role, value: data.role },
                  {
                    label: copy.overview.operatorContext.fields.authMode,
                    value: data.authMode,
                  },
                  {
                    label: copy.overview.operatorContext.fields.hosts,
                    value: String(data.hosts),
                  },
                  {
                    label: copy.overview.operatorContext.fields.hostGroups,
                    value: String(data.hostGroups),
                  },
                  {
                    label: copy.overview.operatorContext.fields.credentialProfiles,
                    value: String(data.credentials),
                  },
                ]}
              />
            </SectionCard>

            <SectionCard
              title={copy.overview.quickLinks.title}
              description={copy.overview.quickLinks.description}
            >
              <div className="grid gap-3 sm:grid-cols-2">
                <QuickLinkCard
                  title={copy.overview.quickLinks.items.resources.title}
                  description={copy.overview.quickLinks.items.resources.description}
                  href={withLocalePath(locale, "/infrastructure?tab=resources")}
                  actionLabel={copy.overview.quickLinks.action}
                />
                <QuickLinkCard
                  title={copy.overview.quickLinks.items.agents.title}
                  description={copy.overview.quickLinks.items.agents.description}
                  href={withLocalePath(locale, "/infrastructure?tab=agents")}
                  actionLabel={copy.overview.quickLinks.action}
                />
                <QuickLinkCard
                  title={copy.overview.quickLinks.items.access.title}
                  description={copy.overview.quickLinks.items.access.description}
                  href={withLocalePath(locale, "/infrastructure?tab=access")}
                  actionLabel={copy.overview.quickLinks.action}
                />
                <QuickLinkCard
                  title={copy.overview.quickLinks.items.liveLogs.title}
                  description={copy.overview.quickLinks.items.liveLogs.description}
                  href={withLocalePath(locale, "/logs/live")}
                  actionLabel={copy.overview.quickLinks.action}
                />
              </div>
            </SectionCard>
          </section>

          <SectionCard
            title={copy.overview.agentHealth.title}
            description={copy.overview.agentHealth.description}
          >
            {data.unhealthyAgents.length === 0 ? (
              <EmptyState
                variant="flush"
                title={copy.overview.agentHealth.emptyTitle}
                description={copy.overview.agentHealth.emptyDescription}
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
                      {copy.overview.agentHealth.versionPrefix}{" "}
                      {formatMaybeValue(agent.version, locale)} /{" "}
                      {copy.overview.agentHealth.lastSeenPrefix}{" "}
                      {formatDateTime(agent.last_seen_at, locale)}
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
  const { locale } = useI18n();
  const copy = copyByLocale[locale];
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
            loadError instanceof Error ? loadError.message : copy.system.loadError
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
  }, [copy.system.loadError]);

  return (
    <SectionCard
      title={copy.system.title}
      description={copy.system.description}
    >
      {loading ? <LoadingState compact label={copy.system.loading} /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}
      {!loading && !error && state ? (
        <div className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-5">
            <MetricCard
              label={copy.system.metrics.publicHealth}
              value={translateValueLabel(state.health, locale)}
              status={state.health}
            />
            <MetricCard
              label={copy.system.metrics.backendReadiness}
              value={translateValueLabel(state.readiness, locale)}
              status={state.readiness}
            />
            <MetricCard label={copy.system.metrics.sessionSubject} value={state.subject} />
            <MetricCard label={copy.system.metrics.role} value={state.role} />
            <MetricCard label={copy.system.metrics.authMode} value={state.authMode} />
          </div>

          <DetailGrid
            items={[
              {
                label: copy.system.details.publicProbe,
                value: translateValueLabel(state.health, locale),
              },
              {
                label: copy.system.details.readiness,
                value: translateValueLabel(state.readiness, locale),
              },
              { label: copy.system.details.currentSubject, value: state.subject },
              { label: copy.system.details.currentRole, value: state.role },
              {
                label: copy.system.details.authenticationMode,
                value: state.authMode,
              },
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
  actionLabel,
}: {
  title: string;
  description: string;
  href: string;
  actionLabel: string;
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
          {actionLabel}
        </Button>
      </Link>
    </Card>
  );
}
