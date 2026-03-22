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
import {
  ErrorCard,
  LoadingCard,
} from "@/src/page-modules/common/ui/runtime-state";

const copyByLocale = {
  en: {
    loadError: "Failed to load audit",
    title: "Audit workspace",
    loading: "Loading audit...",
    emptyTitle: "No audit events",
    emptyDescription:
      "Audit entries will appear after control, deployment, enrollment and alert mutations.",
    columns: {
      event: "Event",
      entity: "Entity",
      actor: "Actor",
      created: "Created",
    },
  },
  ru: {
    loadError: "Не удалось загрузить аудит",
    title: "аудит",
    loading: "Загрузка аудита...",
    emptyTitle: "Нет audit-событий",
    emptyDescription:
      "Записи аудита появятся после изменений control, deployment, enrollment и alert-сущностей.",
    columns: {
      event: "Событие",
      entity: "Сущность",
      actor: "Актор",
      created: "Создано",
    },
  },
} as const;

export function AuditPage() {
  const { locale } = useI18n();
  const copy = copyByLocale[locale];
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
          setError(loadError instanceof Error ? loadError.message : copy.loadError);
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
  }, [copy.loadError]);

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="space-y-6">
          <div className="border-b border-[color:var(--border)] pb-6">
            <h2 className="text-5xl font-semibold text-[color:var(--foreground)]">
              {copy.title}
            </h2>
          </div>

          {loading ? <LoadingCard label={copy.loading} /> : null}
          {!loading && error ? <ErrorCard message={error} /> : null}

          {!loading && !error ? (
            <section className="rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
              {items.length === 0 ? (
                <EmptyState
                  variant="flush"
                  title={copy.emptyTitle}
                  description={copy.emptyDescription}
                />
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{copy.columns.event}</TableHead>
                      <TableHead>{copy.columns.entity}</TableHead>
                      <TableHead>{copy.columns.actor}</TableHead>
                      <TableHead>{copy.columns.created}</TableHead>
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
            </section>
          ) : null}
        </div>
      </Card>
    </div>
  );
}
