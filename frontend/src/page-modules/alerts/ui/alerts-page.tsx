"use client";

import { useEffect, useState } from "react";
import { useI18n } from "@/src/shared/lib/i18n";
import {
  listAlertRules,
  listAlerts,
  type AlertInstanceItem,
  type AlertRuleItem,
} from "@/src/shared/lib/runtime-api";
import {
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

export function AlertsPage() {
  const { dictionary } = useI18n();
  const [rules, setRules] = useState<AlertRuleItem[]>([]);
  const [instances, setInstances] = useState<AlertInstanceItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [rulesResponse, alertsResponse] = await Promise.all([
          listAlertRules({ limit: 20, offset: 0 }),
          listAlerts({ limit: 20, offset: 0 }),
        ]);
        if (!cancelled) {
          setRules(rulesResponse.items);
          setInstances(alertsResponse.items);
        }
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
  }, []);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Alerts"
        description="Live alert instances and alert rules backed by query-alert-plane."
        breadcrumbs={[
          { label: dictionary.common.dashboard, href: "#" },
          { label: "Alerts" },
        ]}
      />

      {loading ? <LoadingCard label="Loading alerts..." /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error ? (
        <section className="grid gap-4 xl:grid-cols-2">
          <Card>
            <div className="space-y-3">
              <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                Alert instances
              </h2>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Title</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Severity</TableHead>
                    <TableHead>Host</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {instances.length === 0 ? (
                    <TableRow>
                      <TableCell colSpan={4}>
                        <EmptyState
                          variant="flush"
                          title="No alert instances"
                          description="Triggered alerts will appear here."
                        />
                      </TableCell>
                    </TableRow>
                  ) : (
                    instances.map((item) => (
                      <TableRow key={item.alert_instance_id}>
                        <TableCell className="font-medium text-[color:var(--foreground)]">
                          {item.title}
                        </TableCell>
                        <TableCell>{item.status}</TableCell>
                        <TableCell>{item.severity}</TableCell>
                        <TableCell>{item.host}</TableCell>
                      </TableRow>
                    ))
                  )}
                </TableBody>
              </Table>
            </div>
          </Card>

          <Card>
            <div className="space-y-3">
              <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                Alert rules
              </h2>
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
                  {rules.length === 0 ? (
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
                    rules.map((rule) => (
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
            </div>
          </Card>
        </section>
      ) : null}
    </div>
  );
}
