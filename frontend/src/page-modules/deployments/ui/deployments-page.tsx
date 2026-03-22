"use client";

import { useEffect, useMemo, useState } from "react";
import { useI18n } from "@/src/shared/lib/i18n";
import {
  deriveDeploymentImageFlow,
  type DeploymentImageFlow,
} from "@/src/shared/lib/operations-workbench";
import {
  getDeployment,
  listDeployments,
  type DeploymentDetailResponse,
  type DeploymentJobItem,
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
import {
  ErrorCard,
  JsonValue,
  LoadingCard,
} from "@/src/page-modules/common/ui/runtime-state";
import { DeploymentImagePanel } from "./deployment-image-panel";

export function DeploymentsPage({
  embedded = false,
}: {
  embedded?: boolean;
} = {}) {
  const { dictionary } = useI18n();
  const [jobs, setJobs] = useState<DeploymentJobItem[]>([]);
  const [selectedJobId, setSelectedJobId] = useState<string | null>(null);
  const [detail, setDetail] = useState<DeploymentDetailResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [detailLoading, setDetailLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);

      try {
        const response = await listDeployments();

        if (!cancelled) {
          setJobs(response.items);
          setSelectedJobId(
            (current) => current ?? response.items[0]?.job_id ?? null
          );
        }
      } catch (loadError) {
        if (!cancelled) {
          setError(
            loadError instanceof Error
              ? loadError.message
              : "Failed to load deployments"
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

    async function loadDetail() {
      if (!selectedJobId) {
        setDetail(null);
        return;
      }

      setDetailLoading(true);

      try {
        const response = await getDeployment(selectedJobId);

        if (!cancelled) {
          setDetail(response);
        }
      } catch (loadError) {
        if (!cancelled) {
          setError(
            loadError instanceof Error
              ? loadError.message
              : "Failed to load deployment detail"
          );
        }
      } finally {
        if (!cancelled) {
          setDetailLoading(false);
        }
      }
    }

    void loadDetail();

    return () => {
      cancelled = true;
    };
  }, [selectedJobId]);

  const imageFlow: DeploymentImageFlow | null = useMemo(() => {
    return detail ? deriveDeploymentImageFlow(detail) : null;
  }, [detail]);

  const content = (
    <div className="space-y-6">
      <div className={embedded ? "space-y-1" : "space-y-2"}>
      </div>

      {loading ? <LoadingCard label="Loading deployments..." /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error ? (
        <div className="grid gap-6 xl:grid-cols-[minmax(0,1.15fr)_minmax(0,1fr)]">
          <section className="space-y-4 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
            <div className="space-y-1">
              <h3 className="text-xl font-semibold text-[color:var(--foreground)]">
                Jobs
              </h3>
              <p className="text-lg text-[color:var(--muted-foreground)]">
                Select a deployment job to inspect its rollout state and runtime
                details.
              </p>
            </div>

            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Job</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Targets</TableHead>
                  <TableHead>Executor</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {jobs.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={4}>
                      <EmptyState
                        variant="flush"
                        title="No deployment jobs"
                        description="Create plans and jobs from WEB or the HTTP API."
                      />
                    </TableCell>
                  </TableRow>
                ) : (
                  jobs.map((job) => {
                    const isSelected = job.job_id === selectedJobId;

                    return (
                      <TableRow
                        key={job.job_id}
                        className={`cursor-pointer transition-colors ${
                          isSelected
                            ? "bg-[color:rgba(56,189,248,0.08)]"
                            : "hover:bg-[color:rgba(255,255,255,0.02)]"
                        }`}
                        onClick={() => setSelectedJobId(job.job_id)}
                      >
                        <TableCell className="font-medium text-[color:var(--foreground)]">
                          {job.job_type}
                          <div className="mt-1 text-xs text-[color:var(--muted-foreground)]">
                            {job.job_id}
                          </div>
                        </TableCell>
                        <TableCell>{job.status}</TableCell>
                        <TableCell>{job.total_targets}</TableCell>
                        <TableCell>{job.executor_kind}</TableCell>
                      </TableRow>
                    );
                  })
                )}
              </TableBody>
            </Table>
          </section>

          <section className="space-y-4 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
            <div className="space-y-1">
              <h3 className="text-xl font-semibold text-[color:var(--foreground)]">
                Job inspector
              </h3>
              <p className="text-base text-[color:var(--muted-foreground)]">
                Attempts, targets, latest step payload, and rollout image flow.
              </p>
            </div>

            {detailLoading ? (
              <LoadingCard label="Loading deployment detail..." />
            ) : detail ? (
              <div className="space-y-4">
                <div className="rounded-xl border border-[color:var(--border)] bg-[color:var(--background)] p-4">
                  <p className="text-lg font-semibold text-[color:var(--foreground)]">
                    {detail.item.job_type} / {detail.item.status}
                  </p>
                  <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                    {detail.item.job_id}
                  </p>
                </div>

                {imageFlow ? (
                  <DeploymentImagePanel imageFlow={imageFlow} />
                ) : null}

                <div className="space-y-2">
                  <h4 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                    Attempts
                  </h4>

                  {detail.attempts.length === 0 ? (
                    <EmptyState
                      variant="flush"
                      title="No attempts"
                      description="Attempts will appear after the job enters execution."
                    />
                  ) : (
                    detail.attempts.map((attempt) => (
                      <div
                        key={attempt.deployment_attempt_id}
                        className="rounded-xl border border-[color:var(--border)] bg-[color:var(--background)] p-3 text-sm text-[color:var(--foreground)]"
                      >
                        Attempt #{attempt.attempt_no} / {attempt.status}
                      </div>
                    ))
                  )}
                </div>

                <div className="space-y-2">
                  <h4 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                    Targets
                  </h4>

                  {detail.targets.length === 0 ? (
                    <EmptyState
                      variant="flush"
                      title="No targets"
                      description="Targets will appear after job execution begins."
                    />
                  ) : (
                    detail.targets.map((target) => (
                      <div
                        key={target.deployment_target_id}
                        className="rounded-xl border border-[color:var(--border)] bg-[color:var(--background)] p-3"
                      >
                        <p className="text-sm font-medium text-[color:var(--foreground)]">
                          {target.hostname_snapshot} / {target.status}
                        </p>

                        {target.artifact ? (
                          <p className="mt-1 text-xs text-[color:var(--muted-foreground)]">
                            {target.artifact.version} /{" "}
                            {target.artifact.package_type}
                          </p>
                        ) : null}
                      </div>
                    ))
                  )}
                </div>

                <div className="space-y-2">
                  <h4 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                    Latest step payload
                  </h4>

                  {detail.steps[0] ? (
                    <JsonValue value={detail.steps[0].payload_json} />
                  ) : (
                    <EmptyState
                      variant="flush"
                      title="No steps yet"
                      description="Step payloads appear once execution starts."
                    />
                  )}
                </div>
              </div>
            ) : (
              <EmptyState
                variant="flush"
                title="No job selected"
                description="Pick a deployment job to inspect attempts, targets, steps, and rollout phases."
              />
            )}
          </section>
        </div>
      ) : null}
    </div>
  );

  return (
    <div className={embedded ? "space-y-4" : "space-y-6"}>
      {!embedded ? (
        <PageHeader
          title="Deployments"
          description="Live deployment jobs, attempts, targets, steps, and image rollout state from deployment-plane."
          breadcrumbs={[
            { label: dictionary.common.dashboard, href: "#" },
            { label: "Deployments" },
          ]}
        />
      ) : null}

      {embedded ? <div>{content}</div> : <Card className="p-6">{content}</Card>}
    </div>
  );
}