"use client";

import { useEffect, useState } from "react";
import { useI18n } from "@/src/shared/lib/i18n";
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

export function PoliciesPage() {
  const { dictionary } = useI18n();
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
            loadError instanceof Error ? loadError.message : "Failed to load policies"
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
              : "Failed to load policy revisions"
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
  }, [selectedPolicyId]);

  const selectedPolicy =
    policies.find((item) => item.policy_id === selectedPolicyId) ?? null;

  return (
    <div className="space-y-6">
      <PageHeader
        title="Policies"
        description="Live policies and append-only revisions from control-plane."
        breadcrumbs={[
          { label: dictionary.common.dashboard, href: "#" },
          { label: "Policies" },
        ]}
      />

      {loading ? <LoadingCard label="Loading policies..." /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error ? (
        <section className="grid gap-4 xl:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)]">
          <Card>
            <div className="space-y-3">
              <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                Policies
              </h2>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Name</TableHead>
                    <TableHead>Revision</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Updated</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {policies.length === 0 ? (
                    <TableRow>
                      <TableCell colSpan={4}>
                        <EmptyState
                          variant="flush"
                          title="No policies"
                          description="Create policies through WEB or the API to populate this module."
                        />
                      </TableCell>
                    </TableRow>
                  ) : (
                    policies.map((policy) => (
                      <TableRow
                        key={policy.policy_id}
                        className={
                          policy.policy_id === selectedPolicyId
                            ? "bg-[color:rgba(56,189,248,0.08)]"
                            : undefined
                        }
                        onClick={() => setSelectedPolicyId(policy.policy_id)}
                      >
                        <TableCell className="font-medium text-[color:var(--foreground)]">
                          {policy.name}
                        </TableCell>
                        <TableCell>{policy.latest_revision || "—"}</TableCell>
                        <TableCell>{policy.is_active ? "active" : "inactive"}</TableCell>
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
                Policy inspector
              </h2>

              {selectedPolicy ? (
                <>
                  <div className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3">
                    <p className="text-lg font-semibold text-[color:var(--foreground)]">
                      {selectedPolicy.name}
                    </p>
                    <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                      {selectedPolicy.description || "No description"}
                    </p>
                  </div>
                  <JsonValue value={selectedPolicy.latest_body_json ?? {}} />

                  <div className="space-y-2">
                    <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                      Revisions
                    </h3>
                    {revisionsLoading ? (
                      <LoadingCard label="Loading revisions..." />
                    ) : revisions.length === 0 ? (
                      <EmptyState
                        variant="flush"
                        title="No revisions"
                        description="Revisions will appear after policy updates."
                      />
                    ) : (
                      revisions.map((revision) => (
                        <div
                          key={revision.policy_revision_id}
                          className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3"
                        >
                          <p className="text-sm font-medium text-[color:var(--foreground)]">
                            Revision {revision.revision}
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
                  title="No policy selected"
                  description="Pick a policy to inspect its current body and revisions."
                />
              )}
            </div>
          </Card>
        </section>
      ) : null}
    </div>
  );
}
