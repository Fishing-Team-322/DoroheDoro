"use client";

import { createApiClient } from "@/src/shared/lib/api";
import { clearCsrfToken, fetchCsrfToken, getCsrfToken } from "@/src/shared/lib/auth/csrf";
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

export type AgentItem = {
  agent_id: string;
  hostname: string;
  status: string;
  version: string;
  metadata_json: Record<string, unknown>;
  first_seen_at: string;
  last_seen_at: string;
};

export type AgentDiagnosticsItem = {
  agent_id: string;
  payload_json: Record<string, unknown>;
  created_at: string;
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
  return runtimeApiClient.get<DashboardOverviewResponse>("/api/v1/dashboards/overview");
}

export function listHosts() {
  return runtimeApiClient.get<{ items: HostItem[]; request_id: string }>("/api/v1/hosts");
}

export function listHostGroups() {
  return runtimeApiClient.get<{ items: HostGroupItem[]; request_id: string }>("/api/v1/host-groups");
}

export function listCredentials() {
  return runtimeApiClient.get<{ items: CredentialItem[]; request_id: string }>("/api/v1/credentials");
}

export function listPolicies() {
  return runtimeApiClient.get<{ items: PolicyItem[]; request_id: string }>("/api/v1/policies");
}

export function getPolicyRevisions(policyId: string) {
  return runtimeApiClient.get<{ items: PolicyRevisionItem[]; request_id: string }>(
    `/api/v1/policies/${policyId}/revisions`
  );
}

export function listDeployments() {
  return runtimeApiClient.get<{ items: DeploymentJobItem[]; total: number; limit: number; offset: number; request_id: string }>(
    "/api/v1/deployments"
  );
}

export function getDeployment(jobId: string) {
  return runtimeApiClient.get<DeploymentDetailResponse>(`/api/v1/deployments/${jobId}`);
}

export function listAgents() {
  return runtimeApiClient.get<{ items: AgentItem[]; request_id: string }>("/api/v1/agents");
}

export function getAgentDiagnostics(agentId: string) {
  return runtimeApiClient.get<{ items: AgentDiagnosticsItem[]; request_id: string }>(
    `/api/v1/agents/${agentId}/diagnostics`
  );
}

export async function searchLogs(input: {
  query?: string;
  from?: string;
  to?: string;
  host?: string;
  service?: string;
  severity?: string;
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
    limit: input.limit ?? 20,
    offset: input.offset ?? 0,
  });
}

export function listLogAnomalies(params: {
  host?: string;
  service?: string;
  severity?: string;
  limit?: number;
  offset?: number;
} = {}) {
  return runtimeApiClient.get<LogAnomaliesResponse>(
    `/api/v1/logs/anomalies${buildQuery(params)}`
  );
}

export function listAlerts(params: {
  status?: string;
  severity?: string;
  host?: string;
  service?: string;
  limit?: number;
  offset?: number;
} = {}) {
  return runtimeApiClient.get<AlertInstancesResponse>(
    `/api/v1/alerts${buildQuery(params)}`
  );
}

export function listAlertRules(params: {
  query?: string;
  status?: string;
  limit?: number;
  offset?: number;
} = {}) {
  return runtimeApiClient.get<AlertRulesResponse>(
    `/api/v1/alerts/rules${buildQuery(params)}`
  );
}

export function listAudit(params: {
  event_type?: string;
  entity_type?: string;
  entity_id?: string;
  actor_id?: string;
  limit?: number;
  offset?: number;
} = {}) {
  return runtimeApiClient.get<AuditEventsResponse>(
    `/api/v1/audit${buildQuery(params)}`
  );
}
