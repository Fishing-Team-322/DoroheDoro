"use client";

import { useSearchParams } from "next/navigation";
import { useEffect, useMemo, useState } from "react";
import { useI18n } from "@/src/shared/lib/i18n";
import {
  SectionCard,
} from "@/src/features/operations/ui/operations-ui";
import {
  getAlertsWorkbenchData,
  getSeverityTone,
  type AlertsWorkbenchData,
} from "@/src/shared/lib/operations-workbench";
import {
  Badge,
  Card,
  EmptyState,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { ErrorCard, LoadingCard } from "@/src/page-modules/common/ui/runtime-state";
import { AlertExplanationDrawer } from "./alert-explanation-drawer";

function toBadgeVariant(value?: string) {
  const tone = getSeverityTone(value);
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

function isOpenStatus(value?: string) {
  const normalized = value?.trim().toLowerCase() ?? "";
  return !["resolved", "closed", "delivered"].includes(normalized);
}

export function AlertsPage({ embedded = false }: { embedded?: boolean } = {}) {
  const { dictionary, locale } = useI18n();
  const searchParams = useSearchParams();
  const alertParam = searchParams.get("alert");

  const [data, setData] = useState<AlertsWorkbenchData | null>(null);
  const [selectedAlertId, setSelectedAlertId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);
      try {
        const response = await getAlertsWorkbenchData();
        if (cancelled) {
          return;
        }
        setData(response);
        setSelectedAlertId((current) => {
          const requested = response.alerts.find((item) => item.id === alertParam)?.id;
          if (requested) {
            return requested;
          }
          if (response.alerts.some((item) => item.id === current)) {
            return current;
          }
          return response.alerts[0]?.id ?? null;
        });
      } catch (loadError) {
        if (!cancelled) {
          setError(loadError instanceof Error ? loadError.message : "Failed to load alerts");
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
  }, [alertParam]);

  useEffect(() => {
    if (!data || !alertParam) {
      return;
    }

    const requested = data.alerts.find((item) => item.id === alertParam);
    if (requested) {
      setSelectedAlertId(requested.id);
    }
  }, [data, alertParam]);

  const selectedAlert = useMemo(() => {
    return data?.alerts.find((item) => item.id === selectedAlertId) ?? null;
  }, [data?.alerts, selectedAlertId]);

  const openAlertsCount = data?.alerts.filter((item) => isOpenStatus(item.status)).length ?? 0;

  return (
    <div className={embedded ? "space-y-4" : "space-y-6"}>
      {!embedded ? (
        <PageHeader
          title="Alerts"
          description="Live alert instances plus a correlated operator detail view for anomaly, posture, binding, and delivery context."
          breadcrumbs={[
            { label: dictionary.common.dashboard, href: "#" },
            { label: "Alerts" },
          ]}
        />
      ) : null}

      {loading ? <LoadingCard label="Loading alerts..." /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error && data ? (
        <>
          <section className="grid gap-4 md:grid-cols-3">
            <Card className="space-y-2 p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">Open alerts</p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {openAlertsCount}
              </p>
            </Card>
            <Card className="space-y-2 p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">Alert rules</p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {data.rules.length}
              </p>
            </Card>
            <Card className="space-y-2 p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                Routed Telegram instances
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {data.telegramInstances.filter((item) => item.enabled).length}
              </p>
            </Card>
          </section>

          <section className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1.05fr)]">
            <SectionCard
              title="Alert instances"
              description="Pick an alert from the table to open the correlated detail panel."
            >
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Title</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Severity</TableHead>
                    <TableHead>Host</TableHead>
                    <TableHead>Service</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {data.alerts.length === 0 ? (
                    <TableRow>
                      <TableCell colSpan={5}>
                        <EmptyState
                          variant="flush"
                          title="No alert instances"
                          description="Triggered alerts will appear here."
                        />
                      </TableCell>
                    </TableRow>
                  ) : (
                    data.alerts.map((item) => (
                      <TableRow
                        key={item.id}
                        className={
                          item.id === selectedAlertId
                            ? "bg-[color:rgba(56,189,248,0.08)]"
                            : undefined
                        }
                        onClick={() => setSelectedAlertId(item.id)}
                      >
                        <TableCell className="font-medium text-[color:var(--foreground)]">
                          {item.title}
                        </TableCell>
                        <TableCell>
                          <Badge>{item.status}</Badge>
                        </TableCell>
                        <TableCell>
                          <Badge variant={toBadgeVariant(item.severity)}>{item.severity}</Badge>
                        </TableCell>
                        <TableCell>{item.host || "n/a"}</TableCell>
                        <TableCell>{item.service || "n/a"}</TableCell>
                      </TableRow>
                    ))
                  )}
                </TableBody>
              </Table>
            </SectionCard>

            <AlertExplanationDrawer alert={selectedAlert} locale={locale} />
          </section>

          <SectionCard
            title="Alert rules"
            description="The list below preserves the existing rule inventory while the detail experience stays focused on live alert instances."
          >
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Severity</TableHead>
                  <TableHead>Scope</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {data.rules.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={4}>
                      <EmptyState
                        variant="flush"
                        title="No alert rules"
                        description="Create rules through the API to start threshold evaluation."
                      />
                    </TableCell>
                  </TableRow>
                ) : (
                  data.rules.map((rule) => (
                    <TableRow key={rule.alert_rule_id}>
                      <TableCell className="font-medium text-[color:var(--foreground)]">
                        {rule.name}
                      </TableCell>
                      <TableCell>{rule.status}</TableCell>
                      <TableCell>{rule.severity}</TableCell>
                      <TableCell>
                        {rule.scope_type}
                        {rule.scope_id ? `:${rule.scope_id}` : ""}
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </SectionCard>
        </>
      ) : null}
    </div>
  );
}
