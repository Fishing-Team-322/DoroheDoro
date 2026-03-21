"use client";

import { useState } from "react";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
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

export function DeploymentDetailsPage({ id }: { id: string }) {
  const { locale } = useI18n();
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
          title: "Retry requested",
          description: `Deployment ${summary.id} was sent to the retry endpoint.`,
          variant: "success",
        });
      } else {
        await cancelDeployment(summary.id);
        showToast({
          title: "Cancellation requested",
          description: `Deployment ${summary.id} was sent to the cancel endpoint.`,
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
        title="Deployment Details"
        description="Inspect job summary, current status, attempts, targets, and steps. When there is no live deployment stream endpoint, this page polls the public details endpoint."
        breadcrumbs={[
          { label: "Deployments", href: withLocalePath(locale, "/deployments") },
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
              Retry
            </Button>
            <Button
              variant="danger"
              size="sm"
              className="h-10 px-4"
              disabled={!canCancel || actionLoading !== null}
              loading={actionLoading === "cancel"}
              onClick={() => void handleAction("cancel")}
            >
              Cancel
            </Button>
          </div>
        }
      />

      {deploymentQuery.isLoading && !deploymentQuery.data ? (
        <LoadingState label="Loading deployment details..." />
      ) : deploymentQuery.error && !deploymentQuery.data ? (
        <ErrorState error={deploymentQuery.error} retry={() => void deploymentQuery.refetch()} />
      ) : (
        <PageStack>
          {actionError ? <ErrorState error={actionError as never} /> : null}

          <SectionCard title="Summary" description="`GET /api/v1/deployments/{id}`">
            <div className="space-y-4">
              <DetailGrid
                items={[
                  { label: "Deployment ID", value: formatMaybeValue(summary?.id) },
                  {
                    label: "Status",
                    value: <StatusBadge value={summary?.status} />,
                  },
                  {
                    label: "Policy ID",
                    value: formatMaybeValue(summary?.policyId),
                  },
                  {
                    label: "Job Type",
                    value: formatMaybeValue(summary?.jobType),
                  },
                  {
                    label: "Created At",
                    value: formatDateTime(summary?.createdAt),
                  },
                  {
                    label: "Current Phase",
                    value: formatMaybeValue(summary?.currentPhase),
                  },
                  {
                    label: "Attempts",
                    value: formatNumber(summary?.attemptCount),
                  },
                  {
                    label: "Targets",
                    value: formatNumber(
                      summary?.totalTargets ?? summary?.agentIds.length
                    ),
                  },
                  {
                    label: "Params",
                    value: formatParamsSummary(summary?.params),
                  },
                ]}
              />
              <div className="flex flex-wrap gap-2 text-xs text-[color:var(--muted-foreground)]">
                {summary?.pendingTargets != null ? (
                  <span>Pending: {formatNumber(summary.pendingTargets)}</span>
                ) : null}
                {summary?.runningTargets != null ? (
                  <span>Running: {formatNumber(summary.runningTargets)}</span>
                ) : null}
                {summary?.succeededTargets != null ? (
                  <span>Succeeded: {formatNumber(summary.succeededTargets)}</span>
                ) : null}
                {summary?.failedTargets != null ? (
                  <span>Failed: {formatNumber(summary.failedTargets)}</span>
                ) : null}
                {summary?.cancelledTargets != null ? (
                  <span>Cancelled: {formatNumber(summary.cancelledTargets)}</span>
                ) : null}
                <span>
                  Refetch cadence:{" "}
                  {isTerminalDeploymentStatus(summary?.status)
                    ? "manual + 5s polling fallback"
                    : "5s polling"}
                </span>
              </div>
              <RequestMetaLine meta={deploymentQuery.meta} />
            </div>
          </SectionCard>

          <SectionCard title="Attempts" description="Current attempt history returned by the details endpoint.">
            <DataTable
              columns={[
                "Attempt",
                "Status",
                "Triggered by",
                "Reason",
                "Started",
                "Finished",
              ]}
              isEmpty={(deploymentQuery.data?.attempts.length ?? 0) === 0}
              rows={(deploymentQuery.data?.attempts ?? []).map((attempt) => (
                <TableRow key={attempt.id}>
                  <TableCell>{formatMaybeValue(attempt.attemptNo)}</TableCell>
                  <TableCell>
                    <StatusBadge value={attempt.status} />
                  </TableCell>
                  <TableCell>{formatMaybeValue(attempt.triggeredBy)}</TableCell>
                  <TableCell>{formatMaybeValue(attempt.reason)}</TableCell>
                  <TableCell>{formatDateTime(attempt.startedAt)}</TableCell>
                  <TableCell>{formatDateTime(attempt.finishedAt)}</TableCell>
                </TableRow>
              ))}
              emptyTitle="No attempts were returned."
            />
          </SectionCard>

          <SectionCard title="Targets" description="Target state for the current or latest attempt.">
            <DataTable
              columns={["Host", "Host ID", "Status", "Started", "Finished", "Error"]}
              isEmpty={(deploymentQuery.data?.targets.length ?? 0) === 0}
              rows={(deploymentQuery.data?.targets ?? []).map((target) => (
                <TableRow key={target.id}>
                  <TableCell>{formatMaybeValue(target.hostname)}</TableCell>
                  <TableCell className="font-mono text-xs text-[color:var(--muted-foreground)]">
                    {formatMaybeValue(target.hostId)}
                  </TableCell>
                  <TableCell>
                    <StatusBadge value={target.status} />
                  </TableCell>
                  <TableCell>{formatDateTime(target.startedAt)}</TableCell>
                  <TableCell>{formatDateTime(target.finishedAt)}</TableCell>
                  <TableCell>{formatMaybeValue(target.errorMessage)}</TableCell>
                </TableRow>
              ))}
              emptyTitle="No targets were returned."
            />
          </SectionCard>

          <SectionCard title="Steps" description="Execution steps exposed by the deployment details endpoint.">
            <DataTable
              columns={["Step", "Status", "Target ID", "Updated", "Message"]}
              isEmpty={(deploymentQuery.data?.steps.length ?? 0) === 0}
              rows={(deploymentQuery.data?.steps ?? []).map((step) => (
                <TableRow key={step.id}>
                  <TableCell className="font-medium text-[color:var(--foreground)]">
                    {formatMaybeValue(step.name)}
                  </TableCell>
                  <TableCell>
                    <StatusBadge value={step.status} />
                  </TableCell>
                  <TableCell className="font-mono text-xs text-[color:var(--muted-foreground)]">
                    {formatMaybeValue(step.targetId)}
                  </TableCell>
                  <TableCell>{formatDateTime(step.updatedAt)}</TableCell>
                  <TableCell>{formatMaybeValue(step.message)}</TableCell>
                </TableRow>
              ))}
              emptyTitle="No steps were returned."
            />
          </SectionCard>

          <SectionCard title="Raw Details" description="Shows the exact payload for contract debugging.">
            <JsonPreview value={deploymentQuery.data?.raw} />
          </SectionCard>
        </PageStack>
      )}
    </PageStack>
  );
}
