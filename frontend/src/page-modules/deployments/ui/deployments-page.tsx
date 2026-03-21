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

export function DeploymentsPage() {
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
          setSelectedJobId((current) => current ?? response.items[0]?.job_id ?? null);
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

  return (
    <div className="space-y-6">
      <PageHeader
        title="Deployments"
        description="Live deployment jobs, attempts, targets, steps, and image rollout state from deployment-plane."
        breadcrumbs={[
          { label: dictionary.common.dashboard, href: "#" },
          { label: "Deployments" },
        ]}
      />

      {loading ? <LoadingCard label="Loading deployments..." /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error ? (
        <section className="grid gap-4 xl:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)]">
          <Card>
            <div className="space-y-3">
              <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                Jobs
              </h2>
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
                    jobs.map((job) => (
                      <TableRow
                        key={job.job_id}
                        className={
                          job.job_id === selectedJobId
                            ? "bg-[color:rgba(56,189,248,0.08)]"
                            : undefined
                        }
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
                    ))
                  )}
                </TableBody>
              </Table>
            </div>
          </Card>

          <Card>
            <div className="space-y-4">
              <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                Job inspector
              </h2>
              {detailLoading ? (
                <LoadingCard label="Loading deployment detail..." />
              ) : detail ? (
                <>
                  <div className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3">
                    <p className="text-lg font-semibold text-[color:var(--foreground)]">
                      {detail.item.job_type} / {detail.item.status}
                    </p>
                    <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                      {detail.item.job_id}
                    </p>
                  </div>

                  {imageFlow ? <DeploymentImagePanel imageFlow={imageFlow} /> : null}

                  <div className="space-y-2">
                    <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                      Attempts
                    </h3>
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
                          className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3 text-sm"
                        >
                          Attempt #{attempt.attempt_no} / {attempt.status}
                        </div>
                      ))
                    )}
                  </div>

                  <div className="space-y-2">
                    <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                      Targets
                    </h3>
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
                          className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3"
                        >
                          <p className="text-sm font-medium text-[color:var(--foreground)]">
                            {target.hostname_snapshot} / {target.status}
                          </p>
                          {target.artifact ? (
                            <p className="mt-1 text-xs text-[color:var(--muted-foreground)]">
                              {target.artifact.version} / {target.artifact.package_type}
                            </p>
                          ) : null}
                        </div>
                      ))
                    )}
                  </div>

                  <div className="space-y-2">
                    <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                      Latest step payload
                    </h3>
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
                </>
              ) : (
                <EmptyState
                  variant="flush"
                  title="No job selected"
                  description="Pick a deployment job to inspect attempts, targets, steps, and rollout phases."
                />
              )}
            </div>
          </Card>
        </section>
      ) : null}
    </div>
  );
}
