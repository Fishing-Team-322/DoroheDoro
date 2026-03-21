import {
  createApiClient,
  type ApiResult,
  type QueryParameters,
} from "@/src/shared/lib/api";
import type {
  Agent,
  AgentDiagnostics,
  AgentsList,
  BootstrapPreview,
  DeploymentAttempt,
  DeploymentDetails,
  DeploymentMutationPayload,
  DeploymentPlan,
  DeploymentPlanTarget,
  DeploymentsList,
  DeploymentStep,
  DeploymentSummary,
  DeploymentTarget,
  HistogramBucket,
  HistogramResponse,
  LiveLogsFilters,
  LogEntry,
  LogSearchFilters,
  LogSearchResponse,
  MeResponse,
  NamedCount,
  NamedCountsResponse,
  PoliciesList,
  Policy,
  RequestResult,
  StatusResponse,
} from "./types";

const EDGE_API_BASE_URL = process.env.NEXT_PUBLIC_API_BASE_URL ?? "/api/edge";

const edgeApiClient = createApiClient({
  baseUrl: EDGE_API_BASE_URL,
  credentials: "include",
  timeoutMs: 15_000,
});

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

const asStringMap = (value: unknown): Record<string, string> => {
  if (!isRecord(value)) {
    return {};
  }

  return Object.fromEntries(
    Object.entries(value)
      .map(([key, item]) => [key, typeof item === "string" ? item : String(item)])
      .filter(([, item]) => Boolean(item))
  );
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

const withMeta = <T>(response: ApiResult<T>): RequestResult<T> => {
  return {
    data: response.data,
    meta: response.meta,
  };
};

const buildApiUrl = (
  path: string,
  query?: QueryParameters
): string => {
  const base = EDGE_API_BASE_URL.endsWith("/")
    ? EDGE_API_BASE_URL.slice(0, -1)
    : EDGE_API_BASE_URL;
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  const absolute = /^https?:\/\//i.test(base);
  const url = new URL(`${base}${normalizedPath}`, "http://api.local");

  if (query) {
    for (const [key, rawValue] of Object.entries(query)) {
      if (rawValue == null || rawValue === "") {
        continue;
      }

      const values = Array.isArray(rawValue) ? rawValue : [rawValue];
      for (const value of values) {
        if (value == null || value === "") {
          continue;
        }
        url.searchParams.append(key, String(value));
      }
    }
  }

  if (absolute) {
    return url.toString();
  }

  return `${url.pathname}${url.search}`;
};

const toLogsQuery = (
  filters: LogSearchFilters,
  extra?: QueryParameters
): QueryParameters => {
  return {
    query: filters.query,
    from: filters.from,
    to: filters.to,
    host: filters.host,
    service: filters.service,
    severity: filters.severity,
    agent_id: filters.agentId,
    limit: filters.limit,
    cursor: filters.cursor,
    ...extra,
  };
};

const toLogsBody = (filters: LogSearchFilters): Record<string, unknown> => {
  return {
    query: filters.query,
    from: filters.from,
    to: filters.to,
    host: filters.host,
    service: filters.service,
    severity: filters.severity,
    agent_id: filters.agentId,
    limit: filters.limit,
    cursor: filters.cursor,
  };
};

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

const normalizeAgent = (value: unknown): Agent => {
  const record = asRecord(value);

  return {
    id: asString(record.id ?? record.agent_id) ?? "unknown-agent",
    host: asString(record.host ?? record.hostname) ?? "unknown-host",
    status: asString(record.status) ?? "unknown",
    policyId: asString(record.policy_id),
    lastSeenAt: asString(record.last_seen_at ?? record.last_seen),
    labels: asStringMap(record.labels),
    raw: value,
  };
};

const normalizeAgentDiagnostics = (value: unknown): AgentDiagnostics => {
  const record = asRecord(value);
  const checks = asArray(record.checks).map((item) => {
    const check = asRecord(item);
    return {
      name: asString(check.name) ?? "unknown",
      status: asString(check.status) ?? "unknown",
      message: asString(check.message),
    };
  });

  return {
    agentId: asString(record.agent_id ?? record.id) ?? "unknown-agent",
    status: asString(record.status) ?? "unknown",
    collectedAt: asString(record.collected_at),
    checks,
    payload: pickObject(record.payload) ?? parseJsonString(record.payload_json),
    raw: value,
  };
};

const normalizePolicy = (value: unknown): Policy => {
  const record = asRecord(value);
  return {
    id: asString(record.id ?? record.policy_id) ?? "unknown-policy",
    name: asString(record.name) ?? asString(record.policy_id) ?? "Unnamed policy",
    revision: asString(record.revision ?? record.latest_revision),
    description: asString(record.description),
    targets: asStringArray(record.targets),
    params:
      pickObject(record.params) ??
      pickObject(record.parameters) ??
      undefined,
    body: parseJsonString(record.policy_body_json),
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
      asString(record.job_type) ??
      asString(summary?.job_type) ??
      undefined,
    status:
      asString(record.status) ??
      asString(summary?.status) ??
      "unknown",
    policyId:
      asString(record.policy_id) ??
      asString(summary?.policy_id) ??
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
    agentIds: asStringArray(record.agent_ids),
    params:
      pickObject(record.params) ??
      pickObject(payload?.params) ??
      undefined,
    totalTargets:
      asNumber(record.total_targets) ??
      asNumber(summary?.total_targets),
    pendingTargets:
      asNumber(record.pending_targets) ??
      asNumber(summary?.pending_targets),
    runningTargets:
      asNumber(record.running_targets) ??
      asNumber(summary?.running_targets),
    succeededTargets:
      asNumber(record.succeeded_targets) ??
      asNumber(summary?.succeeded_targets),
    failedTargets:
      asNumber(record.failed_targets) ??
      asNumber(summary?.failed_targets),
    cancelledTargets:
      asNumber(record.cancelled_targets) ??
      asNumber(summary?.cancelled_targets),
    attemptCount:
      asNumber(record.attempt_count) ??
      asNumber(summary?.attempt_count),
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
    id:
      asString(record.deployment_target_id ?? record.id) ?? "unknown-target",
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

const normalizeLogEntry = (value: unknown): LogEntry => {
  const record = asRecord(value);
  return {
    timestamp: asString(record.timestamp) ?? "",
    agentId: asString(record.agent_id),
    host: asString(record.host),
    service: asString(record.service),
    severity: asString(record.severity) ?? "unknown",
    message: asString(record.message) ?? "",
    labels: asStringMap(record.labels),
    raw: value,
  };
};

const normalizeHistogramBucket = (value: unknown): HistogramBucket => {
  const record = asRecord(value);
  return {
    ts: asString(record.ts ?? record.bucket) ?? "",
    count: asNumber(record.count) ?? 0,
  };
};

const normalizeNamedCount = (value: unknown): NamedCount => {
  const record = asRecord(value);
  return {
    name: asString(record.name ?? record.key) ?? "unknown",
    count: asNumber(record.count) ?? 0,
  };
};

export async function getHealthStatus(
  signal?: AbortSignal
): Promise<RequestResult<StatusResponse>> {
  return withMeta(
    await edgeApiClient.getWithMeta<StatusResponse>("/healthz", { signal })
  );
}

export async function getReadinessStatus(
  signal?: AbortSignal
): Promise<RequestResult<StatusResponse>> {
  return withMeta(
    await edgeApiClient.getWithMeta<StatusResponse>("/readyz", { signal })
  );
}

export async function getAuthContext(
  signal?: AbortSignal
): Promise<RequestResult<MeResponse>> {
  return withMeta(
    await edgeApiClient.getWithMeta<MeResponse>("/api/v1/me", { signal })
  );
}

export async function listAgents(
  options: {
    signal?: AbortSignal;
    cursor?: string;
  } = {}
): Promise<RequestResult<AgentsList>> {
  const response = await edgeApiClient.getWithMeta<unknown>("/api/v1/agents", {
    signal: options.signal,
    query: { cursor: options.cursor },
  });
  const record = asRecord(response.data);

  return {
    meta: response.meta,
    data: {
      items: asArray(record.items).map(normalizeAgent),
      nextCursor: asString(record.next_cursor),
      raw: response.data,
    },
  };
}

export async function getAgent(
  id: string,
  signal?: AbortSignal
): Promise<RequestResult<Agent>> {
  const response = await edgeApiClient.getWithMeta<unknown>(
    `/api/v1/agents/${encodePathSegment(id)}`,
    { signal }
  );
  const record = asRecord(response.data);
  return {
    meta: response.meta,
    data: normalizeAgent(record.item ?? response.data),
  };
}

export async function getAgentDiagnostics(
  id: string,
  signal?: AbortSignal
): Promise<RequestResult<AgentDiagnostics>> {
  const response = await edgeApiClient.getWithMeta<unknown>(
    `/api/v1/agents/${encodePathSegment(id)}/diagnostics`,
    { signal }
  );
  const record = asRecord(response.data);
  return {
    meta: response.meta,
    data: normalizeAgentDiagnostics(record.item ?? response.data),
  };
}

export async function listPolicies(
  options: {
    signal?: AbortSignal;
    cursor?: string;
  } = {}
): Promise<RequestResult<PoliciesList>> {
  const response = await edgeApiClient.getWithMeta<unknown>("/api/v1/policies", {
    signal: options.signal,
    query: { cursor: options.cursor },
  });
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

export async function getPolicy(
  id: string,
  signal?: AbortSignal
): Promise<RequestResult<Policy>> {
  const response = await edgeApiClient.getWithMeta<unknown>(
    `/api/v1/policies/${encodePathSegment(id)}`,
    { signal }
  );
  const record = asRecord(response.data);
  return {
    meta: response.meta,
    data: normalizePolicy(record.item ?? response.data),
  };
}

export async function listDeployments(
  options: {
    signal?: AbortSignal;
    cursor?: string;
  } = {}
): Promise<RequestResult<DeploymentsList>> {
  const response = await edgeApiClient.getWithMeta<unknown>(
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
      items: asArray(record.items ?? record.jobs).map(normalizeDeploymentSummary),
      nextCursor: asString(record.next_cursor),
      total: asNumber(record.total),
      raw: response.data,
    },
  };
}

export async function getDeployment(
  id: string,
  signal?: AbortSignal
): Promise<RequestResult<DeploymentDetails>> {
  const response = await edgeApiClient.getWithMeta<unknown>(
    `/api/v1/deployments/${encodePathSegment(id)}`,
    { signal }
  );
  return {
    meta: response.meta,
    data: normalizeDeploymentDetails(response.data),
  };
}

export async function previewDeploymentPlan(
  payload: DeploymentMutationPayload,
  signal?: AbortSignal
): Promise<RequestResult<DeploymentPlan>> {
  const response = await edgeApiClient.postWithMeta<unknown>(
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
): Promise<RequestResult<DeploymentSummary>> {
  const response = await edgeApiClient.postWithMeta<unknown>(
    "/api/v1/deployments",
    serializeDeploymentPayload(payload),
    { signal }
  );
  const record = asRecord(response.data);
  return {
    meta: response.meta,
    data: normalizeDeploymentSummary(record.item ?? record.job ?? response.data),
  };
}

export async function retryDeployment(
  id: string,
  signal?: AbortSignal
): Promise<RequestResult<DeploymentSummary>> {
  const response = await edgeApiClient.postWithMeta<unknown>(
    `/api/v1/deployments/${encodePathSegment(id)}/retry`,
    undefined,
    { signal }
  );
  const record = asRecord(response.data);
  return {
    meta: response.meta,
    data: normalizeDeploymentSummary(record.item ?? record.job ?? response.data),
  };
}

export async function cancelDeployment(
  id: string,
  signal?: AbortSignal
): Promise<RequestResult<DeploymentSummary>> {
  const response = await edgeApiClient.postWithMeta<unknown>(
    `/api/v1/deployments/${encodePathSegment(id)}/cancel`,
    undefined,
    { signal }
  );
  const record = asRecord(response.data);
  return {
    meta: response.meta,
    data: normalizeDeploymentSummary(record.item ?? record.job ?? response.data),
  };
}

export async function searchLogs(
  filters: LogSearchFilters,
  signal?: AbortSignal
): Promise<RequestResult<LogSearchResponse>> {
  const response = await edgeApiClient.postWithMeta<unknown>(
    "/api/v1/logs/search",
    toLogsBody(filters),
    { signal }
  );
  const record = asRecord(response.data);
  return {
    meta: response.meta,
    data: {
      items: asArray(record.items).map(normalizeLogEntry),
      nextCursor: asString(record.next_cursor),
      total: asNumber(record.total),
      raw: response.data,
    },
  };
}

export async function getLogsHistogram(
  filters: LogSearchFilters,
  signal?: AbortSignal,
  interval = "5m"
): Promise<RequestResult<HistogramResponse>> {
  const response = await edgeApiClient.getWithMeta<unknown>(
    "/api/v1/logs/histogram",
    {
      signal,
      query: toLogsQuery(filters, { interval }),
    }
  );
  const record = asRecord(response.data);
  return {
    meta: response.meta,
    data: {
      items: asArray(record.items).map(normalizeHistogramBucket),
      raw: response.data,
    },
  };
}

async function getNamedCounts(
  path: string,
  filters: LogSearchFilters,
  signal?: AbortSignal,
  top = 5
): Promise<RequestResult<NamedCountsResponse>> {
  const response = await edgeApiClient.getWithMeta<unknown>(path, {
    signal,
    query: toLogsQuery(filters, { top }),
  });
  const record = asRecord(response.data);
  return {
    meta: response.meta,
    data: {
      items: asArray(record.items).map(normalizeNamedCount),
      raw: response.data,
    },
  };
}

export async function getLogsSeverity(
  filters: LogSearchFilters,
  signal?: AbortSignal,
  top = 5
): Promise<RequestResult<NamedCountsResponse>> {
  return getNamedCounts("/api/v1/logs/severity", filters, signal, top);
}

export async function getLogsTopHosts(
  filters: LogSearchFilters,
  signal?: AbortSignal,
  top = 5
): Promise<RequestResult<NamedCountsResponse>> {
  return getNamedCounts("/api/v1/logs/top-hosts", filters, signal, top);
}

export async function getLogsTopServices(
  filters: LogSearchFilters,
  signal?: AbortSignal,
  top = 5
): Promise<RequestResult<NamedCountsResponse>> {
  return getNamedCounts("/api/v1/logs/top-services", filters, signal, top);
}

export function buildLiveLogsStreamUrl(filters: LiveLogsFilters = {}): string {
  return buildApiUrl("/api/v1/stream/logs", {
    host: filters.host,
    service: filters.service,
    severity: filters.severity,
  });
}

export function parseLiveLogEvent(value: unknown): LogEntry | null {
  const record = asRecord(value);
  const payload = record.payload ?? value;

  if (!payload) {
    return null;
  }

  const entry = normalizeLogEntry(payload);
  if (!entry.timestamp && !entry.message) {
    return null;
  }

  return entry;
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
    case "online":
    case "ready":
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

export function isTerminalDeploymentStatus(status?: string): boolean {
  const normalized = (status ?? "").toLowerCase();
  return ["succeeded", "failed", "cancelled", "partial_success"].includes(
    normalized
  );
}

export function hasOperationalReadiness(status?: string): boolean {
  const normalized = (status ?? "").toLowerCase();
  return ["ready", "ok", "online", "healthy", "authenticated"].includes(
    normalized
  );
}

export function boolToStatus(value?: boolean): string {
  if (value == null) {
    return "unknown";
  }

  return value ? "ok" : "unavailable";
}

export function isTruthyState(value?: string): boolean {
  return hasOperationalReadiness(value);
}
