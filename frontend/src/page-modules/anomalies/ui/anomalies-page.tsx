"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import type { Locale } from "@/src/shared/config";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import {
  translateValueLabel,
  useI18n,
  withLocalePath,
} from "@/src/shared/lib/i18n";
import {
  NoticeBanner,
  SectionCard,
  formatDateTime,
} from "@/src/features/operations/ui/operations-ui";
import {
  getAnomalyModeDefinitions,
  getAnomalyWorkbenchData,
  getSeverityTone,
  type AnomalyMode,
  type AnomalyRecord,
  type AnomalyWorkbenchData,
} from "@/src/shared/lib/operations-workbench";
import { Badge, Button, Card, EmptyState } from "@/src/shared/ui";
import { ErrorCard, LoadingCard } from "@/src/page-modules/common/ui/runtime-state";
import { AnomalyTimeline } from "./anomaly-timeline";
import { DetectionModeSelector } from "./detection-mode-selector";

const copyByLocale = {
  en: {
    title: "Anomalies",
    description:
      "Detection workbench for recent anomaly instances, open alerts, and operator correlation timeline.",
    breadcrumbs: "Anomalies",
    mode: {
      title: "Detection mode",
      description:
        "The selector changes the frontend correlation lens and explainer density. It does not mutate backend detector configuration.",
    },
    notice: {
      title: "Open alert correlation",
      description:
        "Anomaly correlation is currently built from existing log anomaly and alert endpoints. A dedicated backend anomaly-mode contract was not found, so the selector above is intentionally frontend-only.",
    },
    loading: "Loading anomalies...",
    error: "Failed to load anomaly workbench",
    metrics: {
      anomalies: "Recent anomaly instances",
      openAlerts: "Open alerts",
      activeLens: "Active lens",
    },
    list: {
      title: "Recent anomalies",
      description:
        "Select an anomaly to inspect its explanation and jump directly into the related alert detail flow.",
      emptyTitle: "No recent anomalies",
      emptyDescription:
        "The current runtime API did not return anomaly instances for this view.",
      unknownHost: "unknown host",
      unknownService: "unknown service",
    },
    timeline: {
      title: "Correlation timeline",
      description:
        "Recent anomalies and their open-alert echoes in one operator-friendly sequence.",
      emptyTitle: "Timeline is empty",
      emptyDescription:
        "No anomaly or alert events were returned for the current lens.",
    },
    detail: {
      emptyTitle: "No anomaly selected",
      emptyDescription:
        "Pick a recent anomaly to inspect the correlation context.",
      title: "Operator detail",
      description:
        "This detail view stays focused on what an operator needs next: scope, recency, and the fastest path to the linked alert.",
      action: "Open alert detail",
      scope: "Scope",
      host: "Host",
      service: "Service",
      fingerprint: "Fingerprint",
      correlation: "Correlation",
      triggered: "Triggered",
      matchingAlerts: "Matching open alerts",
      route: "Alert detail route",
    },
  },
  ru: {
    title: "Аномалии",
    description:
      "Workbench для недавних anomaly-инстансов, открытых алертов и операторской correlation timeline.",
    breadcrumbs: "Аномалии",
    mode: {
      title: "Режим детекции",
      description:
        "Селектор меняет фронтенд-линзу корреляции и плотность пояснений. Он не изменяет backend-конфигурацию детектора.",
    },
    notice: {
      title: "Корреляция открытых алертов",
      description:
        "Корреляция аномалий сейчас строится из существующих endpoint'ов аномалий логов и алертов. Отдельный backend-контракт режима аномалий не найден, поэтому селектор выше намеренно работает только на фронтенде.",
    },
    loading: "Загрузка аномалий...",
    error: "Не удалось загрузить anomaly workbench",
    metrics: {
      anomalies: "Недавние аномалии",
      openAlerts: "Открытые алерты",
      activeLens: "Активная линза",
    },
    list: {
      title: "Недавние аномалии",
      description:
        "Выберите аномалию, чтобы посмотреть объяснение и сразу перейти в связанный alert detail flow.",
      emptyTitle: "Недавних аномалий нет",
      emptyDescription:
        "Текущий runtime API не вернул anomaly-инстансы для этого представления.",
      unknownHost: "неизвестный хост",
      unknownService: "неизвестный сервис",
    },
    timeline: {
      title: "Лента корреляции",
      description:
        "Недавние аномалии и их эхо в открытых алертах в одной последовательности для оператора.",
      emptyTitle: "Лента пуста",
      emptyDescription:
        "Для текущей линзы не вернулись события аномалий или алертов.",
    },
    detail: {
      emptyTitle: "Аномалия не выбрана",
      emptyDescription:
        "Выберите недавнюю аномалию, чтобы посмотреть контекст корреляции.",
      title: "Детали для оператора",
      description:
        "Это представление фокусируется на том, что нужно оператору дальше: скоуп, свежесть и самый быстрый путь к связанному алерту.",
      action: "Открыть детали алерта",
      scope: "Скоуп",
      host: "Хост",
      service: "Сервис",
      fingerprint: "Fingerprint",
      correlation: "Корреляция",
      triggered: "Сработало",
      matchingAlerts: "Совпадающие открытые алерты",
      route: "Маршрут к деталям алерта",
    },
  },
} as const;

function toBadgeVariant(severity?: string) {
  const tone = getSeverityTone(severity);
  if (tone === "danger") {
    return "danger";
  }
  if (tone === "warning") {
    return "warning";
  }
  if (tone === "success") {
    return "success";
  }
  return "default";
}

export function AnomaliesPage({ embedded = false }: { embedded?: boolean } = {}) {
  const { dictionary, locale } = useI18n();
  const copy = copyByLocale[locale];
  const [mode, setMode] = useState<AnomalyMode>("medium");
  const [data, setData] = useState<AnomalyWorkbenchData | null>(null);
  const [selectedAnomalyId, setSelectedAnomalyId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);
      try {
        const response = await getAnomalyWorkbenchData(mode, locale);
        if (cancelled) {
          return;
        }
        setData(response);
        setSelectedAnomalyId((current) =>
          response.anomalies.some((item) => item.id === current)
            ? current
            : response.anomalies[0]?.id ?? null
        );
      } catch (loadError) {
        if (!cancelled) {
          setError(
            loadError instanceof Error
              ? loadError.message
              : copy.error
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
  }, [copy.error, locale, mode]);

  const selectedAnomaly = useMemo(() => {
    return data?.anomalies.find((item) => item.id === selectedAnomalyId) ?? null;
  }, [data?.anomalies, selectedAnomalyId]);

  return (
    <div className={embedded ? "space-y-4" : "space-y-6"}>
      {!embedded ? (
        <PageHeader
          title={copy.title}
          description={copy.description}
          breadcrumbs={[
            { label: dictionary.common.dashboard, href: "#" },
            { label: copy.breadcrumbs },
          ]}
        />
      ) : null}

      <SectionCard
        title={copy.mode.title}
        description={copy.mode.description}
      >
        <DetectionModeSelector
          value={mode}
          options={getAnomalyModeDefinitions(locale)}
          onChange={setMode}
        />
      </SectionCard>

      <NoticeBanner
        title={copy.notice.title}
        description={copy.notice.description}
      />

      {loading ? <LoadingCard label={copy.loading} /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error && data ? (
        <>
          <section className="grid gap-4 md:grid-cols-3">
            <Card className="space-y-2 p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                Recent anomaly instances
                {copy.metrics.anomalies}
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {data.anomalies.length}
              </p>
            </Card>
            <Card className="space-y-2 p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.metrics.openAlerts}
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {data.openAlerts}
              </p>
            </Card>
            <Card className="space-y-2 p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.metrics.activeLens}
              </p>
              <p className="text-3xl font-semibold capitalize text-[color:var(--foreground)]">
                {translateValueLabel(mode, locale)}
              </p>
            </Card>
          </section>

          <section className="grid gap-4 xl:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
            <SectionCard
              title={copy.list.title}
              description={copy.list.description}
            >
              {data.anomalies.length === 0 ? (
                <EmptyState
                  variant="flush"
                  title={copy.list.emptyTitle}
                  description={copy.list.emptyDescription}
                />
              ) : (
                <div className="space-y-3">
                  {data.anomalies.map((item) => {
                    const active = item.id === selectedAnomalyId;

                    return (
                      <button
                        key={item.id}
                        type="button"
                        onClick={() => setSelectedAnomalyId(item.id)}
                        className={`w-full rounded-xl border p-4 text-left transition-colors ${
                          active
                            ? "border-[color:var(--status-info-border)] bg-[color:var(--status-info-bg)]/45"
                            : "border-[color:var(--border)] bg-[color:var(--surface)] hover:bg-[color:var(--surface-subtle)]"
                        }`}
                        >
                        <div className="flex flex-wrap items-center gap-2">
                          <Badge variant={toBadgeVariant(item.severity)}>
                            {translateValueLabel(item.severity, locale)}
                          </Badge>
                          <Badge>{translateValueLabel(item.status, locale)}</Badge>
                        </div>
                        <p className="mt-3 text-base font-semibold text-[color:var(--foreground)]">
                          {item.title}
                        </p>
                        <p className="mt-2 text-sm leading-6 text-[color:var(--muted-foreground)]">
                          {item.host || copy.list.unknownHost} /{" "}
                          {item.service || copy.list.unknownService}
                        </p>
                      </button>
                    );
                  })}
                </div>
              )}
            </SectionCard>

            <AnomalyDetail anomaly={selectedAnomaly} locale={locale} />
          </section>

          <SectionCard
            title={copy.timeline.title}
            description={copy.timeline.description}
          >
            {data.timeline.length === 0 ? (
              <EmptyState
                variant="flush"
                title={copy.timeline.emptyTitle}
                description={copy.timeline.emptyDescription}
              />
            ) : (
              <AnomalyTimeline locale={locale} items={data.timeline} />
            )}
          </SectionCard>
        </>
      ) : null}
    </div>
  );
}

function AnomalyDetail({
  anomaly,
  locale,
}: {
  anomaly: AnomalyRecord | null;
  locale: Locale;
}) {
  const copy = copyByLocale[locale];

  if (!anomaly) {
    return (
      <Card>
        <EmptyState
          variant="flush"
          title={copy.detail.emptyTitle}
          description={copy.detail.emptyDescription}
        />
      </Card>
    );
  }

  return (
    <SectionCard
      title={copy.detail.title}
      description={copy.detail.description}
      action={
        <Link
          href={withLocalePath(locale, `/security?tab=alerts&alert=${anomaly.alertId}`)}
        >
          <Button variant="outline" size="sm" className="h-10 px-4">
            {copy.detail.action}
          </Button>
        </Link>
      }
    >
      <div className="space-y-4">
        <Card className="space-y-3 p-4">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant={toBadgeVariant(anomaly.severity)}>
              {translateValueLabel(anomaly.severity, locale)}
            </Badge>
            <Badge>{translateValueLabel(anomaly.status, locale)}</Badge>
          </div>
          <p className="text-xl font-semibold text-[color:var(--foreground)]">
            {anomaly.title}
          </p>
          <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
            {anomaly.explanation}
          </p>
        </Card>

        <div className="grid gap-4 md:grid-cols-2">
          <Card className="space-y-3 p-4">
            <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
              {copy.detail.scope}
            </h3>
            <p className="text-sm text-[color:var(--foreground)]">
              {copy.detail.host}: {anomaly.host || "n/a"}
            </p>
            <p className="text-sm text-[color:var(--foreground)]">
              {copy.detail.service}: {anomaly.service || "n/a"}
            </p>
            <p className="text-sm text-[color:var(--foreground)]">
              {copy.detail.fingerprint}: {anomaly.fingerprint || "n/a"}
            </p>
          </Card>

          <Card className="space-y-3 p-4">
            <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
              {copy.detail.correlation}
            </h3>
            <p className="text-sm text-[color:var(--foreground)]">
              {copy.detail.triggered}: {formatDateTime(anomaly.triggeredAt, locale)}
            </p>
            <p className="text-sm text-[color:var(--foreground)]">
              {copy.detail.matchingAlerts}: {anomaly.matchingAlerts}
            </p>
            <p className="text-sm text-[color:var(--foreground)]">
              {copy.detail.route}: /security?tab=alerts&alert={anomaly.alertId}
            </p>
          </Card>
        </div>
      </div>
    </SectionCard>
  );
}
