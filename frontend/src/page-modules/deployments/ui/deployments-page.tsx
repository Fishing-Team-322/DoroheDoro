"use client";

import { useEffect, useMemo, useState } from "react";
import { translateValueLabel, useI18n } from "@/src/shared/lib/i18n";
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

const copyByLocale = {
  en: {
    loadError: "Failed to load deployments",
    detailError: "Failed to load deployment detail",
    title: "Deployments",
    description:
      "Live deployment jobs, attempts, targets, steps, and image rollout state from deployment-plane.",
    loading: "Loading deployments...",
    jobs: {
      title: "Jobs",
      description:
        "Select a deployment job to inspect its rollout state and runtime details.",
      columns: {
        job: "Job",
        status: "Status",
        targets: "Targets",
        executor: "Executor",
      },
      emptyTitle: "No deployment jobs",
      emptyDescription:
        "Create plans and jobs from WEB or the HTTP API.",
    },
    inspector: {
      title: "Job inspector",
      description:
        "Attempts, targets, latest step payload, and rollout image flow.",
      loading: "Loading deployment detail...",
      attempts: "Attempts",
      attemptsEmptyTitle: "No attempts",
      attemptsEmptyDescription:
        "Attempts will appear after the job enters execution.",
      targets: "Targets",
      targetsEmptyTitle: "No targets",
      targetsEmptyDescription:
        "Targets will appear after job execution begins.",
      latestStep: "Latest step payload",
      latestStepEmptyTitle: "No steps yet",
      latestStepEmptyDescription:
        "Step payloads appear once execution starts.",
      emptyTitle: "No job selected",
      emptyDescription:
        "Pick a deployment job to inspect attempts, targets, steps, and rollout phases.",
      attemptPrefix: "Attempt",
    },
  },
  ru: {
    loadError: "Не удалось загрузить раскатки",
    detailError: "Не удалось загрузить детали раскатки",
    title: "Раскатки",
    description:
      "Живые deployment jobs, attempts, targets, steps и состояние rollout image из deployment-plane.",
    loading: "Загрузка раскаток...",
    jobs: {
      title: "Задачи",
      description:
        "Выберите deployment job, чтобы посмотреть состояние rollout и runtime-детали.",
      columns: {
        job: "Задача",
        status: "Статус",
        targets: "Таргеты",
        executor: "Исполнитель",
      },
      emptyTitle: "Нет deployment jobs",
      emptyDescription:
        "Создайте планы и задачи через WEB или HTTP API.",
    },
    inspector: {
      title: "Инспектор задачи",
      description:
        "Attempts, targets, последний payload шага и flow rollout-образа.",
      loading: "Загрузка деталей раскатки...",
      attempts: "Попытки",
      attemptsEmptyTitle: "Попыток нет",
      attemptsEmptyDescription:
        "Попытки появятся после перехода задачи в execution.",
      targets: "Таргеты",
      targetsEmptyTitle: "Таргетов нет",
      targetsEmptyDescription:
        "Таргеты появятся после начала выполнения задачи.",
      latestStep: "Payload последнего шага",
      latestStepEmptyTitle: "Шагов пока нет",
      latestStepEmptyDescription:
        "Payload шагов появятся после старта выполнения.",
      emptyTitle: "Задача не выбрана",
      emptyDescription:
        "Выберите deployment job, чтобы посмотреть attempts, targets, steps и фазы rollout.",
      attemptPrefix: "Попытка",
    },
  },
} as const;

export function DeploymentsPage({
  embedded = false,
}: {
  embedded?: boolean;
} = {}) {
  const { dictionary, locale } = useI18n();
  const copy = copyByLocale[locale];
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
              : copy.loadError
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
              : copy.detailError
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
  }, [copy.detailError, selectedJobId]);

  const imageFlow: DeploymentImageFlow | null = useMemo(() => {
    return detail ? deriveDeploymentImageFlow(detail, locale) : null;
  }, [detail, locale]);

  const content = (
    <div className="space-y-6">
      <div className={embedded ? "space-y-1" : "space-y-2"}>
      </div>

      {loading ? <LoadingCard label={copy.loading} /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error ? (
        <div className="grid gap-6 xl:grid-cols-[minmax(0,1.15fr)_minmax(0,1fr)]">
          <section className="space-y-4 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
            <div className="space-y-1">
              <h3 className="text-xl font-semibold text-[color:var(--foreground)]">
                {copy.jobs.title}
              </h3>
              <p className="text-lg text-[color:var(--muted-foreground)]">
                {copy.jobs.description}
              </p>
            </div>

            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{copy.jobs.columns.job}</TableHead>
                  <TableHead>{copy.jobs.columns.status}</TableHead>
                  <TableHead>{copy.jobs.columns.targets}</TableHead>
                  <TableHead>{copy.jobs.columns.executor}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {jobs.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={4}>
                      <EmptyState
                        variant="flush"
                        title={copy.jobs.emptyTitle}
                        description={copy.jobs.emptyDescription}
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
                        <TableCell>{translateValueLabel(job.status, locale)}</TableCell>
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
                {copy.inspector.title}
              </h3>
              <p className="text-base text-[color:var(--muted-foreground)]">
                {copy.inspector.description}
              </p>
            </div>

            {detailLoading ? (
              <LoadingCard label={copy.inspector.loading} />
            ) : detail ? (
              <div className="space-y-4">
                <div className="rounded-xl border border-[color:var(--border)] bg-[color:var(--background)] p-4">
                  <p className="text-lg font-semibold text-[color:var(--foreground)]">
                    {detail.item.job_type} /{" "}
                    {translateValueLabel(detail.item.status, locale)}
                  </p>
                  <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                    {detail.item.job_id}
                  </p>
                </div>

                {imageFlow ? (
                  <DeploymentImagePanel imageFlow={imageFlow} locale={locale} />
                ) : null}

                <div className="space-y-2">
                    <h4 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                      {copy.inspector.attempts}
                    </h4>

                  {detail.attempts.length === 0 ? (
                    <EmptyState
                      variant="flush"
                      title={copy.inspector.attemptsEmptyTitle}
                      description={copy.inspector.attemptsEmptyDescription}
                    />
                  ) : (
                    detail.attempts.map((attempt) => (
                      <div
                        key={attempt.deployment_attempt_id}
                        className="rounded-xl border border-[color:var(--border)] bg-[color:var(--background)] p-3 text-sm text-[color:var(--foreground)]"
                      >
                        {copy.inspector.attemptPrefix} #{attempt.attempt_no} /{" "}
                        {translateValueLabel(attempt.status, locale)}
                      </div>
                    ))
                  )}
                </div>

                <div className="space-y-2">
                    <h4 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                      {copy.inspector.targets}
                    </h4>

                  {detail.targets.length === 0 ? (
                    <EmptyState
                      variant="flush"
                      title={copy.inspector.targetsEmptyTitle}
                      description={copy.inspector.targetsEmptyDescription}
                    />
                  ) : (
                    detail.targets.map((target) => (
                      <div
                        key={target.deployment_target_id}
                        className="rounded-xl border border-[color:var(--border)] bg-[color:var(--background)] p-3"
                      >
                        <p className="text-sm font-medium text-[color:var(--foreground)]">
                          {target.hostname_snapshot} /{" "}
                          {translateValueLabel(target.status, locale)}
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
                      {copy.inspector.latestStep}
                    </h4>

                  {detail.steps[0] ? (
                    <JsonValue value={detail.steps[0].payload_json} />
                  ) : (
                    <EmptyState
                      variant="flush"
                      title={copy.inspector.latestStepEmptyTitle}
                      description={copy.inspector.latestStepEmptyDescription}
                    />
                  )}
                </div>
              </div>
            ) : (
              <EmptyState
                variant="flush"
                title={copy.inspector.emptyTitle}
                description={copy.inspector.emptyDescription}
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
          title={copy.title}
          description={copy.description}
          breadcrumbs={[
            { label: dictionary.common.dashboard, href: "#" },
            { label: copy.title },
          ]}
        />
      ) : null}

      {embedded ? <div>{content}</div> : <Card className="p-6">{content}</Card>}
    </div>
  );
}
