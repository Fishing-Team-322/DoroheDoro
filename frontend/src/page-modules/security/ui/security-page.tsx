"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import type { Locale } from "@/src/shared/config";
import {
  getSecurityPostureData,
  getSeverityTone,
  type SecurityFinding,
  type SecurityPostureData,
} from "@/src/shared/lib/operations-workbench";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
import { Badge, Button, Card, EmptyState } from "@/src/shared/ui";
import {
  NoticeBanner,
  SectionCard,
} from "@/src/features/operations/ui/operations-ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { ErrorCard, LoadingCard } from "@/src/page-modules/common/ui/runtime-state";
import { SecurityOverviewCard } from "./security-overview-card";

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

export function SecurityPage() {
  const { dictionary, locale } = useI18n();
  const [data, setData] = useState<SecurityPostureData | null>(null);
  const [selectedFindingId, setSelectedFindingId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);
      try {
        const response = await getSecurityPostureData();
        if (cancelled) {
          return;
        }
        setData(response);
        setSelectedFindingId((current) =>
          response.findings.some((item) => item.id === current)
            ? current
            : response.findings[0]?.id ?? null
        );
      } catch (loadError) {
        if (!cancelled) {
          setError(
            loadError instanceof Error
              ? loadError.message
              : "Failed to build security posture"
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

  const selectedFinding = useMemo(() => {
    return data?.findings.find((item) => item.id === selectedFindingId) ?? null;
  }, [data?.findings, selectedFindingId]);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Security"
        description="Operator-facing security posture synthesized from runtime policies, alerts, agents, deployments, and audit events."
        breadcrumbs={[
          { label: dictionary.common.dashboard, href: "#" },
          { label: "Security" },
        ]}
      />

      <NoticeBanner
        title="Frontend posture synthesis"
        description="A dedicated backend security-posture contract was not found in the current frontend runtime API. This page safely composes posture findings from existing frontend-visible endpoints and keeps operator UX usable in demo mode."
      />

      {loading ? <LoadingCard label="Loading security posture..." /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error && data ? (
        <>
          <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
            {data.summary.map((item) => (
              <SecurityOverviewCard key={item.id} item={item} />
            ))}
          </section>

          {data.findings.length === 0 ? (
            <Card>
              <EmptyState
                variant="flush"
                title="Security posture looks stable"
                description="The current runtime data did not produce active posture findings. Continue monitoring alerts, agent coverage, and rollout health."
              />
            </Card>
          ) : (
            <section className="grid gap-4 xl:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
              <SectionCard
                title="Findings"
                description="The left rail stays compact so operators can move quickly between findings without leaving the dashboard."
              >
                <div className="space-y-3">
                  {data.findings.map((finding) => {
                    const active = finding.id === selectedFindingId;

                    return (
                      <button
                        key={finding.id}
                        type="button"
                        onClick={() => setSelectedFindingId(finding.id)}
                        className={`w-full rounded-xl border p-4 text-left transition-colors ${
                          active
                            ? "border-[color:var(--status-info-border)] bg-[color:var(--status-info-bg)]/50"
                            : "border-[color:var(--border)] bg-[color:var(--surface)] hover:bg-[color:var(--surface-subtle)]"
                        }`}
                      >
                        <div className="flex flex-wrap items-center gap-2">
                          <Badge variant={toBadgeVariant(finding.severity)}>
                            {finding.severity}
                          </Badge>
                          <Badge>{finding.status}</Badge>
                        </div>
                        <p className="mt-3 text-base font-semibold text-[color:var(--foreground)]">
                          {finding.title}
                        </p>
                        <p className="mt-2 text-sm leading-6 text-[color:var(--muted-foreground)]">
                          {finding.summary}
                        </p>
                      </button>
                    );
                  })}
                </div>
              </SectionCard>

              <SecurityFindingDetail finding={selectedFinding} locale={locale} />
            </section>
          )}
        </>
      ) : null}
    </div>
  );
}

function SecurityFindingDetail({
  finding,
  locale,
}: {
  finding: SecurityFinding | null;
  locale: Locale;
}) {
  if (!finding) {
    return (
      <Card>
        <EmptyState
          variant="flush"
          title="No finding selected"
          description="Choose a finding from the list to inspect impact, evidence, and the recommended next action."
        />
      </Card>
    );
  }

  return (
    <SectionCard
      title="Detail view"
      description="This panel is designed for operator handoff: what happened, why it matters, and what to do next."
      action={
        finding.relatedRoute ? (
          <Link href={withLocalePath(locale, finding.relatedRoute.href)}>
            <Button variant="outline" size="sm" className="h-10 px-4">
              {finding.relatedRoute.label}
            </Button>
          </Link>
        ) : null
      }
    >
      <div className="space-y-5">
        <div className="rounded-xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant={toBadgeVariant(finding.severity)}>{finding.severity}</Badge>
            <Badge>{finding.status}</Badge>
          </div>
          <p className="mt-3 text-xl font-semibold text-[color:var(--foreground)]">
            {finding.title}
          </p>
          <p className="mt-3 text-sm leading-6 text-[color:var(--muted-foreground)]">
            {finding.summary}
          </p>
        </div>

        <div className="grid gap-4 md:grid-cols-2">
          <Card className="space-y-3 p-4">
            <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
              Operator impact
            </h3>
            <p className="text-sm leading-6 text-[color:var(--foreground)]">
              {finding.impact}
            </p>
          </Card>

          <Card className="space-y-3 p-4">
            <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
              Recommended action
            </h3>
            <p className="text-sm leading-6 text-[color:var(--foreground)]">
              {finding.recommendedAction}
            </p>
          </Card>
        </div>

        <Card className="space-y-3 p-4">
          <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
            Evidence
          </h3>
          <div className="space-y-2">
            {finding.evidence.map((item) => (
              <div
                key={item}
                className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface-subtle)] px-3 py-2 text-sm text-[color:var(--foreground)]"
              >
                {item}
              </div>
            ))}
          </div>
        </Card>
      </div>
    </SectionCard>
  );
}
