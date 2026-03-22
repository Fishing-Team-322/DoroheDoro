"use client";

import { useEffect, useState } from "react";
import { translateValueLabel, useI18n } from "@/src/shared/lib/i18n";
import {
  getPolicyRevisions,
  listPolicies,
  type PolicyItem,
  type PolicyRevisionItem,
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
import { ErrorCard, JsonValue, LoadingCard } from "@/src/page-modules/common/ui/runtime-state";

const copyByLocale = {
  en: {
    loadError: "Failed to load policies",
    revisionsError: "Failed to load policy revisions",
    title: "Policies",
    description: "Live policies and append-only revisions from control-plane.",
    loading: "Loading policies...",
    listTitle: "Policies",
    inspectorTitle: "Policy inspector",
    columns: {
      name: "Name",
      revision: "Revision",
      status: "Status",
      updated: "Updated",
    },
    emptyTitle: "No policies",
    emptyDescription:
      "Create policies through WEB or the API to populate this module.",
    noDescription: "No description",
    revisionsTitle: "Revisions",
    revisionsLoading: "Loading revisions...",
    revisionsEmptyTitle: "No revisions",
    revisionsEmptyDescription: "Revisions will appear after policy updates.",
    revisionPrefix: "Revision",
    emptySelectionTitle: "No policy selected",
    emptySelectionDescription:
      "Pick a policy to inspect its current body and revisions.",
  },
  ru: {
    loadError: "Не удалось загрузить политики",
    revisionsError: "Не удалось загрузить ревизии политики",
    title: "Политики",
    description:
      "Живые политики и append-only ревизии из control-plane.",
    loading: "Загрузка политик...",
    listTitle: "Политики",
    inspectorTitle: "Инспектор политики",
    columns: {
      name: "Имя",
      revision: "Ревизия",
      status: "Статус",
      updated: "Обновлено",
    },
    emptyTitle: "Политик нет",
    emptyDescription:
      "Создайте политики через WEB или API, чтобы заполнить этот модуль.",
    noDescription: "Нет описания",
    revisionsTitle: "Ревизии",
    revisionsLoading: "Загрузка ревизий...",
    revisionsEmptyTitle: "Ревизий нет",
    revisionsEmptyDescription: "Ревизии появятся после обновлений политики.",
    revisionPrefix: "Ревизия",
    emptySelectionTitle: "Политика не выбрана",
    emptySelectionDescription:
      "Выберите политику, чтобы посмотреть текущее тело и ревизии.",
  },
} as const;

export function PoliciesPage({ embedded = false }: { embedded?: boolean } = {}) {
  const { dictionary, locale } = useI18n();
  const copy = copyByLocale[locale];
  const [policies, setPolicies] = useState<PolicyItem[]>([]);
  const [selectedPolicyId, setSelectedPolicyId] = useState<string | null>(null);
  const [revisions, setRevisions] = useState<PolicyRevisionItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [revisionsLoading, setRevisionsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);
      try {
        const response = await listPolicies();
        if (cancelled) {
          return;
        }
        setPolicies(response.items);
        setSelectedPolicyId((current) => current ?? response.items[0]?.policy_id ?? null);
      } catch (loadError) {
        if (!cancelled) {
          setError(
            loadError instanceof Error ? loadError.message : copy.loadError
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
  }, [copy.loadError]);

  useEffect(() => {
    let cancelled = false;

    async function loadRevisions() {
      if (!selectedPolicyId) {
        setRevisions([]);
        return;
      }

      setRevisionsLoading(true);
      try {
        const response = await getPolicyRevisions(selectedPolicyId);
        if (!cancelled) {
          setRevisions(response.items);
        }
      } catch (loadError) {
        if (!cancelled) {
          setError(
            loadError instanceof Error
              ? loadError.message
              : copy.revisionsError
          );
        }
      } finally {
        if (!cancelled) {
          setRevisionsLoading(false);
        }
      }
    }

    void loadRevisions();
    return () => {
      cancelled = true;
    };
  }, [copy.revisionsError, selectedPolicyId]);

  const selectedPolicy =
    policies.find((item) => item.policy_id === selectedPolicyId) ?? null;

  return (
    <div className={embedded ? "space-y-4" : "space-y-6"}>
      {!embedded ? (
        <PageHeader
          title={copy.title}
          description={copy.description}
          breadcrumbs={[
            { label: dictionary.common.dashboard, href: "#" },
            { label: copy.title },
          ]}
        />
      ) : null}

      {loading ? <LoadingCard label={copy.loading} /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error ? (
        <section className="grid gap-4 xl:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)]">
          <Card>
            <div className="space-y-3">
                <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                  {copy.listTitle}
                </h2>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{copy.columns.name}</TableHead>
                    <TableHead>{copy.columns.revision}</TableHead>
                    <TableHead>{copy.columns.status}</TableHead>
                    <TableHead>{copy.columns.updated}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {policies.length === 0 ? (
                    <TableRow>
                      <TableCell colSpan={4}>
                        <EmptyState
                          variant="flush"
                          title={copy.emptyTitle}
                          description={copy.emptyDescription}
                        />
                      </TableCell>
                    </TableRow>
                  ) : (
                    policies.map((policy) => (
                      <TableRow
                        key={policy.policy_id}
                        className={
                          policy.policy_id === selectedPolicyId
                            ? "bg-transparent"
                            : undefined
                        }
                        onClick={() => setSelectedPolicyId(policy.policy_id)}
                      >
                        <TableCell className="font-medium text-[color:var(--foreground)]">
                          {policy.name}
                        </TableCell>
                        <TableCell>{policy.latest_revision || "—"}</TableCell>
                        <TableCell>
                          {translateValueLabel(
                            policy.is_active ? "active" : "inactive",
                            locale
                          )}
                        </TableCell>
                        <TableCell>{policy.updated_at}</TableCell>
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
                  {copy.inspectorTitle}
                </h2>

              {selectedPolicy ? (
                <>
                  <div className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3">
                    <p className="text-lg font-semibold text-[color:var(--foreground)]">
                      {selectedPolicy.name}
                    </p>
                    <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                      {selectedPolicy.description || copy.noDescription}
                    </p>
                  </div>
                  <JsonValue value={selectedPolicy.latest_body_json ?? {}} />

                  <div className="space-y-2">
                    <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                      {copy.revisionsTitle}
                    </h3>
                    {revisionsLoading ? (
                      <LoadingCard label={copy.revisionsLoading} />
                    ) : revisions.length === 0 ? (
                      <EmptyState
                        variant="flush"
                        title={copy.revisionsEmptyTitle}
                        description={copy.revisionsEmptyDescription}
                      />
                    ) : (
                      revisions.map((revision) => (
                        <div
                          key={revision.policy_revision_id}
                          className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3"
                        >
                          <p className="text-sm font-medium text-[color:var(--foreground)]">
                            {copy.revisionPrefix} {revision.revision}
                          </p>
                          <p className="mt-1 text-xs uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                            {revision.created_at}
                          </p>
                        </div>
                      ))
                    )}
                  </div>
                </>
              ) : (
                <EmptyState
                  variant="flush"
                  title={copy.emptySelectionTitle}
                  description={copy.emptySelectionDescription}
                />
              )}
            </div>
          </Card>
        </section>
      ) : null}
    </div>
  );
}
