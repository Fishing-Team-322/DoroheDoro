"use client";

import { useState } from "react";
import {
  translateValueLabel,
  useI18n,
  withLocalePath,
} from "@/src/shared/lib/i18n";
import { Button, TableCell, TableRow, useToast } from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import {
  cancelDeployment,
  canCancelDeployment,
  canRetryDeployment,
  getDeployment,
  isTerminalDeploymentStatus,
  retryDeployment,
} from "../api";
import { useApiQuery } from "../model";
import {
  DataTable,
  DetailGrid,
  ErrorState,
  JsonPreview,
  LoadingState,
  PageStack,
  RequestMetaLine,
  SectionCard,
  StatusBadge,
  formatDateTime,
  formatMaybeValue,
  formatNumber,
  formatParamsSummary,
} from "./operations-ui";

const copyByLocale = {
  en: {
    retryRequested: {
      title: "Retry requested",
      description: (id: string) =>
        `Deployment ${id} was sent to the retry endpoint.`,
    },
    cancelRequested: {
      title: "Cancellation requested",
      description: (id: string) =>
        `Deployment ${id} was sent to the cancel endpoint.`,
    },
    page: {
      title: "Deployment Details",
      description:
        "Inspect job summary, current status, attempts, targets, and steps. When there is no live deployment stream endpoint, this page polls the public details endpoint.",
      breadcrumbs: "Deployments",
      retry: "Retry",
      cancel: "Cancel",
    },
    loading: "Loading deployment details...",
    summary: {
      title: "Summary",
      description: "`GET /api/v1/deployments/{id}`",
      fields: {
        deploymentId: "Deployment ID",
        status: "Status",
        policyId: "Policy ID",
        jobType: "Job Type",
        createdAt: "Created At",
        currentPhase: "Current Phase",
        attempts: "Attempts",
        targets: "Targets",
        params: "Params",
      },
      counters: {
        pending: "Pending",
        running: "Running",
        succeeded: "Succeeded",
        failed: "Failed",
        cancelled: "Cancelled",
        cadence: "Refetch cadence",
        terminal: "manual + 5s polling fallback",
        polling: "5s polling",
      },
    },
    attempts: {
      title: "Attempts",
      description:
        "Current attempt history returned by the details endpoint.",
      columns: ["Attempt", "Status", "Triggered by", "Reason", "Started", "Finished"],
      empty: "No attempts were returned.",
    },
    targets: {
      title: "Targets",
      description: "Target state for the current or latest attempt.",
      columns: ["Host", "Host ID", "Status", "Started", "Finished", "Error"],
      empty: "No targets were returned.",
    },
    steps: {
      title: "Steps",
      description:
        "Execution steps exposed by the deployment details endpoint.",
      columns: ["Step", "Status", "Target ID", "Updated", "Message"],
      empty: "No steps were returned.",
    },
    raw: {
      title: "Raw Details",
      description: "Shows the exact payload for contract debugging.",
    },
  },
  ru: {
    retryRequested: {
      title: "Повтор запрошен",
      description: (id: string) =>
        `Раскатка ${id} отправлена в retry endpoint.`,
    },
    cancelRequested: {
      title: "Отмена запрошена",
      description: (id: string) =>
        `Раскатка ${id} отправлена в cancel endpoint.`,
    },
    page: {
      title: "Детали раскатки",
      description:
        "Посмотрите сводку задачи, текущий статус, попытки, таргеты и шаги. Если live deployment stream endpoint отсутствует, страница опрашивает публичный endpoint деталей.",
      breadcrumbs: "Раскатки",
      retry: "Повторить",
      cancel: "Отменить",
    },
    loading: "Загрузка деталей раскатки...",
    summary: {
      title: "Сводка",
      description: "`GET /api/v1/deployments/{id}`",
      fields: {
        deploymentId: "ID раскатки",
        status: "Статус",
        policyId: "ID политики",
        jobType: "Тип задачи",
        createdAt: "Создано",
        currentPhase: "Текущая фаза",
        attempts: "Попытки",
        targets: "Таргеты",
        params: "Параметры",
      },
      counters: {
        pending: "Ожидают",
        running: "Выполняются",
        succeeded: "Успешно",
        failed: "Ошибки",
        cancelled: "Отменено",
        cadence: "Интервал обновления",
        terminal: "вручную + fallback polling каждые 5с",
        polling: "polling каждые 5с",
      },
    },
    attempts: {
      title: "Попытки",
      description:
        "История попыток, которую вернул endpoint деталей.",
      columns: ["Попытка", "Статус", "Кем запущено", "Причина", "Старт", "Финиш"],
      empty: "Попытки не были возвращены.",
    },
    targets: {
      title: "Таргеты",
      description: "Состояние таргетов для текущей или последней попытки.",
      columns: ["Хост", "ID хоста", "Статус", "Старт", "Финиш", "Ошибка"],
      empty: "Таргеты не были возвращены.",
    },
    steps: {
      title: "Шаги",
      description:
        "Шаги выполнения, которые вернул endpoint деталей раскатки.",
      columns: ["Шаг", "Статус", "ID таргета", "Обновлено", "Сообщение"],
      empty: "Шаги не были возвращены.",
    },
    raw: {
      title: "Сырые детали",
      description: "Показывает точный payload для отладки контракта.",
    },
  },
} as const;

export function DeploymentDetailsPage({ id }: { id: string }) {
  const { locale } = useI18n();
  const copy = copyByLocale[locale];
  const { showToast } = useToast();
  const [actionError, setActionError] = useState<unknown>();
  const [actionLoading, setActionLoading] = useState<"retry" | "cancel" | null>(
    null
  );

  const deploymentQuery = useApiQuery({
    queryFn: (signal) => getDeployment(id, signal),
    deps: [id],
    pollIntervalMs: 5_000,
  });

  const summary = deploymentQuery.data?.summary;
  const canRetry = canRetryDeployment(summary?.status);
  const canCancel = canCancelDeployment(summary?.status);

  const handleAction = async (action: "retry" | "cancel") => {
    if (!summary) {
      return;
    }

    setActionLoading(action);
    setActionError(undefined);

    try {
      if (action === "retry") {
        await retryDeployment(summary.id);
        showToast({
          title: copy.retryRequested.title,
          description: copy.retryRequested.description(summary.id),
          variant: "success",
        });
      } else {
        await cancelDeployment(summary.id);
        showToast({
          title: copy.cancelRequested.title,
          description: copy.cancelRequested.description(summary.id),
          variant: "success",
        });
      }

      await deploymentQuery.refetch({ silent: true });
    } catch (caughtError) {
      setActionError(caughtError);
    } finally {
      setActionLoading(null);
    }
  };

  return (
    <PageStack>
      <PageHeader
        title={copy.page.title}
        description={copy.page.description}
        breadcrumbs={[
          {
            label: copy.page.breadcrumbs,
            href: withLocalePath(locale, "/deployments"),
          },
          { label: id },
        ]}
        action={
          <div className="flex flex-wrap gap-2">
            <Button
              variant="outline"
              size="sm"
              className="h-10 px-4"
              disabled={!canRetry || actionLoading !== null}
              loading={actionLoading === "retry"}
              onClick={() => void handleAction("retry")}
            >
              {copy.page.retry}
            </Button>
            <Button
              variant="danger"
              size="sm"
              className="h-10 px-4"
              disabled={!canCancel || actionLoading !== null}
              loading={actionLoading === "cancel"}
              onClick={() => void handleAction("cancel")}
            >
              {copy.page.cancel}
            </Button>
          </div>
        }
      />

      {deploymentQuery.isLoading && !deploymentQuery.data ? (
        <LoadingState label={copy.loading} />
      ) : deploymentQuery.error && !deploymentQuery.data ? (
        <ErrorState error={deploymentQuery.error} retry={() => void deploymentQuery.refetch()} />
      ) : (
        <PageStack>
          {actionError ? <ErrorState error={actionError as never} /> : null}

          <SectionCard
            title={copy.summary.title}
            description={copy.summary.description}
          >
            <div className="space-y-4">
              <DetailGrid
                items={[
                  {
                    label: copy.summary.fields.deploymentId,
                    value: formatMaybeValue(summary?.id, locale),
                  },
                  {
                    label: copy.summary.fields.status,
                    value: <StatusBadge value={summary?.status} />,
                  },
                  {
                    label: copy.summary.fields.policyId,
                    value: formatMaybeValue(summary?.policyId, locale),
                  },
                  {
                    label: copy.summary.fields.jobType,
                    value: translateValueLabel(summary?.jobType, locale),
                  },
                  {
                    label: copy.summary.fields.createdAt,
                    value: formatDateTime(summary?.createdAt, locale),
                  },
                  {
                    label: copy.summary.fields.currentPhase,
                    value: translateValueLabel(summary?.currentPhase, locale),
                  },
                  {
                    label: copy.summary.fields.attempts,
                    value: formatNumber(summary?.attemptCount, locale),
                  },
                  {
                    label: copy.summary.fields.targets,
                    value: formatNumber(
                      summary?.totalTargets ?? summary?.agentIds.length,
                      locale
                    ),
                  },
                  {
                    label: copy.summary.fields.params,
                    value: formatParamsSummary(summary?.params, locale),
                  },
                ]}
              />
              <div className="flex flex-wrap gap-2 text-xs text-[color:var(--muted-foreground)]">
                {summary?.pendingTargets != null ? (
                  <span>
                    {copy.summary.counters.pending}:{" "}
                    {formatNumber(summary.pendingTargets, locale)}
                  </span>
                ) : null}
                {summary?.runningTargets != null ? (
                  <span>
                    {copy.summary.counters.running}:{" "}
                    {formatNumber(summary.runningTargets, locale)}
                  </span>
                ) : null}
                {summary?.succeededTargets != null ? (
                  <span>
                    {copy.summary.counters.succeeded}:{" "}
                    {formatNumber(summary.succeededTargets, locale)}
                  </span>
                ) : null}
                {summary?.failedTargets != null ? (
                  <span>
                    {copy.summary.counters.failed}:{" "}
                    {formatNumber(summary.failedTargets, locale)}
                  </span>
                ) : null}
                {summary?.cancelledTargets != null ? (
                  <span>
                    {copy.summary.counters.cancelled}:{" "}
                    {formatNumber(summary.cancelledTargets, locale)}
                  </span>
                ) : null}
                <span>
                  {copy.summary.counters.cadence}:{" "}
                  {isTerminalDeploymentStatus(summary?.status)
                    ? copy.summary.counters.terminal
                    : copy.summary.counters.polling}
                </span>
              </div>
              <RequestMetaLine meta={deploymentQuery.meta} />
            </div>
          </SectionCard>

          <SectionCard
            title={copy.attempts.title}
            description={copy.attempts.description}
          >
            <DataTable
              columns={[...copy.attempts.columns]}
              isEmpty={(deploymentQuery.data?.attempts.length ?? 0) === 0}
              rows={(deploymentQuery.data?.attempts ?? []).map((attempt) => (
                <TableRow key={attempt.id}>
                  <TableCell>{formatMaybeValue(attempt.attemptNo, locale)}</TableCell>
                  <TableCell>
                    <StatusBadge value={attempt.status} />
                  </TableCell>
                  <TableCell>{translateValueLabel(attempt.triggeredBy, locale)}</TableCell>
                  <TableCell>{formatMaybeValue(attempt.reason, locale)}</TableCell>
                  <TableCell>{formatDateTime(attempt.startedAt, locale)}</TableCell>
                  <TableCell>{formatDateTime(attempt.finishedAt, locale)}</TableCell>
                </TableRow>
              ))}
              emptyTitle={copy.attempts.empty}
            />
          </SectionCard>

          <SectionCard
            title={copy.targets.title}
            description={copy.targets.description}
          >
            <DataTable
              columns={[...copy.targets.columns]}
              isEmpty={(deploymentQuery.data?.targets.length ?? 0) === 0}
              rows={(deploymentQuery.data?.targets ?? []).map((target) => (
                <TableRow key={target.id}>
                  <TableCell>{formatMaybeValue(target.hostname, locale)}</TableCell>
                  <TableCell className="font-mono text-xs text-[color:var(--muted-foreground)]">
                    {formatMaybeValue(target.hostId, locale)}
                  </TableCell>
                  <TableCell>
                    <StatusBadge value={target.status} />
                  </TableCell>
                  <TableCell>{formatDateTime(target.startedAt, locale)}</TableCell>
                  <TableCell>{formatDateTime(target.finishedAt, locale)}</TableCell>
                  <TableCell>{formatMaybeValue(target.errorMessage, locale)}</TableCell>
                </TableRow>
              ))}
              emptyTitle={copy.targets.empty}
            />
          </SectionCard>

          <SectionCard
            title={copy.steps.title}
            description={copy.steps.description}
          >
            <DataTable
              columns={[...copy.steps.columns]}
              isEmpty={(deploymentQuery.data?.steps.length ?? 0) === 0}
              rows={(deploymentQuery.data?.steps ?? []).map((step) => (
                <TableRow key={step.id}>
                  <TableCell className="font-medium text-[color:var(--foreground)]">
                    {formatMaybeValue(step.name, locale)}
                  </TableCell>
                  <TableCell>
                    <StatusBadge value={step.status} />
                  </TableCell>
                  <TableCell className="font-mono text-xs text-[color:var(--muted-foreground)]">
                    {formatMaybeValue(step.targetId, locale)}
                  </TableCell>
                  <TableCell>{formatDateTime(step.updatedAt, locale)}</TableCell>
                  <TableCell>{formatMaybeValue(step.message, locale)}</TableCell>
                </TableRow>
              ))}
              emptyTitle={copy.steps.empty}
            />
          </SectionCard>

          <SectionCard
            title={copy.raw.title}
            description={copy.raw.description}
          >
            <JsonPreview value={deploymentQuery.data?.raw} />
          </SectionCard>
        </PageStack>
      )}
    </PageStack>
  );
}
