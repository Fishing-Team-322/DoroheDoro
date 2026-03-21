import type { ApiResponseMeta } from "@/src/shared/lib/api";

export type RequestResult<T> = {
  data: T;
  meta: ApiResponseMeta;
};

export type StatusResponse = {
  status: string;
};

export type AuthContext = {
  subject: string;
  role: string;
  agentId?: string;
};

export type AuthMeta = {
  mode: string;
  rbac: string;
};

export type MeResponse = {
  user: AuthContext;
  auth: AuthMeta;
};

export type Agent = {
  id: string;
  host: string;
  status: string;
  policyId?: string;
  lastSeenAt?: string;
  labels: Record<string, string>;
  raw?: unknown;
};

export type AgentsList = {
  items: Agent[];
  nextCursor?: string;
  raw?: unknown;
};

export type DiagnosticCheck = {
  name: string;
  status: string;
  message?: string;
};

export type AgentDiagnostics = {
  agentId: string;
  status: string;
  collectedAt?: string;
  checks: DiagnosticCheck[];
  payload?: unknown;
  raw?: unknown;
};

export type Policy = {
  id: string;
  name: string;
  revision?: string;
  description?: string;
  targets: string[];
  params?: Record<string, unknown>;
  body?: unknown;
  raw?: unknown;
};

export type PoliciesList = {
  items: Policy[];
  nextCursor?: string;
  raw?: unknown;
};

export type DeploymentSummary = {
  id: string;
  jobType?: string;
  status: string;
  policyId?: string;
  createdAt?: string;
  startedAt?: string;
  finishedAt?: string;
  updatedAt?: string;
  currentPhase?: string;
  requestedBy?: string;
  credentialProfileId?: string;
  executorKind?: string;
  agentIds: string[];
  params?: Record<string, unknown>;
  totalTargets?: number;
  pendingTargets?: number;
  runningTargets?: number;
  succeededTargets?: number;
  failedTargets?: number;
  cancelledTargets?: number;
  attemptCount?: number;
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

export type DeploymentsList = {
  items: DeploymentSummary[];
  nextCursor?: string;
  total?: number;
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

export type DeploymentMutationPayload = {
  policyId: string;
  agentIds?: string[];
  params?: Record<string, string>;
};

export type LogEntry = {
  timestamp: string;
  agentId?: string;
  host?: string;
  service?: string;
  severity: string;
  message: string;
  labels: Record<string, string>;
  raw?: unknown;
};

export type LogSearchFilters = {
  query?: string;
  from?: string;
  to?: string;
  host?: string;
  service?: string;
  severity?: string;
  agentId?: string;
  limit?: number;
  cursor?: string;
};

export type LogSearchResponse = {
  items: LogEntry[];
  nextCursor?: string;
  total?: number;
  raw?: unknown;
};

export type HistogramBucket = {
  ts: string;
  count: number;
};

export type NamedCount = {
  name: string;
  count: number;
};

export type HistogramResponse = {
  items: HistogramBucket[];
  raw?: unknown;
};

export type NamedCountsResponse = {
  items: NamedCount[];
  raw?: unknown;
};

export type LiveLogsFilters = Pick<
  LogSearchFilters,
  "host" | "service" | "severity"
>;
