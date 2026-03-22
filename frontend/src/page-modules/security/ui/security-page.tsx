"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import { useSearchParams } from "next/navigation";
import type { Locale } from "@/src/shared/config";
import {
  getSecurityPostureData,
  getSeverityTone,
  type SecurityFinding,
  type SecurityPostureData,
} from "@/src/shared/lib/operations-workbench";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
import { AlertsPage } from "@/src/page-modules/alerts";
import { AnomaliesPage } from "@/src/page-modules/anomalies";
import { PoliciesPage } from "@/src/page-modules/policies";
import { Badge, Button, Card, EmptyState } from "@/src/shared/ui";
import {
  NoticeBanner,
  SectionCard,
} from "@/src/features/operations/ui/operations-ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import {
  ErrorCard,
  LoadingCard,
} from "@/src/page-modules/common/ui/runtime-state";
import { SecurityOverviewCard } from "./security-overview-card";

type SecurityTab = "overview" | "findings" | "alerts" | "policies" | "anomalies";

const validTabs: SecurityTab[] = [
  "overview",
  "findings",
  "alerts",
  "policies",
  "anomalies",
];

function getActiveTab(value: string | null): SecurityTab {
  return validTabs.includes(value as SecurityTab)
    ? (value as SecurityTab)
    : "overview";
}

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
  const { locale } = useI18n();
  const searchParams = useSearchParams();
  const activeTab = getActiveTab(searchParams.get("tab"));

  const tabs = useMemo(
    () => [
      {
        id: "overview" as const,
        label: "Overview",
        href: withLocalePath(locale, "/security?tab=overview"),
      },
      {
        id: "findings" as const,
        label: "Findings",
        href: withLocalePath(locale, "/security?tab=findings"),
      },
      {
        id: "alerts" as const,
        label: "Alerts",
        href: withLocalePath(locale, "/security?tab=alerts"),
      },
      {
        id: "policies" as const,
        label: "Policies",
        href: withLocalePath(locale, "/security?tab=policies"),
      },
      {
        id: "anomalies" as const,
        label: "Anomalies",
        href: withLocalePath(locale, "/security?tab=anomalies"),
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
                security workspace
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
            {activeTab === "overview" ? <SecurityOverviewSection /> : null}
            {activeTab === "findings" ? <SecurityFindingsSection /> : null}
            {activeTab === "alerts" ? <AlertsPage embedded /> : null}
            {activeTab === "policies" ? <PoliciesPage embedded /> : null}
            {activeTab === "anomalies" ? <AnomaliesPage embedded /> : null}
          </div>
        </div>
      </Card>
    </div>
  );
}

function SecurityOverviewSection() {
  const posture = useSecurityPosture();

  if (posture.loading) {
    return <LoadingCard label="Loading security posture..." />;
  }

  if (posture.error || !posture.data) {
    return <ErrorCard message={posture.error ?? "Failed to build security posture"} />;
  }

  const topFindings = posture.data.findings.slice(0, 3);

  return (
    <div className="space-y-6">
      <NoticeBanner
        title="Frontend posture synthesis"
        description="A dedicated backend security-posture contract was not found in the current frontend runtime API. This page safely composes posture findings from existing frontend-visible endpoints and keeps operator UX usable in demo mode."
      />

      <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        {posture.data.summary.map((item) => (
          <SecurityOverviewCard key={item.id} item={item} />
        ))}
      </section>

      <SectionCard
        title="Risk cards"
        description="The loudest posture signals stay above the fold so operators can triage without jumping across separate pages."
      >
        {topFindings.length === 0 ? (
          <EmptyState
            variant="flush"
            title="Security posture looks stable"
            description="The current runtime data did not produce active posture findings."
          />
        ) : (
          <div className="grid gap-4 lg:grid-cols-3">
            {topFindings.map((finding) => (
              <Card key={finding.id} className="space-y-3 p-4">
                <div className="flex flex-wrap gap-2">
                  <Badge variant={toBadgeVariant(finding.severity)}>
                    {finding.severity}
                  </Badge>
                  <Badge>{finding.status}</Badge>
                </div>
                <p className="text-base font-semibold text-[color:var(--foreground)]">
                  {finding.title}
                </p>
                <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
                  {finding.summary}
                </p>
              </Card>
            ))}
          </div>
        )}
      </SectionCard>

      <SecurityFindingsWorkbench
        data={posture.data}
        selectedFindingId={posture.selectedFindingId}
        setSelectedFindingId={posture.setSelectedFindingId}
        locale={posture.locale}
        title="Posture overview"
        description="Summary cards, grouped findings, and the current operator detail panel live together here."
      />
    </div>
  );
}

function SecurityFindingsSection() {
  const posture = useSecurityPosture();

  if (posture.loading) {
    return <LoadingCard label="Loading findings..." />;
  }

  if (posture.error || !posture.data) {
    return <ErrorCard message={posture.error ?? "Failed to load findings"} />;
  }

  return (
    <div className="space-y-6">
      <NoticeBanner
        title="Grouped operator findings"
        description="Severity and status remain grouped inside one focused view so operators can review, hand off, and jump to related sections without leaving Security."
      />

      <SecurityFindingsWorkbench
        data={posture.data}
        selectedFindingId={posture.selectedFindingId}
        setSelectedFindingId={posture.setSelectedFindingId}
        locale={posture.locale}
        title="Findings"
        description="Open and watching posture findings, grouped into one operator workbench."
      />
    </div>
  );
}

function SecurityFindingsWorkbench({
  data,
  selectedFindingId,
  setSelectedFindingId,
  locale,
  title,
  description,
}: {
  data: SecurityPostureData;
  selectedFindingId: string | null;
  setSelectedFindingId: (value: string) => void;
  locale: Locale;
  title: string;
  description: string;
}) {
  const selectedFinding =
    data.findings.find((item) => item.id === selectedFindingId) ?? null;

  if (data.findings.length === 0) {
    return (
      <section className="rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
        <EmptyState
          variant="flush"
          title="No findings"
          description="The current runtime data did not produce active posture findings."
        />
      </section>
    );
  }

  return (
    <section className="grid gap-4 xl:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
      <SectionCard title={title} description={description}>
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
      <section className="rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
        <EmptyState
          variant="flush"
          title="No finding selected"
          description="Choose a finding from the list to inspect impact, evidence, and the recommended next action."
        />
      </section>
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
            <Badge variant={toBadgeVariant(finding.severity)}>
              {finding.severity}
            </Badge>
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

function useSecurityPosture() {
  const { locale } = useI18n();
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

  return {
    data,
    selectedFindingId,
    setSelectedFindingId,
    loading,
    error,
    locale,
  };
}