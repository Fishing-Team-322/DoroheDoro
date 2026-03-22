"use client";

import { createApiClient, type ApiResult } from "@/src/shared/lib/api";
import {
  clearCsrfToken,
  fetchCsrfToken,
  getCsrfToken,
} from "@/src/shared/lib/auth/csrf";
import { emitUnauthorized } from "@/src/shared/lib/auth/events";

const runtimeApiClient = createApiClient({
  baseUrl: process.env.NEXT_PUBLIC_API_BASE_URL ?? "/api/edge",
  credentials: "include",
  getCsrfToken,
  onUnauthorized: () => {
    clearCsrfToken();
    emitUnauthorized();
  },
});

type QueryValue = string | number | boolean | null | undefined;

export type RuntimeRequestResult<T> = ApiResult<T>;

export type DashboardMetricItem = {
  key: string;
  label: string;
  value: string;
  change: number;
  trend: string;
  description: string;
};

export type DashboardActivityItem = {
  kind: string;
  title: string;
  description: string;
  timestamp: string;
};

export type HistogramBucketItem = {
  bucket: string;
  count: number;
};

export type CountBucketItem = {
  key: string;
  count: number;
};

export type DashboardOverviewResponse = {
  metrics: DashboardMetricItem[];
  active_hosts: number;
  open_alerts: number;
  deployment_jobs: number;
  ingested_events: number;
  recent_activity: DashboardActivityItem[];
  log_histogram: HistogramBucketItem[];
  top_services: CountBucketItem[];
  top_hosts: CountBucketItem[];
  request_id: string;
};

export type HostItem = {
  host_id: string;
  hostname: string;
  ip: string;
  ssh_port: number;
  remote_user: string;
  labels: Record<string, string>;
  created_at: string;
  updated_at: string;
};

export type HostGroupMemberItem = {
  host_group_member_id: string;
  host_group_id: string;
  host_id: string;
  hostname: string;
};

export type HostGroupItem = {
  host_group_id: string;
  name: string;
  description: string;
  created_at: string;
  updated_at: string;
  members: HostGroupMemberItem[];
};

export type CredentialItem = {
  credentials_profile_id: string;
  name: string;
  kind: string;
  description: string;
  vault_ref: string;
  created_at: string;
  updated_at: string;
};

export type ClusterItem = {
  cluster_id: string;
  name: string;
  slug: string;
  description: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
  created_by?: string;
  updated_by?: string;
  metadata_json?: Record<string, unknown>;
  host_count: number;
  agent_count: number;
};

export type TelegramIntegrationConfigItem = {
  bot_name?: string;
  parse_mode?: string;
  default_chat_id?: string;
  message_template_version?: string;
  delivery_enabled?: boolean;
  has_secret_ref?: boolean;
  masked_secret_ref?: string;
};

export type IntegrationBindingItem = {
  integration_binding_id: string;
  integration_id: string;
  scope_type: string;
  scope_id: string;
  event_types_json: string[];
  severity_threshold: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
};

export type IntegrationItem = {
  integration_id: string;
  name: string;
  kind: string;
  description: string;
  config_json: TelegramIntegrationConfigItem | Record<string, unknown>;
  is_active: boolean;
  created_at: string;
  updated_at: string;
  created_by?: string;
  updated_by?: string;
  bindings: IntegrationBindingItem[];
};

export type IntegrationUpsertPayload = {
  name: string;
  kind: string;
  description?: string;
  config_json: Record<string, unknown>;
  is_active: boolean;
  reason?: string;
};

export type IntegrationBindingPayload = {
  scope_type: string;
  scope_id?: string;
  event_types_json: string[];
  severity_threshold: string;
  is_active: boolean;
  reason?: string;
};

export type TelegramIntegrationHealthcheckResponse = {
  status: string;
  request_id: string;
  correlation_id: string;
  integration_id: string;
  subject?: string;
};

export type PolicyItem = {
  policy_id: string;
  name: string;
  description: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
  latest_revision?: string;
  latest_body_json?: Record<string, unknown>;
};

export type PolicyRevisionItem = {
  policy_revision_id: string;
  revision: string;
  body_json: Record<string, unknown>;
  created_at: string;
};

export type PolicySummary = {
  id: string;
  name: string;
  description?: string;
  isActive?: boolean;
  revision?: string;
  createdAt?: string;
  updatedAt?: string;
  body?: Record<string, unknown>;
  raw?: unknown;
};

export type PoliciesList = {
  items: PolicySummary[];
  nextCursor?: string;
  raw?: unknown;
};

export type DeploymentJobItem = {
  job_id: string;
  job_type: string;
  status: string;
  requested_by: string;
  policy_id: string;
  policy_revision_id: string;
  credential_profile_id: string;
  executor_kind: string;
  current_phase: string;
  total_targets: number;
  pending_targets: number;
  running_targets: number;
  succeeded_targets: number;
  failed_targets: number;
  cancelled_targets: number;
  attempt_count: number;
  created_at: string;
  started_at?: string;
  finished_at?: string;
  updated_at: string;
};

export type DeploymentAttemptItem = {
  deployment_attempt_id: string;
  attempt_no: number;
  status: string;
  triggered_by: string;
  reason: string;
  created_at: string;
  started_at?: string;
  finished_at?: string;
};

export type DeploymentTargetArtifact = {
  version: string;
  package_type: string;
  source_uri: string;
  sha256: string;
  artifact_name?: string;
  arch?: string;
  distro_family?: string;
};

export type DeploymentTargetItem = {
  deployment_target_id: string;
  deployment_attempt_id: string;
  host_id: string;
  hostname_snapshot: string;
  status: string;
  error_message?: string;
  created_at: string;
  started_at?: string;
  finished_at?: string;
  updated_at: string;
  artifact?: DeploymentTargetArtifact;
};

export type DeploymentStepItem = {
  deployment_step_id: string;
  deployment_attempt_id: string;
  deployment_target_id?: string;
  step_name: string;
  status: string;
  message: string;
  payload_json: Record<string, unknown>;
  created_at: string;
  updated_at: string;
};

export type DeploymentDetailResponse = {
  item: DeploymentJobItem;
  attempts: DeploymentAttemptItem[];
  targets: DeploymentTargetItem[];
  steps: DeploymentStepItem[];
  request_id: string;
};

export type DeploymentMutationPayload = {
  policyId: string;
  agentIds?: string[];
  params?: Record<string, string>;
};

export type DeploymentSummary = {
  id: string;
  jobType?: string;
  status: string;
  policyId?: string;
  policyRevisionId?: string;
  createdAt?: string;
  startedAt?: string;
  finishedAt?: string;
  updatedAt?: string;
  currentPhase?: string;
  requestedBy?: string;
  credentialProfileId?: string;
  executorKind?: string;
  totalTargets?: number;
  pendingTargets?: number;
  runningTargets?: number;
  succeededTargets?: number;
  failedTargets?: number;
  cancelledTargets?: number;
  attemptCount?: number;
  params?: Record<string, unknown>;
  raw?: unknown;
};

export type DeploymentsList = {
  items: DeploymentSummary[];
  nextCursor?: string;
  total?: number;
  raw?: unknown;
};

export type DeploymentAttempt = {
  id: string;
  attemptNo?: number;
  status?: string;
  triggeredBy?: string;
  reason?: string;
  createdAt?: string;
  startedAt?: string;
  finishedAt?: string;
  raw?: unknown;
};

export type DeploymentPlanTarget = {
  hostId?: string;
  hostname?: string;
  ip?: string;
  sshPort?: number;
  remoteUser?: string;
};

export type BootstrapPreview = {
  hostId?: string;
  hostname?: string;
  bootstrapYaml?: string;
};

export type DeploymentPlan = {
  jobType?: string;
  policyId?: string;
  policyRevisionId?: string;
  policyRevision?: string;
  credentialProfileId?: string;
  credentialSummary?: string;
  executorKind?: string;
  actionSummary?: string;
  targets: DeploymentPlanTarget[];
  bootstrapPreviews: BootstrapPreview[];
  warnings: string[];
  raw?: unknown;
};

export type DeploymentTarget = {
  id: string;
  attemptId?: string;
  hostId?: string;
  hostname?: string;
  status?: string;
  errorMessage?: string;
  createdAt?: string;
  startedAt?: string;
  finishedAt?: string;
  updatedAt?: string;
  raw?: unknown;
};

export type DeploymentStep = {
  id: string;
  attemptId?: string;
  targetId?: string;
  name?: string;
  status?: string;
  message?: string;
  payload?: unknown;
  createdAt?: string;
  updatedAt?: string;
  raw?: unknown;
};

export type DeploymentDetails = {
  summary: DeploymentSummary;
  attempts: DeploymentAttempt[];
  targets: DeploymentTarget[];
  steps: DeploymentStep[];
  raw?: unknown;
};

export type BootstrapTokenIssuePayload = {
  policyId: string;
  policyRevisionId: string;
  requestedBy: string;
  expiresAtUnixMs: number;
};

export type BootstrapTokenItem = {
  tokenId: string;
  bootstrapToken: string;
  policyId: string;
  policyRevisionId: string;
  expiresAtUnixMs: number;
  createdAtUnixMs: number;
  raw?: unknown;
};

export type AgentItem = {
  agent_id: string;
  hostname: string;
  status: string;
  version: string;
  metadata_json: Record<string, unknown>;
  first_seen_at: string;
  last_seen_at: string;
  effective_policy?: {
    policy_id?: string;
    policy_revision_id?: string;
    policy_revision?: string;
    assigned_at?: string;
    policy_name?: string;
    policy_description?: string;
  };
};

export type AgentDiagnosticsItem = {
  agent_id: string;
  payload_json: Record<string, unknown>;
  created_at: string;
};

export type AgentRegistryList = {
  items: AgentItem[];
  raw?: unknown;
};

export type LogEventItem = {
  id: string;
  timestamp: string;
  host: string;
  agent_id: string;
  source_type: string;
  source: string;
  service: string;
  severity: string;
  message: string;
  fingerprint: string;
  labels: Record<string, string>;
  fields_json: Record<string, unknown>;
  raw: string;
};

export type LogSearchResponse = {
  items: LogEventItem[];
  total: number;
  limit: number;
  offset: number;
  took_ms: number;
  request_id: string;
};

export type LogAnomalyItem = {
  alert_instance_id: string;
  alert_rule_id: string;
  status: string;
  severity: string;
  title: string;
  fingerprint: string;
  host: string;
  service: string;
  triggered_at: string;
  payload_json: Record<string, unknown>;
};

export type LogAnomaliesResponse = {
  items: LogAnomalyItem[];
  total: number;
  limit: number;
  offset: number;
  request_id: string;
};

export type AlertRuleItem = {
  alert_rule_id: string;
  name: string;
  description: string;
  status: string;
  severity: string;
  scope_type: string;
  scope_id: string;
  condition_json: Record<string, unknown>;
  created_at: string;
  updated_at: string;
  created_by: string;
  updated_by: string;
};

export type AlertInstanceItem = {
  alert_instance_id: string;
  alert_rule_id: string;
  title: string;
  status: string;
  severity: string;
  triggered_at: string;
  acknowledged_at?: string;
  resolved_at?: string;
  host: string;
  service: string;
  fingerprint: string;
  payload_json: Record<string, unknown>;
};

export type PagingMeta = {
  limit: number;
  offset: number;
  total: number;
};

export type AlertRulesResponse = {
  items: AlertRuleItem[];
  paging: PagingMeta;
  request_id: string;
};

export type AlertInstancesResponse = {
  items: AlertInstanceItem[];
  paging: PagingMeta;
  request_id: string;
};

export type AuditEventItem = {
  audit_event_id: string;
  event_type: string;
  entity_type: string;
  entity_id: string;
  actor_id: string;
  actor_type: string;
  request_id: string;
  reason: string;
  payload_json: Record<string, unknown>;
  created_at: string;
};

export type AuditEventsResponse = {
  items: AuditEventItem[];
  paging: PagingMeta;
  request_id: string;
};

const isRecord = (value: unknown): value is Record<string, unknown> => {
  return typeof value === "object" && value !== null;
};

const asRecord = (value: unknown): Record<string, unknown> => {
  return isRecord(value) ? value : {};
};

const asArray = (value: unknown): unknown[] => {
  return Array.isArray(value) ? value : [];
};

const asString = (value: unknown): string | undefined => {
  return typeof value === "string" && value.trim() ? value : undefined;
};

const asBoolean = (value: unknown): boolean | undefined => {
  if (typeof value === "boolean") {
    return value;
  }

  if (typeof value === "string") {
    if (value === "true") {
      return true;
    }

    if (value === "false") {
      return false;
    }
  }

  return undefined;
};

const asNumber = (value: unknown): number | undefined => {
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }

  if (typeof value === "string" && value.trim()) {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) {
      return parsed;
    }
  }

  return undefined;
};

const asStringArray = (value: unknown): string[] => {
  return asArray(value)
    .map((item) => asString(item))
    .filter((item): item is string => Boolean(item));
};

const parseJsonString = (value: unknown): unknown => {
  if (typeof value !== "string" || !value.trim()) {
    return undefined;
  }

  try {
    return JSON.parse(value);
  } catch {
    return value;
  }
};

const pickObject = (value: unknown): Record<string, unknown> | undefined => {
  if (isRecord(value)) {
    return value;
  }

  const parsed = parseJsonString(value);
  return isRecord(parsed) ? parsed : undefined;
};

const encodePathSegment = (value: string): string => encodeURIComponent(value);

const serializeDeploymentPayload = (
  payload: DeploymentMutationPayload
): Record<string, unknown> => {
  return {
    policy_id: payload.policyId,
    ...(payload.agentIds && payload.agentIds.length > 0
      ? { agent_ids: payload.agentIds }
      : {}),
    ...(payload.params && Object.keys(payload.params).length > 0
      ? { params: payload.params }
      : {}),
  };
};

const normalizePolicy = (value: unknown): PolicySummary => {
  const record = asRecord(value);

  return {
    id: asString(record.id ?? record.policy_id) ?? "unknown-policy",
    name:
      asString(record.name) ?? asString(record.policy_id) ?? "Unnamed policy",
    description: asString(record.description),
    isActive: asBoolean(record.is_active),
    revision: asString(record.revision ?? record.latest_revision),
    createdAt: asString(record.created_at),
    updatedAt: asString(record.updated_at),
    body:
      pickObject(record.latest_body_json) ??
      pickObject(record.body_json) ??
      pickObject(record.policy_body_json),
    raw: value,
  };
};

const normalizeDeploymentSummary = (value: unknown): DeploymentSummary => {
  const record = asRecord(value);
  const summary = pickObject(record.summary_json);
  const payload = pickObject(record.payload_json);

  return {
    id: asString(record.id ?? record.job_id) ?? "unknown-deployment",
    jobType:
      asString(record.job_type) ?? asString(summary?.job_type) ?? undefined,
    status: asString(record.status) ?? asString(summary?.status) ?? "unknown",
    policyId:
      asString(record.policy_id) ?? asString(summary?.policy_id) ?? undefined,
    policyRevisionId:
      asString(record.policy_revision_id) ??
      asString(summary?.policy_revision_id) ??
      undefined,
    createdAt: asString(record.created_at),
    startedAt: asString(record.started_at),
    finishedAt: asString(record.finished_at),
    updatedAt: asString(record.updated_at),
    currentPhase:
      asString(record.current_phase) ??
      asString(summary?.current_phase) ??
      undefined,
    requestedBy:
      asString(record.requested_by) ??
      asString(summary?.requested_by) ??
      undefined,
    credentialProfileId:
      asString(record.credential_profile_id) ??
      asString(summary?.credential_profile_id) ??
      undefined,
    executorKind:
      asString(record.executor_kind) ??
      asString(summary?.executor_kind) ??
      undefined,
    totalTargets:
      asNumber(record.total_targets) ?? asNumber(summary?.total_targets),
    pendingTargets:
      asNumber(record.pending_targets) ?? asNumber(summary?.pending_targets),
    runningTargets:
      asNumber(record.running_targets) ?? asNumber(summary?.running_targets),
    succeededTargets:
      asNumber(record.succeeded_targets) ??
      asNumber(summary?.succeeded_targets),
    failedTargets:
      asNumber(record.failed_targets) ?? asNumber(summary?.failed_targets),
    cancelledTargets:
      asNumber(record.cancelled_targets) ??
      asNumber(summary?.cancelled_targets),
    attemptCount:
      asNumber(record.attempt_count) ?? asNumber(summary?.attempt_count),
    params:
      pickObject(record.params) ?? pickObject(payload?.params) ?? undefined,
    raw: value,
  };
};

const normalizeDeploymentAttempt = (value: unknown): DeploymentAttempt => {
  const record = asRecord(value);

  return {
    id:
      asString(record.deployment_attempt_id ?? record.id) ?? "unknown-attempt",
    attemptNo: asNumber(record.attempt_no),
    status: asString(record.status),
    triggeredBy: asString(record.triggered_by),
    reason: asString(record.reason),
    createdAt: asString(record.created_at),
    startedAt: asString(record.started_at),
    finishedAt: asString(record.finished_at),
    raw: value,
  };
};

const normalizeDeploymentTarget = (value: unknown): DeploymentTarget => {
  const record = asRecord(value);

  return {
    id: asString(record.deployment_target_id ?? record.id) ?? "unknown-target",
    attemptId: asString(record.deployment_attempt_id ?? record.attempt_id),
    hostId: asString(record.host_id),
    hostname:
      asString(record.hostname_snapshot ?? record.hostname ?? record.host) ??
      undefined,
    status: asString(record.status),
    errorMessage: asString(record.error_message),
    createdAt: asString(record.created_at),
    startedAt: asString(record.started_at),
    finishedAt: asString(record.finished_at),
    updatedAt: asString(record.updated_at),
    raw: value,
  };
};

const normalizeDeploymentStep = (value: unknown): DeploymentStep => {
  const record = asRecord(value);

  return {
    id: asString(record.deployment_step_id ?? record.id) ?? "unknown-step",
    attemptId: asString(record.deployment_attempt_id ?? record.attempt_id),
    targetId: asString(record.deployment_target_id ?? record.target_id),
    name: asString(record.step_name ?? record.name),
    status: asString(record.status),
    message: asString(record.message),
    payload: parseJsonString(record.payload_json) ?? pickObject(record.payload),
    createdAt: asString(record.created_at),
    updatedAt: asString(record.updated_at),
    raw: value,
  };
};

const normalizeDeploymentDetails = (value: unknown): DeploymentDetails => {
  const record = asRecord(value);
  const summarySource = record.item ?? record.job ?? value;

  return {
    summary: normalizeDeploymentSummary(summarySource),
    attempts: asArray(record.attempts).map(normalizeDeploymentAttempt),
    targets: asArray(record.targets).map(normalizeDeploymentTarget),
    steps: asArray(record.steps).map(normalizeDeploymentStep),
    raw: value,
  };
};

const normalizePlanTarget = (value: unknown): DeploymentPlanTarget => {
  const record = asRecord(value);

  return {
    hostId: asString(record.host_id),
    hostname: asString(record.hostname ?? record.host),
    ip: asString(record.ip),
    sshPort: asNumber(record.ssh_port),
    remoteUser: asString(record.remote_user),
  };
};

const normalizeBootstrapPreview = (value: unknown): BootstrapPreview => {
  const record = asRecord(value);

  return {
    hostId: asString(record.host_id),
    hostname: asString(record.hostname),
    bootstrapYaml: asString(record.bootstrap_yaml),
  };
};

const normalizeBootstrapToken = (value: unknown): BootstrapTokenItem => {
  const record = asRecord(value);

  return {
    tokenId: asString(record.token_id) ?? "unknown-token",
    bootstrapToken: asString(record.bootstrap_token) ?? "",
    policyId: asString(record.policy_id) ?? "",
    policyRevisionId: asString(record.policy_revision_id) ?? "",
    expiresAtUnixMs: asNumber(record.expires_at_unix_ms) ?? 0,
    createdAtUnixMs: asNumber(record.created_at_unix_ms) ?? 0,
    raw: value,
  };
};

const normalizeAgentItem = (value: unknown): AgentItem => {
  const record = asRecord(value);
  const effectivePolicy = pickObject(record.effective_policy);

  return {
    agent_id: asString(record.agent_id) ?? "unknown-agent",
    hostname: asString(record.hostname) ?? "unknown-host",
    status: asString(record.status) ?? "unknown",
    version: asString(record.version) ?? "",
    metadata_json: pickObject(record.metadata_json) ?? {},
    first_seen_at: asString(record.first_seen_at) ?? "",
    last_seen_at: asString(record.last_seen_at) ?? "",
    effective_policy: effectivePolicy
      ? {
          policy_id: asString(effectivePolicy.policy_id),
          policy_revision_id: asString(effectivePolicy.policy_revision_id),
          policy_revision: asString(effectivePolicy.policy_revision),
          assigned_at: asString(effectivePolicy.assigned_at),
          policy_name: asString(effectivePolicy.policy_name),
          policy_description: asString(effectivePolicy.policy_description),
        }
      : undefined,
  };
};

const normalizeClusterItem = (value: unknown): ClusterItem => {
  const record = asRecord(value);

  return {
    cluster_id: asString(record.cluster_id) ?? "unknown-cluster",
    name: asString(record.name) ?? "Unnamed cluster",
    slug: asString(record.slug) ?? "",
    description: asString(record.description) ?? "",
    is_active: asBoolean(record.is_active) ?? true,
    created_at: asString(record.created_at) ?? "",
    updated_at: asString(record.updated_at) ?? "",
    created_by: asString(record.created_by),
    updated_by: asString(record.updated_by),
    metadata_json: pickObject(record.metadata_json),
    host_count: asNumber(record.host_count) ?? 0,
    agent_count: asNumber(record.agent_count) ?? 0,
  };
};

const normalizeIntegrationBindingItem = (
  value: unknown
): IntegrationBindingItem => {
  const record = asRecord(value);

  return {
    integration_binding_id:
      asString(record.integration_binding_id) ?? "unknown-binding",
    integration_id: asString(record.integration_id) ?? "",
    scope_type: asString(record.scope_type) ?? "global",
    scope_id: asString(record.scope_id) ?? "",
    event_types_json: asStringArray(record.event_types_json),
    severity_threshold: asString(record.severity_threshold) ?? "info",
    is_active: asBoolean(record.is_active) ?? true,
    created_at: asString(record.created_at) ?? "",
    updated_at: asString(record.updated_at) ?? "",
  };
};

const normalizeIntegrationItem = (value: unknown): IntegrationItem => {
  const record = asRecord(value);

  return {
    integration_id: asString(record.integration_id) ?? "unknown-integration",
    name: asString(record.name) ?? "Unnamed integration",
    kind: asString(record.kind) ?? "",
    description: asString(record.description) ?? "",
    config_json: pickObject(record.config_json) ?? {},
    is_active: asBoolean(record.is_active) ?? true,
    created_at: asString(record.created_at) ?? "",
    updated_at: asString(record.updated_at) ?? "",
    created_by: asString(record.created_by),
    updated_by: asString(record.updated_by),
    bindings: asArray(record.bindings).map(normalizeIntegrationBindingItem),
  };
};

const normalizeDeploymentPlan = (value: unknown): DeploymentPlan => {
  const record = asRecord(value);

  return {
    jobType: asString(record.job_type),
    policyId: asString(record.policy_id),
    policyRevisionId: asString(record.policy_revision_id),
    policyRevision: asString(record.policy_revision),
    credentialProfileId: asString(record.credential_profile_id),
    credentialSummary: asString(record.credential_summary),
    executorKind: asString(record.executor_kind),
    actionSummary: asString(record.action_summary),
    targets: asArray(record.targets).map(normalizePlanTarget),
    bootstrapPreviews: asArray(record.bootstrap_previews).map(
      normalizeBootstrapPreview
    ),
    warnings: asStringArray(record.warnings),
    raw: value,
  };
};

function buildQuery(params: Record<string, QueryValue>): string {
  const search = new URLSearchParams();
  Object.entries(params).forEach(([key, value]) => {
    if (value === undefined || value === null || value === "") {
      return;
    }
    search.set(key, String(value));
  });
  const encoded = search.toString();
  return encoded ? `?${encoded}` : "";
}

async function ensureCsrfToken(): Promise<void> {
  if (getCsrfToken()) {
    return;
  }
  await fetchCsrfToken(process.env.NEXT_PUBLIC_API_BASE_URL ?? "/api/edge");
}

export function getDashboardOverview() {
  return runtimeApiClient.get<DashboardOverviewResponse>(
    "/api/v1/dashboards/overview"
  );
}

export function listHosts() {
  return runtimeApiClient.get<{ items: HostItem[]; request_id: string }>(
    "/api/v1/hosts"
  );
}

export function listHostGroups() {
  return runtimeApiClient.get<{ items: HostGroupItem[]; request_id: string }>(
    "/api/v1/host-groups"
  );
}

export function listCredentials() {
  return runtimeApiClient.get<{ items: CredentialItem[]; request_id: string }>(
    "/api/v1/credentials"
  );
}

export function listClusters(
  params: {
    limit?: number;
    offset?: number;
    include_members?: boolean;
  } = {}
) {
  return runtimeApiClient
    .get<{
      items: ClusterItem[];
      limit: number;
      offset: number;
      total: number;
      request_id: string;
    }>(`/api/v1/clusters${buildQuery(params)}`)
    .then((response) => ({
      ...response,
      items: response.items.map(normalizeClusterItem),
    }));
}

export function listIntegrations(
  params: {
    limit?: number;
    offset?: number;
  } = {}
) {
  return runtimeApiClient
    .get<{
      items: IntegrationItem[];
      limit: number;
      offset: number;
      total: number;
      request_id: string;
    }>(`/api/v1/integrations${buildQuery(params)}`)
    .then((response) => ({
      ...response,
      items: response.items.map(normalizeIntegrationItem),
    }));
}

export function getIntegration(id: string) {
  return runtimeApiClient
    .get<{
      item: IntegrationItem;
      request_id: string;
    }>(`/api/v1/integrations/${encodePathSegment(id)}`)
    .then((response) => ({
      ...response,
      item: normalizeIntegrationItem(response.item),
    }));
}

export async function createIntegration(
  payload: IntegrationUpsertPayload,
  signal?: AbortSignal
) {
  await ensureCsrfToken();
  const response = await runtimeApiClient.postWithMeta<unknown>(
    "/api/v1/integrations",
    payload,
    { signal }
  );
  const record = asRecord(response.data);

  return {
    meta: response.meta,
    data: {
      item: normalizeIntegrationItem(record.item ?? response.data),
      request_id: asString(record.request_id) ?? response.meta.requestId ?? "",
    },
  };
}

export async function updateIntegration(
  id: string,
  payload: IntegrationUpsertPayload,
  signal?: AbortSignal
) {
  await ensureCsrfToken();
  const response = await runtimeApiClient.patchWithMeta<unknown>(
    `/api/v1/integrations/${encodePathSegment(id)}`,
    payload,
    { signal }
  );
  const record = asRecord(response.data);

  return {
    meta: response.meta,
    data: {
      item: normalizeIntegrationItem(record.item ?? response.data),
      request_id: asString(record.request_id) ?? response.meta.requestId ?? "",
    },
  };
}

export async function createIntegrationBinding(
  integrationId: string,
  payload: IntegrationBindingPayload,
  signal?: AbortSignal
) {
  await ensureCsrfToken();
  const response = await runtimeApiClient.postWithMeta<unknown>(
    `/api/v1/integrations/${encodePathSegment(integrationId)}/bindings`,
    payload,
    { signal }
  );
  const record = asRecord(response.data);

  return {
    meta: response.meta,
    data: {
      item: normalizeIntegrationBindingItem(record.item ?? response.data),
      request_id: asString(record.request_id) ?? response.meta.requestId ?? "",
    },
  };
}

export async function deleteIntegrationBinding(
  integrationId: string,
  bindingId: string,
  signal?: AbortSignal
) {
  await ensureCsrfToken();
  return runtimeApiClient.deleteWithMeta<{
    status: string;
    request_id: string;
  }>(
    `/api/v1/integrations/${encodePathSegment(integrationId)}/bindings/${encodePathSegment(bindingId)}`,
    { signal }
  );
}

export async function requestTelegramIntegrationHealthcheck(
  integrationId: string,
  payload: {
    chat_id_override?: string;
    reason?: string;
  } = {},
  signal?: AbortSignal
) {
  await ensureCsrfToken();
  return runtimeApiClient.postWithMeta<TelegramIntegrationHealthcheckResponse>(
    `/api/v1/integrations/${encodePathSegment(integrationId)}/telegram/healthcheck`,
    payload,
    { signal }
  );
}

export async function getPolicies(
  options: {
    signal?: AbortSignal;
    cursor?: string;
  } = {}
): Promise<RuntimeRequestResult<PoliciesList>> {
  const response = await runtimeApiClient.getWithMeta<unknown>(
    "/api/v1/policies",
    {
      signal: options.signal,
      query: { cursor: options.cursor },
    }
  );
  const record = asRecord(response.data);

  return {
    meta: response.meta,
    data: {
      items: asArray(record.items).map(normalizePolicy),
      nextCursor: asString(record.next_cursor),
      raw: response.data,
    },
  };
}

export async function getPolicyById(
  id: string,
  signal?: AbortSignal
): Promise<RuntimeRequestResult<PolicySummary>> {
  const response = await runtimeApiClient.getWithMeta<unknown>(
    `/api/v1/policies/${encodePathSegment(id)}`,
    { signal }
  );
  const record = asRecord(response.data);

  return {
    meta: response.meta,
    data: normalizePolicy(record.item ?? response.data),
  };
}

export function listPolicies() {
  return runtimeApiClient.get<{ items: PolicyItem[]; request_id: string }>(
    "/api/v1/policies"
  );
}

export function getPolicyRevisions(policyId: string) {
  return runtimeApiClient.get<{
    items: PolicyRevisionItem[];
    request_id: string;
  }>(`/api/v1/policies/${policyId}/revisions`);
}

export async function getDeployments(
  options: {
    signal?: AbortSignal;
    cursor?: string;
  } = {}
): Promise<RuntimeRequestResult<DeploymentsList>> {
  const response = await runtimeApiClient.getWithMeta<unknown>(
    "/api/v1/deployments",
    {
      signal: options.signal,
      query: { cursor: options.cursor },
    }
  );
  const record = asRecord(response.data);

  return {
    meta: response.meta,
    data: {
      items: asArray(record.items ?? record.jobs).map(
        normalizeDeploymentSummary
      ),
      nextCursor: asString(record.next_cursor),
      total: asNumber(record.total),
      raw: response.data,
    },
  };
}

export function listDeployments() {
  return runtimeApiClient.get<{
    items: DeploymentJobItem[];
    total: number;
    limit: number;
    offset: number;
    request_id: string;
  }>("/api/v1/deployments");
}

export async function getDeploymentById(
  id: string,
  signal?: AbortSignal
): Promise<RuntimeRequestResult<DeploymentDetails>> {
  const response = await runtimeApiClient.getWithMeta<unknown>(
    `/api/v1/deployments/${encodePathSegment(id)}`,
    { signal }
  );

  return {
    meta: response.meta,
    data: normalizeDeploymentDetails(response.data),
  };
}

export function getDeployment(jobId: string) {
  return runtimeApiClient.get<DeploymentDetailResponse>(
    `/api/v1/deployments/${jobId}`
  );
}

export async function createDeploymentPlan(
  payload: DeploymentMutationPayload,
  signal?: AbortSignal
): Promise<RuntimeRequestResult<DeploymentPlan>> {
  await ensureCsrfToken();
  const response = await runtimeApiClient.postWithMeta<unknown>(
    "/api/v1/deployments/plan",
    serializeDeploymentPayload(payload),
    { signal }
  );

  return {
    meta: response.meta,
    data: normalizeDeploymentPlan(response.data),
  };
}

export async function createDeployment(
  payload: DeploymentMutationPayload,
  signal?: AbortSignal
): Promise<RuntimeRequestResult<DeploymentSummary>> {
  await ensureCsrfToken();
  const response = await runtimeApiClient.postWithMeta<unknown>(
    "/api/v1/deployments",
    serializeDeploymentPayload(payload),
    { signal }
  );
  const record = asRecord(response.data);

  return {
    meta: response.meta,
    data: normalizeDeploymentSummary(
      record.item ?? record.job ?? response.data
    ),
  };
}

export async function retryDeployment(
  id: string,
  payload?: Record<string, unknown>,
  signal?: AbortSignal
): Promise<RuntimeRequestResult<DeploymentSummary>> {
  await ensureCsrfToken();
  const response = await runtimeApiClient.postWithMeta<unknown>(
    `/api/v1/deployments/${encodePathSegment(id)}/retry`,
    payload,
    { signal }
  );
  const record = asRecord(response.data);

  return {
    meta: response.meta,
    data: normalizeDeploymentSummary(
      record.item ?? record.job ?? response.data
    ),
  };
}

export async function cancelDeployment(
  id: string,
  payload?: Record<string, unknown>,
  signal?: AbortSignal
): Promise<RuntimeRequestResult<DeploymentSummary>> {
  await ensureCsrfToken();
  const response = await runtimeApiClient.postWithMeta<unknown>(
    `/api/v1/deployments/${encodePathSegment(id)}/cancel`,
    payload,
    { signal }
  );
  const record = asRecord(response.data);

  return {
    meta: response.meta,
    data: normalizeDeploymentSummary(
      record.item ?? record.job ?? response.data
    ),
  };
}

export async function issueBootstrapToken(
  payload: BootstrapTokenIssuePayload,
  signal?: AbortSignal
): Promise<RuntimeRequestResult<BootstrapTokenItem>> {
  await ensureCsrfToken();
  const response = await runtimeApiClient.postWithMeta<unknown>(
    "/api/v1/agents/bootstrap-tokens",
    {
      policy_id: payload.policyId,
      policy_revision_id: payload.policyRevisionId,
      requested_by: payload.requestedBy,
      expires_at_unix_ms: payload.expiresAtUnixMs,
    },
    { signal }
  );
  const record = asRecord(response.data);

  return {
    meta: response.meta,
    data: normalizeBootstrapToken(record.item ?? response.data),
  };
}

export function listAgents() {
  return runtimeApiClient.get<{ items: AgentItem[]; request_id: string }>(
    "/api/v1/agents"
  );
}

export async function getAgentsRegistry(
  options: {
    signal?: AbortSignal;
  } = {}
): Promise<RuntimeRequestResult<AgentRegistryList>> {
  const response = await runtimeApiClient.getWithMeta<unknown>(
    "/api/v1/agents",
    {
      signal: options.signal,
    }
  );
  const record = asRecord(response.data);

  return {
    meta: response.meta,
    data: {
      items: asArray(record.items).map(normalizeAgentItem),
      raw: response.data,
    },
  };
}

export function getAgentDiagnostics(agentId: string) {
  return runtimeApiClient.get<{
    items: AgentDiagnosticsItem[];
    request_id: string;
  }>(`/api/v1/agents/${agentId}/diagnostics`);
}

export function getDeploymentStatusCategory(status?: string) {
  switch ((status ?? "").toLowerCase()) {
    case "accepted":
    case "queued":
      return "default";
    case "running":
      return "warning";
    case "succeeded":
    case "healthy":
    case "ready":
    case "online":
    case "ok":
      return "success";
    case "partial_success":
    case "warn":
      return "warning";
    case "failed":
    case "cancelled":
    case "offline":
    case "unavailable":
    case "error":
      return "danger";
    default:
      return "default";
  }
}

export function canRetryDeployment(status?: string): boolean {
  const normalized = (status ?? "").toLowerCase();
  return ["failed", "partial_success", "cancelled"].includes(normalized);
}

export function canCancelDeployment(status?: string): boolean {
  const normalized = (status ?? "").toLowerCase();
  return ["accepted", "queued", "running"].includes(normalized);
}

export async function searchLogs(input: {
  query?: string;
  from?: string;
  to?: string;
  host?: string;
  service?: string;
  severity?: string;
  agentId?: string;
  limit?: number;
  offset?: number;
}) {
  await ensureCsrfToken();
  return runtimeApiClient.post<LogSearchResponse>("/api/v1/logs/search", {
    query: input.query ?? "",
    from: input.from ?? "",
    to: input.to ?? "",
    host: input.host ?? "",
    service: input.service ?? "",
    severity: input.severity ?? "",
    agent_id: input.agentId ?? "",
    limit: input.limit ?? 20,
    offset: input.offset ?? 0,
  });
}

export function listLogAnomalies(
  params: {
    host?: string;
    service?: string;
    severity?: string;
    limit?: number;
    offset?: number;
  } = {}
) {
  return runtimeApiClient.get<LogAnomaliesResponse>(
    `/api/v1/logs/anomalies${buildQuery(params)}`
  );
}

export function listAlerts(
  params: {
    status?: string;
    severity?: string;
    host?: string;
    service?: string;
    limit?: number;
    offset?: number;
  } = {}
) {
  return runtimeApiClient.get<AlertInstancesResponse>(
    `/api/v1/alerts${buildQuery(params)}`
  );
}

export function listAlertRules(
  params: {
    query?: string;
    status?: string;
    limit?: number;
    offset?: number;
  } = {}
) {
  return runtimeApiClient.get<AlertRulesResponse>(
    `/api/v1/alerts/rules${buildQuery(params)}`
  );
}

export function listAudit(
  params: {
    event_type?: string;
    entity_type?: string;
    entity_id?: string;
    actor_id?: string;
    limit?: number;
    offset?: number;
  } = {}
) {
  return runtimeApiClient.get<AuditEventsResponse>(
    `/api/v1/audit${buildQuery(params)}`
  );
}
