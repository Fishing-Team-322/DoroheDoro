"use client";

import { useEffect, useState } from "react";
import { useI18n } from "@/src/shared/lib/i18n";
import { listAudit, type AuditEventItem } from "@/src/shared/lib/runtime-api";
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

export function AuditPage() {
  const { dictionary } = useI18n();
  const [items, setItems] = useState<AuditEventItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);
      try {
        const response = await listAudit({ limit: 50, offset: 0 });
        if (!cancelled) {
          setItems(response.items);
        }
      } catch (loadError) {
        if (!cancelled) {
          setError(loadError instanceof Error ? loadError.message : "Failed to load audit");
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
        title="Audit"
        description="Cross-plane state-changing events persisted in the shared runtime audit log."
        breadcrumbs={[
          { label: dictionary.common.dashboard, href: "#" },
          { label: "Audit" },
        ]}
      />

      {loading ? <LoadingCard label="Loading audit..." /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error ? (
        <Card>
          {items.length === 0 ? (
            <EmptyState
              variant="flush"
              title="No audit events"
              description="Audit entries will appear after control, deployment, enrollment and alert mutations."
            />
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Event</TableHead>
                  <TableHead>Entity</TableHead>
                  <TableHead>Actor</TableHead>
                  <TableHead>Created</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {items.map((item) => (
                  <TableRow key={item.audit_event_id}>
                    <TableCell>
                      <div className="font-medium text-[color:var(--foreground)]">
                        {item.event_type}
                      </div>
                      <div className="mt-1 text-xs text-[color:var(--muted-foreground)]">
                        {item.reason}
                      </div>
                    </TableCell>
                    <TableCell>
                      {item.entity_type}
                      <div className="mt-1 text-xs text-[color:var(--muted-foreground)]">
                        {item.entity_id}
                      </div>
                    </TableCell>
                    <TableCell>{item.actor_id}</TableCell>
                    <TableCell>{item.created_at}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </Card>
      ) : null}
    </div>
  );
}
