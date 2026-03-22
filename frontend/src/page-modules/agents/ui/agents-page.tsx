"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import type { ApiResponseMeta } from "@/src/shared/lib/api";
import { useApiQuery } from "@/src/features/operations/model";
import {
  DetailGrid,
  ErrorState,
  JsonPreview,
  LoadingState,
  MetricCard,
  NoticeBanner,
  RequestMetaLine,
  SectionCard,
  StatusBadge,
  TextAreaField,
  formatDateTime,
  formatMaybeValue,
  formatNumber,
  formatParamsSummary,
} from "@/src/features/operations/ui/operations-ui";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
import {
  canCancelDeployment,
  canRetryDeployment,
  cancelDeployment,
  createDeployment,
  createDeploymentPlan,
  getDeploymentById,
  getDeployments,
  getPolicies,
  getPolicyById,
  retryDeployment,
  type DeploymentMutationPayload,
  type DeploymentPlan,
} from "@/src/shared/lib/runtime-api";
import {
  Badge,
  Button,
  Card,
  EmptyState,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  useToast,
} from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { AgentEnrollmentDialog } from "./agent-enrollment-dialog";

export function AgentsPage({ embedded = false }: { embedded?: boolean } = {}) {
  const { dictionary, locale } = useI18n();
  const { showToast } = useToast();

  const [createAgentDialogOpen, setCreateAgentDialogOpen] = useState(false);
  const [selectedPolicyId, setSelectedPolicyId] = useState<string | null>(null);
  const [selectedDeploymentId, setSelectedDeploymentId] = useState<
    string | null
  >(null);
  const [agentIdsText, setAgentIdsText] = useState("");
  const [paramsText, setParamsText] = useState("");
  const [formError, setFormError] = useState<string>();
  const [paramsError, setParamsError] = useState<string>();
  const [planResult, setPlanResult] = useState<DeploymentPlan>();
  const [planMeta, setPlanMeta] = useState<ApiResponseMeta>();
  const [planError, setPlanError] = useState<unknown>();
  const [planLoading, setPlanLoading] = useState(false);
  const [createLoading, setCreateLoading] = useState(false);
  const [actionLoading, setActionLoading] = useState<"retry" | "cancel" | null>(
    null
  );
  const [actionError, setActionError] = useState<unknown>();

  const policiesQuery = useApiQuery({
    queryFn: (signal) => getPolicies({ signal }),
    deps: [],
  });

  const policyDetailsQuery = useApiQuery({
    enabled: Boolean(selectedPolicyId),
    queryFn: (signal) => getPolicyById(selectedPolicyId ?? "", signal),
    deps: [selectedPolicyId],
  });

  const deploymentsQuery = useApiQuery({
    queryFn: (signal) => getDeployments({ signal }),
    deps: [],
    pollIntervalMs: 10_000,
  });

  const deploymentDetailsQuery = useApiQuery({
    enabled: Boolean(selectedDeploymentId),
    queryFn: (signal) => getDeploymentById(selectedDeploymentId ?? "", signal),
    deps: [selectedDeploymentId],
    pollIntervalMs: 5_000,
  });

  useEffect(() => {
    const items = policiesQuery.data?.items ?? [];

    if (items.length === 0) {
      setSelectedPolicyId(null);
      return;
    }

    setSelectedPolicyId((current) => {
      if (current && items.some((item) => item.id === current)) {
        return current;
      }

      return items[0]?.id ?? null;
    });
  }, [policiesQuery.data?.items]);

  useEffect(() => {
    const items = deploymentsQuery.data?.items ?? [];

    if (items.length === 0 && !createLoading) {
      setSelectedDeploymentId(null);
      return;
    }

    setSelectedDeploymentId((current) => {
      if (current && items.some((item) => item.id === current)) {
        return current;
      }

      return current ?? items[0]?.id ?? null;
    });
  }, [createLoading, deploymentsQuery.data?.items]);

  useEffect(() => {
    setFormError(undefined);
    setParamsError(undefined);
    setPlanError(undefined);
    setPlanResult(undefined);
    setPlanMeta(undefined);
  }, [selectedPolicyId, agentIdsText, paramsText]);

  const selectedDeployment = deploymentDetailsQuery.data?.summary;
  const activePoliciesCount =
    policiesQuery.data?.items.filter((item) => item.isActive === true).length ??
    0;
  const inflightDeploymentsCount =
    deploymentsQuery.data?.items.filter((item) =>
      canCancelDeployment(item.status)
    ).length ?? 0;
  const retryableDeploymentsCount =
    deploymentsQuery.data?.items.filter((item) =>
      canRetryDeployment(item.status)
    ).length ?? 0;

  const handlePlan = async () => {
    setFormError(undefined);
    setParamsError(undefined);
    setPlanError(undefined);

    const parsedPayload = parseDraftPayload({
      policyId: selectedPolicyId ?? "",
      agentIdsText,
      paramsText,
    });

    if (parsedPayload.error) {
      setFormError(parsedPayload.error);
      if (parsedPayload.field === "params") {
        setParamsError(parsedPayload.error);
      }
      return;
    }

    if (!parsedPayload.payload) {
      return;
    }

    setPlanLoading(true);
    try {
      const response = await createDeploymentPlan(parsedPayload.payload);
      setPlanResult(response.data);
      setPlanMeta(response.meta);
      showToast({
        title: "Plan preview loaded",
        description:
          "The deployment plan preview was returned by the public Edge API.",
      });
    } catch (caughtError) {
      setPlanError(caughtError);
    } finally {
      setPlanLoading(false);
    }
  };

  const handleCreate = async () => {
    setFormError(undefined);
    setParamsError(undefined);

    const parsedPayload = parseDraftPayload({
      policyId: selectedPolicyId ?? "",
      agentIdsText,
      paramsText,
    });

    if (parsedPayload.error) {
      setFormError(parsedPayload.error);
      if (parsedPayload.field === "params") {
        setParamsError(parsedPayload.error);
      }
      return;
    }

    if (!parsedPayload.payload) {
      return;
    }

    setCreateLoading(true);
    try {
      const response = await createDeployment(parsedPayload.payload);
      setSelectedDeploymentId(response.data.id);
      showToast({
        title: "Deployment created",
        description: `Job ${response.data.id} was accepted by the public Edge API.`,
        variant: "success",
      });
      await deploymentsQuery.refetch({ silent: true });
    } catch (caughtError) {
      setFormError(
        caughtError instanceof Error
          ? caughtError.message
          : "Failed to create deployment."
      );
    } finally {
      setCreateLoading(false);
    }
  };

  const handleDeploymentAction = async (action: "retry" | "cancel") => {
    if (!selectedDeployment) {
      return;
    }

    setActionError(undefined);
    setActionLoading(action);

    try {
      if (action === "retry") {
        await retryDeployment(selectedDeployment.id);
        showToast({
          title: "Retry requested",
          description: `Deployment ${selectedDeployment.id} was sent to the retry endpoint.`,
          variant: "success",
        });
      } else {
        await cancelDeployment(selectedDeployment.id);
        showToast({
          title: "Cancellation requested",
          description: `Deployment ${selectedDeployment.id} was sent to the cancel endpoint.`,
          variant: "success",
        });
      }

      await Promise.all([
        deploymentsQuery.refetch({ silent: true }),
        deploymentDetailsQuery.refetch({ silent: true }),
      ]);
    } catch (caughtError) {
      setActionError(caughtError);
    } finally {
      setActionLoading(null);
    }
  };

  const content = (
    <div className="space-y-6">
      <NoticeBanner
        title="Public Edge API scope"
        description="This workspace uses only confirmed HTTP endpoints for policies and deployments. Agent registry, diagnostics, and bootstrap token issuance remain disabled until the missing public Edge bridges are exposed."
      />

      <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          label="Policies"
          value={
            policiesQuery.data
              ? formatNumber(policiesQuery.data.items.length)
              : policiesQuery.error
                ? "n/a"
                : "..."
          }
          hint="Public `GET /api/v1/policies`"
        />
        <MetricCard
          label="Active policies"
          value={policiesQuery.data ? formatNumber(activePoliciesCount) : "..."}
          hint="Policies marked active by Edge"
        />
        <MetricCard
          label="Jobs in flight"
          value={
            deploymentsQuery.data
              ? formatNumber(inflightDeploymentsCount)
              : deploymentsQuery.error
                ? "n/a"
                : "..."
          }
          hint="Accepted, queued, or running"
        />
        <MetricCard
          label="Retryable jobs"
          value={
            deploymentsQuery.data
              ? formatNumber(retryableDeploymentsCount)
              : deploymentsQuery.error
                ? "n/a"
                : "..."
          }
          hint="Failed, partial success, or cancelled"
        />
      </section>

      <section className="grid gap-4 xl:grid-cols-[minmax(0,1.2fr)_minmax(0,0.8fr)]">
        <SectionCard
          title="Agent Registry"
          description="The current public Edge API does not expose `/api/v1/agents` or diagnostics endpoints to WEB, so this table stays honest instead of mirroring internal gRPC or NATS state."
          action={
            <Button
              size="sm"
              className="h-10 px-4"
              onClick={() => setCreateAgentDialogOpen(true)}
            >
              Create Agent
            </Button>
          }
        >
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Host</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Policy</TableHead>
                <TableHead>Last seen</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TableRow>
                <TableCell colSpan={4}>
                  <EmptyState
                    variant="flush"
                    title="Agent registry bridge is not available yet"
                    description="Use the policy and deployment controls below today. Real agent list and diagnostics can be connected once Edge exposes public HTTP endpoints for them."
                  />
                </TableCell>
              </TableRow>
            </TableBody>
          </Table>
        </SectionCard>

        <SectionCard
          title="Enrollment Bridge"
          description="Frontend structure is ready for future enrollment work, but bootstrap issuance remains disabled until Edge exposes the missing bridge."
        >
          <div className="space-y-4">
            <DetailGrid
              items={[
                {
                  label: "Registry API",
                  value: <StatusBadge value="unavailable" />,
                },
                {
                  label: "Diagnostics API",
                  value: <StatusBadge value="unavailable" />,
                },
                {
                  label: "Missing bridge",
                  value: "agents.bootstrap-token.issue",
                },
              ]}
            />

            <div className="flex flex-wrap gap-2">
              <Button
                variant="outline"
                size="sm"
                className="h-10 px-4"
                disabled
              >
                Issue Bootstrap Token
              </Button>
              <Link
                href={withLocalePath(locale, "/infrastructure?tab=resources")}
              >
                <Button variant="outline" size="sm" className="h-10 px-4">
                  Open Inventory
                </Button>
              </Link>
            </div>
          </div>
        </SectionCard>
      </section>

      <section className="grid gap-4 xl:grid-cols-[minmax(0,1.15fr)_minmax(0,0.85fr)]">
        <SectionCard
          title="Policies"
          description="Confirmed public sources: `GET /api/v1/policies` and `GET /api/v1/policies/{id}`."
        >
          {policiesQuery.isLoading && !policiesQuery.data ? (
            <LoadingState label="Loading policies..." />
          ) : policiesQuery.error && !policiesQuery.data ? (
            <ErrorState
              error={policiesQuery.error}
              retry={() => void policiesQuery.refetch()}
            />
          ) : (
            <div className="space-y-4">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Name</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Revision</TableHead>
                    <TableHead>Updated</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {(policiesQuery.data?.items.length ?? 0) === 0 ? (
                    <TableRow>
                      <TableCell colSpan={4}>
                        <EmptyState
                          variant="flush"
                          title="No policies were returned"
                          description="Create policies through WEB or the public API before building deployments from this workspace."
                        />
                      </TableCell>
                    </TableRow>
                  ) : (
                    policiesQuery.data?.items.map((policy) => (
                      <TableRow
                        key={policy.id}
                        className={
                          policy.id === selectedPolicyId
                            ? "bg-[color:rgba(56,189,248,0.08)]"
                            : undefined
                        }
                        onClick={() => setSelectedPolicyId(policy.id)}
                      >
                        <TableCell className="font-medium text-[color:var(--foreground)]">
                          {policy.name}
                        </TableCell>
                        <TableCell>
                          <StatusBadge
                            value={
                              policy.isActive === false ? "inactive" : "active"
                            }
                          />
                        </TableCell>
                        <TableCell>
                          {formatMaybeValue(policy.revision)}
                        </TableCell>
                        <TableCell>
                          {formatDateTime(policy.updatedAt)}
                        </TableCell>
                      </TableRow>
                    ))
                  )}
                </TableBody>
              </Table>
              <RequestMetaLine meta={policiesQuery.meta} />
            </div>
          )}
        </SectionCard>

        <SectionCard
          title="Deployment Builder"
          description="Preview and create deployments using only confirmed HTTP fields: `policy_id`, optional `agent_ids`, and optional `params`."
          action={
            <div className="flex flex-wrap gap-2">
              <Button
                variant="outline"
                size="sm"
                className="h-10 px-4"
                loading={planLoading}
                disabled={!selectedPolicyId || createLoading}
                onClick={() => void handlePlan()}
              >
                Preview Plan
              </Button>
              <Button
                size="sm"
                className="h-10 px-4"
                loading={createLoading}
                disabled={!selectedPolicyId || planLoading}
                onClick={() => void handleCreate()}
              >
                Create Deployment
              </Button>
            </div>
          }
        >
          <div className="space-y-5">
            {selectedPolicyId ? (
              policyDetailsQuery.isLoading && !policyDetailsQuery.data ? (
                <LoadingState compact label="Loading selected policy..." />
              ) : policyDetailsQuery.error && !policyDetailsQuery.data ? (
                <ErrorState
                  title="Policy details request failed"
                  error={policyDetailsQuery.error}
                  retry={() => void policyDetailsQuery.refetch()}
                />
              ) : policyDetailsQuery.data ? (
                <div className="space-y-4">
                  <DetailGrid
                    items={[
                      {
                        label: "Policy",
                        value: policyDetailsQuery.data.name,
                      },
                      {
                        label: "Policy ID",
                        value: policyDetailsQuery.data.id,
                      },
                      {
                        label: "Status",
                        value: (
                          <StatusBadge
                            value={
                              policyDetailsQuery.data.isActive === false
                                ? "inactive"
                                : "active"
                            }
                          />
                        ),
                      },
                      {
                        label: "Revision",
                        value: formatMaybeValue(
                          policyDetailsQuery.data.revision
                        ),
                      },
                      {
                        label: "Updated",
                        value: formatDateTime(
                          policyDetailsQuery.data.updatedAt
                        ),
                      },
                      {
                        label: "Description",
                        value: formatMaybeValue(
                          policyDetailsQuery.data.description
                        ),
                      },
                    ]}
                  />
                  <JsonPreview
                    value={policyDetailsQuery.data.body}
                    emptyLabel="The selected policy response does not include a materialized JSON body."
                  />
                  <RequestMetaLine meta={policyDetailsQuery.meta} />
                </div>
              ) : null
            ) : (
              <EmptyState
                variant="flush"
                title="No policy selected"
                description="Pick a policy from the table to preview or create deployments."
              />
            )}

            <TextAreaField
              id="agent_ids"
              label="Explicit agent IDs (optional)"
              helperText="Paste agent IDs only if you already know them. There is no public agent registry endpoint to populate this field automatically yet."
              value={agentIdsText}
              onChange={(event) => setAgentIdsText(event.target.value)}
              placeholder={"agent-01\nagent-02"}
            />

            <TextAreaField
              id="params_json"
              label="Params JSON (optional)"
              helperText="Provide a flat JSON object. Values are serialized to strings to match the currently confirmed public HTTP shape."
              value={paramsText}
              onChange={(event) => setParamsText(event.target.value)}
              placeholder='{"rollout":"canary","window":"15m"}'
              error={paramsError}
            />

            {formError ? (
              <p className="text-sm text-[color:var(--status-danger-fg)]">
                {formError}
              </p>
            ) : null}

            <div className="rounded-xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
              <div className="space-y-4">
                <div>
                  <p className="text-sm font-semibold text-[color:var(--foreground)]">
                    Plan preview
                  </p>
                  <p className="mt-1 text-sm leading-6 text-[color:var(--muted-foreground)]">
                    The preview stays empty until the public `POST
                    /api/v1/deployments/plan` endpoint is called.
                  </p>
                </div>

                {planLoading ? (
                  <LoadingState compact label="Loading plan preview..." />
                ) : planError ? (
                  <ErrorState error={planError as never} />
                ) : planResult ? (
                  <div className="space-y-4">
                    <DetailGrid
                      items={[
                        {
                          label: "Policy ID",
                          value: formatMaybeValue(planResult.policyId),
                        },
                        {
                          label: "Revision",
                          value: formatMaybeValue(planResult.policyRevision),
                        },
                        {
                          label: "Executor",
                          value: formatMaybeValue(planResult.executorKind),
                        },
                        {
                          label: "Targets",
                          value: formatNumber(planResult.targets.length),
                        },
                        {
                          label: "Bootstrap previews",
                          value: formatNumber(
                            planResult.bootstrapPreviews.length
                          ),
                        },
                        {
                          label: "Warnings",
                          value: formatNumber(planResult.warnings.length),
                        },
                      ]}
                    />

                    {planResult.actionSummary ? (
                      <p className="text-sm leading-6 text-[color:var(--foreground)]">
                        {planResult.actionSummary}
                      </p>
                    ) : null}

                    {planResult.warnings.length > 0 ? (
                      <div className="flex flex-wrap gap-2">
                        {planResult.warnings.map((warning) => (
                          <Badge key={warning} variant="warning">
                            {warning}
                          </Badge>
                        ))}
                      </div>
                    ) : null}

                    {planResult.targets.length > 0 ? (
                      <div className="space-y-2">
                        <p className="text-sm font-semibold text-[color:var(--foreground)]">
                          Preview targets
                        </p>
                        {planResult.targets.slice(0, 6).map((target, index) => (
                          <div
                            key={`${target.hostId ?? target.hostname ?? "target"}-${index}`}
                            className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface-elevated)] p-3 text-sm"
                          >
                            <p className="font-medium text-[color:var(--foreground)]">
                              {target.hostname ??
                                target.hostId ??
                                "Unnamed target"}
                            </p>
                            <p className="mt-1 text-[color:var(--muted-foreground)]">
                              {target.ip ? `${target.ip}` : "IP unavailable"}
                              {target.remoteUser
                                ? ` via ${target.remoteUser}`
                                : ""}
                            </p>
                          </div>
                        ))}
                      </div>
                    ) : null}

                    <RequestMetaLine meta={planMeta} />
                  </div>
                ) : (
                  <EmptyState
                    variant="flush"
                    title="No plan preview yet"
                    description="Select a policy and run Preview Plan to inspect the deployment result available from the public Edge API."
                  />
                )}
              </div>
            </div>
          </div>
        </SectionCard>
      </section>

      <section className="grid gap-4 xl:grid-cols-[minmax(0,1.15fr)_minmax(0,0.85fr)]">
        <SectionCard
          title="Deployment Jobs"
          description="Confirmed public sources: `GET /api/v1/deployments`, `POST /api/v1/deployments/{id}/retry`, and `POST /api/v1/deployments/{id}/cancel`."
        >
          {deploymentsQuery.isLoading && !deploymentsQuery.data ? (
            <LoadingState label="Loading deployment jobs..." />
          ) : deploymentsQuery.error && !deploymentsQuery.data ? (
            <ErrorState
              error={deploymentsQuery.error}
              retry={() => void deploymentsQuery.refetch()}
            />
          ) : (
            <div className="space-y-4">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Job</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Policy</TableHead>
                    <TableHead>Targets</TableHead>
                    <TableHead>Updated</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {(deploymentsQuery.data?.items.length ?? 0) === 0 ? (
                    <TableRow>
                      <TableCell colSpan={5}>
                        <EmptyState
                          variant="flush"
                          title="No deployment jobs were returned"
                          description="Create a deployment from the builder above or through WEB to populate this list."
                        />
                      </TableCell>
                    </TableRow>
                  ) : (
                    deploymentsQuery.data?.items.map((job) => (
                      <TableRow
                        key={job.id}
                        className={
                          job.id === selectedDeploymentId
                            ? "bg-[color:rgba(56,189,248,0.08)]"
                            : undefined
                        }
                        onClick={() => setSelectedDeploymentId(job.id)}
                      >
                        <TableCell className="font-medium text-[color:var(--foreground)]">
                          <div>{formatMaybeValue(job.jobType)}</div>
                          <div className="mt-1 text-xs text-[color:var(--muted-foreground)]">
                            {job.id}
                          </div>
                        </TableCell>
                        <TableCell>
                          <StatusBadge value={job.status} />
                        </TableCell>
                        <TableCell>{formatMaybeValue(job.policyId)}</TableCell>
                        <TableCell>{formatNumber(job.totalTargets)}</TableCell>
                        <TableCell>{formatDateTime(job.updatedAt)}</TableCell>
                      </TableRow>
                    ))
                  )}
                </TableBody>
              </Table>
              <RequestMetaLine meta={deploymentsQuery.meta} />
            </div>
          )}
        </SectionCard>

        <SectionCard
          title="Job Details"
          description="Confirmed public source: `GET /api/v1/deployments/{id}`."
          action={
            selectedDeployment ? (
              <div className="flex flex-wrap gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  className="h-10 px-4"
                  disabled={
                    !canRetryDeployment(selectedDeployment.status) ||
                    actionLoading !== null
                  }
                  loading={actionLoading === "retry"}
                  onClick={() => void handleDeploymentAction("retry")}
                >
                  Retry
                </Button>
                <Button
                  variant="danger"
                  size="sm"
                  className="h-10 px-4"
                  disabled={
                    !canCancelDeployment(selectedDeployment.status) ||
                    actionLoading !== null
                  }
                  loading={actionLoading === "cancel"}
                  onClick={() => void handleDeploymentAction("cancel")}
                >
                  Cancel
                </Button>
                <Link
                  href={withLocalePath(
                    locale,
                    `/deployments/${selectedDeployment.id}`
                  )}
                >
                  <Button variant="outline" size="sm" className="h-10 px-4">
                    Open Full Details
                  </Button>
                </Link>
              </div>
            ) : null
          }
        >
          {!selectedDeploymentId ? (
            <EmptyState
              variant="flush"
              title="No job selected"
              description="Choose a deployment job from the table to inspect its public details."
            />
          ) : deploymentDetailsQuery.isLoading &&
            !deploymentDetailsQuery.data ? (
            <LoadingState label="Loading deployment details..." />
          ) : deploymentDetailsQuery.error && !deploymentDetailsQuery.data ? (
            <ErrorState
              error={deploymentDetailsQuery.error}
              retry={() => void deploymentDetailsQuery.refetch()}
            />
          ) : deploymentDetailsQuery.data ? (
            <div className="space-y-5">
              {actionError ? <ErrorState error={actionError as never} /> : null}

              <DetailGrid
                items={[
                  {
                    label: "Deployment ID",
                    value: deploymentDetailsQuery.data.summary.id,
                  },
                  {
                    label: "Status",
                    value: (
                      <StatusBadge
                        value={deploymentDetailsQuery.data.summary.status}
                      />
                    ),
                  },
                  {
                    label: "Policy ID",
                    value: formatMaybeValue(
                      deploymentDetailsQuery.data.summary.policyId
                    ),
                  },
                  {
                    label: "Job type",
                    value: formatMaybeValue(
                      deploymentDetailsQuery.data.summary.jobType
                    ),
                  },
                  {
                    label: "Current phase",
                    value: formatMaybeValue(
                      deploymentDetailsQuery.data.summary.currentPhase
                    ),
                  },
                  {
                    label: "Updated",
                    value: formatDateTime(
                      deploymentDetailsQuery.data.summary.updatedAt
                    ),
                  },
                  {
                    label: "Attempts",
                    value: formatNumber(
                      deploymentDetailsQuery.data.summary.attemptCount
                    ),
                  },
                  {
                    label: "Targets",
                    value: formatNumber(
                      deploymentDetailsQuery.data.summary.totalTargets
                    ),
                  },
                  {
                    label: "Params",
                    value: formatParamsSummary(
                      deploymentDetailsQuery.data.summary.params
                    ),
                  },
                ]}
              />

              <div className="space-y-2">
                <p className="text-sm font-semibold text-[color:var(--foreground)]">
                  Attempts
                </p>
                {deploymentDetailsQuery.data.attempts.length === 0 ? (
                  <EmptyState
                    variant="flush"
                    title="No attempts were returned"
                    description="Attempts will appear after the deployment enters execution."
                  />
                ) : (
                  deploymentDetailsQuery.data.attempts.map((attempt) => (
                    <div
                      key={attempt.id}
                      className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3 text-sm"
                    >
                      <div className="flex flex-wrap items-center justify-between gap-3">
                        <span className="font-medium text-[color:var(--foreground)]">
                          Attempt {formatMaybeValue(attempt.attemptNo)}
                        </span>
                        <StatusBadge value={attempt.status} />
                      </div>
                      <p className="mt-2 text-[color:var(--muted-foreground)]">
                        Triggered by {formatMaybeValue(attempt.triggeredBy)} at{" "}
                        {formatDateTime(attempt.createdAt)}
                      </p>
                    </div>
                  ))
                )}
              </div>

              <div className="space-y-2">
                <p className="text-sm font-semibold text-[color:var(--foreground)]">
                  Targets
                </p>
                {deploymentDetailsQuery.data.targets.length === 0 ? (
                  <EmptyState
                    variant="flush"
                    title="No targets were returned"
                    description="Targets will appear once the deployment has been planned or started."
                  />
                ) : (
                  deploymentDetailsQuery.data.targets
                    .slice(0, 8)
                    .map((target) => (
                      <div
                        key={target.id}
                        className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3 text-sm"
                      >
                        <div className="flex flex-wrap items-center justify-between gap-3">
                          <span className="font-medium text-[color:var(--foreground)]">
                            {target.hostname ?? target.hostId ?? target.id}
                          </span>
                          <StatusBadge value={target.status} />
                        </div>
                        {target.errorMessage ? (
                          <p className="mt-2 text-[color:var(--status-danger-fg)]">
                            {target.errorMessage}
                          </p>
                        ) : null}
                      </div>
                    ))
                )}
              </div>

              <div className="space-y-2">
                <p className="text-sm font-semibold text-[color:var(--foreground)]">
                  Latest step payload
                </p>
                {deploymentDetailsQuery.data.steps[0] ? (
                  <div className="space-y-3">
                    <div className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3 text-sm">
                      <div className="flex flex-wrap items-center justify-between gap-3">
                        <span className="font-medium text-[color:var(--foreground)]">
                          {formatMaybeValue(
                            deploymentDetailsQuery.data.steps[0].name
                          )}
                        </span>
                        <StatusBadge
                          value={deploymentDetailsQuery.data.steps[0].status}
                        />
                      </div>
                      <p className="mt-2 text-[color:var(--muted-foreground)]">
                        Updated{" "}
                        {formatDateTime(
                          deploymentDetailsQuery.data.steps[0].updatedAt
                        )}
                      </p>
                    </div>
                    <JsonPreview
                      value={deploymentDetailsQuery.data.steps[0].payload}
                    />
                  </div>
                ) : (
                  <EmptyState
                    variant="flush"
                    title="No steps were returned"
                    description="Step payloads will appear once the job starts executing."
                  />
                )}
              </div>

              <RequestMetaLine meta={deploymentDetailsQuery.meta} />
            </div>
          ) : null}
        </SectionCard>
      </section>
    </div>
  );

  return (
    <div className={embedded ? "space-y-4" : "space-y-6"}>
      {!embedded ? (
        <PageHeader
          title="Agents"
          description="Operator-facing agent workspace wired only to confirmed public Edge API endpoints. Policy selection and deployments work today; registry, diagnostics, and enrollment still await Edge bridges."
          breadcrumbs={[
            { label: dictionary.common.dashboard, href: "#" },
            { label: "Agents" },
          ]}
        />
      ) : null}

      {embedded ? <div>{content}</div> : <Card className="p-6">{content}</Card>}

      {createAgentDialogOpen ? (
        <AgentEnrollmentDialog
          open={createAgentDialogOpen}
          onClose={() => setCreateAgentDialogOpen(false)}
          policies={policiesQuery.data?.items ?? []}
          initialPolicyId={selectedPolicyId}
        />
      ) : null}
    </div>
  );
}

function parseDraftPayload(input: {
  policyId: string;
  agentIdsText: string;
  paramsText: string;
}):
  | { payload: DeploymentMutationPayload; error?: undefined; field?: undefined }
  | { payload?: undefined; error: string; field?: "policy" | "params" } {
  const policyId = input.policyId.trim();

  if (!policyId) {
    return {
      error: "Select a policy before previewing or creating a deployment.",
      field: "policy",
    };
  }

  const agentIds = input.agentIdsText
    .split(/\r?\n|,/)
    .map((item) => item.trim())
    .filter(Boolean);

  let params: Record<string, string> | undefined;

  if (input.paramsText.trim()) {
    try {
      const parsed = JSON.parse(input.paramsText);

      if (
        typeof parsed !== "object" ||
        parsed === null ||
        Array.isArray(parsed)
      ) {
        return {
          error: "Params must be a JSON object.",
          field: "params",
        };
      }

      params = Object.fromEntries(
        Object.entries(parsed).map(([key, value]) => [key, String(value)])
      );
    } catch {
      return {
        error: "Params must be valid JSON.",
        field: "params",
      };
    }
  }

  return {
    payload: {
      policyId,
      ...(agentIds.length > 0 ? { agentIds } : {}),
      ...(params && Object.keys(params).length > 0 ? { params } : {}),
    },
  };
}
