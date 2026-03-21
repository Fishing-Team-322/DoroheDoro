"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import type { Locale } from "@/src/shared/config";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
import {
  NoticeBanner,
  SectionCard,
  formatDateTime,
} from "@/src/features/operations/ui/operations-ui";
import {
  anomalyModeDefinitions,
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

export function AnomaliesPage() {
  const { dictionary, locale } = useI18n();
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
        const response = await getAnomalyWorkbenchData(mode);
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
              : "Failed to load anomaly workbench"
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
  }, [mode]);

  const selectedAnomaly = useMemo(() => {
    return data?.anomalies.find((item) => item.id === selectedAnomalyId) ?? null;
  }, [data?.anomalies, selectedAnomalyId]);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Anomalies"
        description="Detection workbench for recent anomaly instances, open alerts, and operator correlation timeline."
        breadcrumbs={[
          { label: dictionary.common.dashboard, href: "#" },
          { label: "Anomalies" },
        ]}
      />

      <SectionCard
        title="Detection mode"
        description="The selector changes the frontend correlation lens and explainer density. It does not mutate backend detector configuration."
      >
        <DetectionModeSelector
          value={mode}
          options={anomalyModeDefinitions}
          onChange={setMode}
        />
      </SectionCard>

      <NoticeBanner
        title="Open alert correlation"
        description="Anomaly correlation is currently built from existing log anomaly and alert endpoints. A dedicated backend anomaly-mode contract was not found, so the selector above is intentionally frontend-only."
      />

      {loading ? <LoadingCard label="Loading anomalies..." /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error && data ? (
        <>
          <section className="grid gap-4 md:grid-cols-3">
            <Card className="space-y-2 p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                Recent anomaly instances
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {data.anomalies.length}
              </p>
            </Card>
            <Card className="space-y-2 p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                Open alerts
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {data.openAlerts}
              </p>
            </Card>
            <Card className="space-y-2 p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                Active lens
              </p>
              <p className="text-3xl font-semibold capitalize text-[color:var(--foreground)]">
                {mode}
              </p>
            </Card>
          </section>

          <section className="grid gap-4 xl:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
            <SectionCard
              title="Recent anomalies"
              description="Select an anomaly to inspect its explanation and jump directly into the related alert detail flow."
            >
              {data.anomalies.length === 0 ? (
                <EmptyState
                  variant="flush"
                  title="No recent anomalies"
                  description="The current runtime API did not return anomaly instances for this view."
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
                          <Badge variant={toBadgeVariant(item.severity)}>{item.severity}</Badge>
                          <Badge>{item.status}</Badge>
                        </div>
                        <p className="mt-3 text-base font-semibold text-[color:var(--foreground)]">
                          {item.title}
                        </p>
                        <p className="mt-2 text-sm leading-6 text-[color:var(--muted-foreground)]">
                          {item.host || "unknown host"} / {item.service || "unknown service"}
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
            title="Correlation timeline"
            description="Recent anomalies and their open-alert echoes in one operator-friendly sequence."
          >
            {data.timeline.length === 0 ? (
              <EmptyState
                variant="flush"
                title="Timeline is empty"
                description="No anomaly or alert events were returned for the current lens."
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
  if (!anomaly) {
    return (
      <Card>
        <EmptyState
          variant="flush"
          title="No anomaly selected"
          description="Pick a recent anomaly to inspect the correlation context."
        />
      </Card>
    );
  }

  return (
    <SectionCard
      title="Operator detail"
      description="This detail view stays focused on what an operator needs next: scope, recency, and the fastest path to the linked alert."
      action={
        <Link href={withLocalePath(locale, `/alerts?alert=${anomaly.alertId}`)}>
          <Button variant="outline" size="sm" className="h-10 px-4">
            Open alert detail
          </Button>
        </Link>
      }
    >
      <div className="space-y-4">
        <Card className="space-y-3 p-4">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant={toBadgeVariant(anomaly.severity)}>{anomaly.severity}</Badge>
            <Badge>{anomaly.status}</Badge>
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
              Scope
            </h3>
            <p className="text-sm text-[color:var(--foreground)]">
              Host: {anomaly.host || "n/a"}
            </p>
            <p className="text-sm text-[color:var(--foreground)]">
              Service: {anomaly.service || "n/a"}
            </p>
            <p className="text-sm text-[color:var(--foreground)]">
              Fingerprint: {anomaly.fingerprint || "n/a"}
            </p>
          </Card>

          <Card className="space-y-3 p-4">
            <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
              Correlation
            </h3>
            <p className="text-sm text-[color:var(--foreground)]">
              Triggered: {formatDateTime(anomaly.triggeredAt)}
            </p>
            <p className="text-sm text-[color:var(--foreground)]">
              Matching open alerts: {anomaly.matchingAlerts}
            </p>
            <p className="text-sm text-[color:var(--foreground)]">
              Alert detail route: /alerts?alert={anomaly.alertId}
            </p>
          </Card>
        </div>
      </div>
    </SectionCard>
  );
}
