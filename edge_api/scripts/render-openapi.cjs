#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '..');
const docsDir = path.join(root, 'docs');
const checkOnly = process.argv.includes('--check');

const str = { type: 'string' };
const bool = { type: 'boolean' };
const dt = { type: 'string', format: 'date-time' };
const u32 = { type: 'integer', minimum: 0 };
const u64 = { type: 'integer', minimum: 0 };
const freeObject = { type: 'object', additionalProperties: true };

const ref = (name) => ({ $ref: `#/components/schemas/${name}` });
const headerRef = (name) => ({ $ref: `#/components/headers/${name}` });
const jsonBody = (schema) => ({
  'application/json': {
    schema: typeof schema === 'string' ? ref(schema) : schema,
  },
});

const itemEnvelope = (schema, extra = {}) => ({
  type: 'object',
  properties: {
    item: ref(schema),
    ...extra,
    request_id: str,
  },
  required: ['request_id'],
});

const listEnvelope = (schema, paged = false, extra = {}) => ({
  type: 'object',
  properties: {
    items: { type: 'array', items: ref(schema) },
    ...(paged ? { limit: u32, offset: u64, total: u64 } : {}),
    ...extra,
    request_id: str,
  },
  required: ['items', 'request_id'],
});

const natsHeaders = {
  'X-Request-ID': headerRef('RequestId'),
  'X-NATS-Subject': headerRef('NatsSubject'),
};

const runtimeHeaders = {
  ...natsHeaders,
  'X-Boundary-State': headerRef('BoundaryState'),
};

const response = (description, schema, headers) => {
  const out = { description };
  if (headers) out.headers = headers;
  if (schema) out.content = jsonBody(schema);
  return out;
};

const err = (code, description) => ({
  description,
  headers: { 'X-Request-ID': headerRef('RequestId') },
  content: jsonBody('ErrorEnvelope'),
});

const publicErrors = (...codes) => {
  const bag = {};
  for (const code of codes) {
    if (code === 400) bag[400] = err('invalid_argument', 'Invalid request payload or parameters.');
    if (code === 401) bag[401] = err('unauthorized', 'Authentication is required or the session is missing.');
    if (code === 403) bag[403] = err('forbidden', 'CSRF or permission validation failed.');
    if (code === 404) bag[404] = err('not_found', 'Requested resource was not found.');
    if (code === 501) {
      bag[501] = {
        description: 'The route exists in the boundary, but the backing Rust runtime plane is not available yet.',
        headers: runtimeHeaders,
        content: jsonBody('ErrorEnvelope'),
      };
    }
    if (code === 502) bag[502] = err('internal', 'Invalid upstream response or bridge decode failure.');
    if (code === 503) bag[503] = err('unavailable', 'Upstream runtime or NATS bridge is unavailable.');
    if (code === 504) bag[504] = err('deadline_exceeded', 'Upstream request timed out.');
  }
  return bag;
};

const secure = (operation) => ({ security: [{ SessionCookie: [] }], ...operation });

const q = (name, schema, description) => ({ name, in: 'query', schema, description });
const p = (name, description) => ({ name, in: 'path', required: true, schema: str, description });

const sse = (tag, summary, description) =>
  secure({
    tags: [tag],
    summary,
    description,
    responses: {
      200: {
        description: 'Server-sent events stream.',
        headers: { 'X-Request-ID': headerRef('RequestId') },
        content: { 'text/event-stream': { schema: { type: 'string', format: 'binary' } } },
      },
      ...publicErrors(401),
    },
  });

const spec = {
  openapi: '3.0.3',
  info: {
    title: 'DoroheDoro Edge API',
    version: '0.4.0',
    description:
      'Public boundary API for WEB and AGENT. The Go edge service validates transport payloads, applies web auth or agent mTLS, bridges requests over NATS to server-rs runtime planes, and exposes SSE streams for UI consumers.',
  },
  servers: [
    { url: '/', description: 'Direct edge-api or reverse proxy with stripped prefix.' },
    { url: '/api/edge', description: 'Same-origin frontend proxy path.' },
  ],
  tags: [
    { name: 'system', description: 'Public health, readiness, version and discovery routes.' },
    { name: 'auth', description: 'Frontend compatibility auth routes and stub session inspection.' },
    { name: 'agents', description: 'Agent registry, diagnostics and current policy read-side.' },
    { name: 'policies', description: 'Policies bridged to control-plane.' },
    { name: 'inventory', description: 'Hosts, host groups and credentials metadata.' },
    { name: 'deployments', description: 'Deployment plan, jobs, steps and targets.' },
    { name: 'clusters', description: 'Cluster registry and host membership.' },
    { name: 'roles', description: 'Roles, permissions and role bindings.' },
    { name: 'integrations', description: 'Outbound integrations and scope bindings.' },
    { name: 'tickets', description: 'Ticket lifecycle actions and ticket comments.' },
    { name: 'anomalies', description: 'Anomaly rules and anomaly instances.' },
    { name: 'logs', description: 'Live query-plane log search, context and analytics routes backed by server-rs.' },
    { name: 'dashboards', description: 'Live overview dashboards backed by query-alert-plane.' },
    { name: 'alerts', description: 'Live alert instance and alert rule APIs backed by query-alert-plane.' },
    { name: 'audit', description: 'Live cross-plane audit feed backed by control-plane runtime audit events.' },
    { name: 'stream', description: 'SSE gateway endpoints.' },
  ],
  paths: {},
  components: {
    securitySchemes: {
      SessionCookie: { type: 'apiKey', in: 'cookie', name: 'session_token' },
    },
    headers: {
      RequestId: { description: 'Request correlation ID generated or propagated by the boundary.', schema: str },
      NatsSubject: { description: 'NATS subject bridged by the boundary for the request.', schema: str },
      BoundaryState: { description: 'Boundary runtime state marker for controlled placeholder routes.', schema: { type: 'string', enum: ['awaiting-runtime'] } },
    },
    schemas: {
      ErrorBody: { type: 'object', properties: { code: str, message: str, request_id: str }, required: ['code', 'message', 'request_id'] },
      ErrorEnvelope: { type: 'object', properties: { error: ref('ErrorBody') }, required: ['error'] },
      StatusResponse: { type: 'object', properties: { status: str }, required: ['status'] },
      VersionResponse: { type: 'object', properties: { version: str }, required: ['version'] },
      RootResponse: { type: 'object', properties: { service: str, version: str, status: str, docs: str, openapi: str, health: str, readiness: str, grpc: str }, required: ['service', 'version', 'status'] },
      AuthContext: { type: 'object', properties: { subject: str, role: str, agent_id: str } },
      MeResponse: { type: 'object', properties: { user: ref('AuthContext'), auth: freeObject }, required: ['user', 'auth'] },
      CompatUser: { type: 'object', properties: { id: str, email: str, login: str, role: str, displayName: str, updatedAt: dt }, required: ['id', 'email', 'login', 'displayName'] },
      CompatSession: { type: 'object', properties: { user: ref('CompatUser'), csrfToken: str, expiresAt: dt }, required: ['user'] },
      CompatLoginRequest: { type: 'object', properties: { identifier: str, email: str, login: str, password: str }, required: ['password'] },
      CompatProfileUpdateRequest: { type: 'object', properties: { displayName: str }, required: ['displayName'] },
      CsrfResponse: { type: 'object', properties: { csrfToken: str }, required: ['csrfToken'] },
      SuccessResponse: { type: 'object', properties: { status: { type: 'string', enum: ['ok'] }, success: bool, request_id: str }, additionalProperties: true },
      FreeFormObject: freeObject,
      PolicyItem: { type: 'object', properties: { policy_id: str, name: str, description: str, is_active: bool, created_at: dt, updated_at: dt, latest_revision: str, latest_body_json: freeObject }, required: ['policy_id', 'name', 'is_active', 'created_at', 'updated_at'] },
      PolicyRevisionItem: { type: 'object', properties: { policy_revision_id: str, revision: str, body_json: freeObject, created_at: dt }, required: ['policy_revision_id', 'revision', 'created_at'] },
      HostItem: { type: 'object', properties: { host_id: str, hostname: str, ip: str, ssh_port: u32, remote_user: str, labels: { type: 'object', additionalProperties: { type: 'string' } }, created_at: dt, updated_at: dt }, required: ['host_id', 'hostname', 'ip', 'ssh_port', 'remote_user', 'created_at', 'updated_at'] },
      HostGroupMemberItem: { type: 'object', properties: { host_group_member_id: str, host_group_id: str, host_id: str, hostname: str }, required: ['host_group_member_id', 'host_group_id', 'host_id'] },
      HostGroupItem: { type: 'object', properties: { host_group_id: str, name: str, description: str, created_at: dt, updated_at: dt, members: { type: 'array', items: ref('HostGroupMemberItem') } }, required: ['host_group_id', 'name', 'created_at', 'updated_at'] },
      CredentialItem: { type: 'object', properties: { credentials_profile_id: str, name: str, kind: str, description: str, vault_ref: str, created_at: dt, updated_at: dt }, required: ['credentials_profile_id', 'name', 'kind', 'vault_ref', 'created_at', 'updated_at'] },
      AgentItem: { type: 'object', properties: { agent_id: str, hostname: str, status: str, version: str, metadata_json: freeObject, first_seen_at: dt, last_seen_at: dt }, required: ['agent_id', 'hostname', 'status', 'first_seen_at', 'last_seen_at'] },
      AgentDiagnosticsItem: { type: 'object', properties: { agent_id: str, payload_json: freeObject, created_at: dt }, required: ['agent_id', 'created_at'] },
      DeploymentJobItem: { type: 'object', properties: { job_id: str, job_type: str, status: str, requested_by: str, policy_id: str, policy_revision_id: str, credential_profile_id: str, executor_kind: str, current_phase: str, total_targets: u32, pending_targets: u32, running_targets: u32, succeeded_targets: u32, failed_targets: u32, cancelled_targets: u32, attempt_count: u32, created_at: dt, started_at: dt, finished_at: dt, updated_at: dt }, required: ['job_id', 'job_type', 'status', 'created_at', 'updated_at'] },
      DeploymentAttemptItem: { type: 'object', properties: { deployment_attempt_id: str, attempt_no: u32, status: str, triggered_by: str, reason: str, created_at: dt, started_at: dt, finished_at: dt }, required: ['deployment_attempt_id', 'attempt_no', 'status', 'created_at'] },
      DeploymentTargetItem: { type: 'object', properties: { deployment_target_id: str, deployment_attempt_id: str, host_id: str, hostname_snapshot: str, status: str, error_message: str, created_at: dt, started_at: dt, finished_at: dt, updated_at: dt }, required: ['deployment_target_id', 'deployment_attempt_id', 'host_id', 'hostname_snapshot', 'status', 'created_at', 'updated_at'] },
      DeploymentStepItem: { type: 'object', properties: { deployment_step_id: str, deployment_attempt_id: str, deployment_target_id: str, step_name: str, status: str, message: str, payload_json: freeObject, created_at: dt, updated_at: dt }, required: ['deployment_step_id', 'deployment_attempt_id', 'step_name', 'status', 'created_at', 'updated_at'] },
      DeploymentPlanTargetItem: { type: 'object', properties: { host_id: str, hostname: str, ip: str, ssh_port: u32, remote_user: str }, required: ['host_id', 'hostname', 'ip', 'ssh_port', 'remote_user'] },
      BootstrapPreviewItem: { type: 'object', properties: { host_id: str, hostname: str, bootstrap_yaml: str }, required: ['host_id', 'hostname', 'bootstrap_yaml'] },
      DeploymentPlanItem: { type: 'object', properties: { job_type: str, policy_id: str, policy_revision_id: str, policy_revision: str, credential_profile_id: str, credential_summary: str, executor_kind: str, action_summary: str, targets: { type: 'array', items: ref('DeploymentPlanTargetItem') }, bootstrap_previews: { type: 'array', items: ref('BootstrapPreviewItem') }, warnings: { type: 'array', items: str } }, required: ['job_type', 'policy_id', 'policy_revision_id', 'credential_profile_id', 'executor_kind', 'targets', 'bootstrap_previews'] },
      ClusterHostBindingItem: { type: 'object', properties: { cluster_host_id: str, host_id: str, hostname: str, created_at: dt }, required: ['cluster_host_id', 'host_id', 'hostname', 'created_at'] },
      ClusterAgentBindingItem: { type: 'object', properties: { cluster_agent_id: str, agent_id: str, created_at: dt }, required: ['cluster_agent_id', 'agent_id', 'created_at'] },
      ClusterItem: { type: 'object', properties: { cluster_id: str, name: str, slug: str, description: str, is_active: bool, created_at: dt, updated_at: dt, created_by: str, updated_by: str, metadata_json: freeObject, host_count: u32, agent_count: u32, hosts: { type: 'array', items: ref('ClusterHostBindingItem') }, agents: { type: 'array', items: ref('ClusterAgentBindingItem') } }, required: ['cluster_id', 'name', 'slug', 'is_active', 'created_at', 'updated_at', 'host_count', 'agent_count'] },
      PermissionItem: { type: 'object', properties: { permission_id: str, code: str, description: str }, required: ['permission_id', 'code'] },
      RoleItem: { type: 'object', properties: { role_id: str, name: str, slug: str, description: str, is_system: bool, created_at: dt, updated_at: dt, created_by: str, updated_by: str, permissions: { type: 'array', items: ref('PermissionItem') } }, required: ['role_id', 'name', 'slug', 'is_system', 'created_at', 'updated_at'] },
      RoleBindingItem: { type: 'object', properties: { role_binding_id: str, user_id: str, role_id: str, scope_type: str, scope_id: str, created_at: dt }, required: ['role_binding_id', 'user_id', 'role_id', 'created_at'] },
      IntegrationBindingItem: { type: 'object', properties: { integration_binding_id: str, integration_id: str, scope_type: str, scope_id: str, event_types_json: freeObject, severity_threshold: str, is_active: bool, created_at: dt, updated_at: dt }, required: ['integration_binding_id', 'integration_id', 'is_active', 'created_at', 'updated_at'] },
      IntegrationItem: { type: 'object', properties: { integration_id: str, name: str, kind: str, description: str, config_json: freeObject, is_active: bool, created_at: dt, updated_at: dt, created_by: str, updated_by: str, bindings: { type: 'array', items: ref('IntegrationBindingItem') } }, required: ['integration_id', 'name', 'kind', 'is_active', 'created_at', 'updated_at'] },
      TicketCommentItem: { type: 'object', properties: { ticket_comment_id: str, ticket_id: str, author_user_id: str, body: str, created_at: dt }, required: ['ticket_comment_id', 'ticket_id', 'body', 'created_at'] },
      TicketEventItem: { type: 'object', properties: { ticket_event_id: str, ticket_id: str, event_type: str, payload_json: freeObject, created_at: dt }, required: ['ticket_event_id', 'ticket_id', 'event_type', 'created_at'] },
      TicketItem: { type: 'object', properties: { ticket_id: str, ticket_key: str, title: str, description: str, cluster_id: str, cluster_name: str, source_type: str, source_id: str, severity: str, status: str, assignee_user_id: str, created_by: str, resolution: str, created_at: dt, updated_at: dt, resolved_at: dt, closed_at: dt, comments: { type: 'array', items: ref('TicketCommentItem') }, events: { type: 'array', items: ref('TicketEventItem') } }, required: ['ticket_id', 'ticket_key', 'title', 'created_at', 'updated_at'] },
      AnomalyRuleItem: { type: 'object', properties: { anomaly_rule_id: str, name: str, kind: str, scope_type: str, scope_id: str, config_json: freeObject, is_active: bool, created_at: dt, updated_at: dt, created_by: str, updated_by: str }, required: ['anomaly_rule_id', 'name', 'kind', 'is_active', 'created_at', 'updated_at'] },
      AnomalyInstanceItem: { type: 'object', properties: { anomaly_instance_id: str, anomaly_rule_id: str, cluster_id: str, severity: str, status: str, started_at: dt, resolved_at: dt, payload_json: freeObject }, required: ['anomaly_instance_id', 'anomaly_rule_id'] },
      PolicyCreateRequest: { type: 'object', properties: { name: str, description: str, policy_body_json: freeObject }, required: ['name', 'policy_body_json'] },
      PolicyUpdateRequest: { type: 'object', properties: { description: str, policy_body_json: freeObject }, required: ['policy_body_json'] },
      HostUpsertRequest: { type: 'object', properties: { hostname: str, ip: str, ssh_port: u32, remote_user: str, labels: { type: 'object', additionalProperties: { type: 'string' } } }, required: ['hostname', 'ip', 'ssh_port', 'remote_user'] },
      HostGroupUpsertRequest: { type: 'object', properties: { name: str, description: str }, required: ['name'] },
      HostGroupMemberMutationRequest: { type: 'object', properties: { host_id: str, reason: str }, required: ['host_id'] },
      CredentialCreateRequest: { type: 'object', properties: { name: str, kind: str, description: str, vault_ref: str }, required: ['name', 'kind', 'vault_ref'] },
      DeploymentUpsertRequest: { type: 'object', properties: { job_type: str, policy_id: str, target_host_ids: { type: 'array', items: str }, target_host_group_ids: { type: 'array', items: str }, credential_profile_id: str, requested_by: str, preserve_state: bool, force: bool, dry_run: bool }, required: ['job_type', 'policy_id', 'credential_profile_id'] },
      DeploymentRetryRequest: { type: 'object', properties: { strategy: str, triggered_by: str, reason: str } },
      DeploymentCancelRequest: { type: 'object', properties: { requested_by: str, reason: str } },
      ClusterUpsertRequest: { type: 'object', properties: { name: str, slug: str, description: str, is_active: bool, metadata_json: freeObject, reason: str }, required: ['name', 'slug'] },
      ClusterHostMutationRequest: { type: 'object', properties: { host_id: str, reason: str }, required: ['host_id'] },
      RoleUpsertRequest: { type: 'object', properties: { name: str, slug: str, description: str, reason: str }, required: ['name', 'slug'] },
      RolePermissionsRequest: { type: 'object', properties: { permission_codes: { type: 'array', items: str }, reason: str }, required: ['permission_codes'] },
      RoleBindingCreateRequest: { type: 'object', properties: { user_id: str, role_id: str, scope_type: str, scope_id: str, reason: str }, required: ['user_id', 'role_id'] },
      IntegrationUpsertRequest: { type: 'object', properties: { name: str, kind: str, description: str, config_json: freeObject, is_active: bool, reason: str }, required: ['name', 'kind'] },
      IntegrationBindingRequest: { type: 'object', properties: { scope_type: str, scope_id: str, event_types_json: freeObject, severity_threshold: str, is_active: bool, reason: str } },
      TicketCreateRequest: { type: 'object', properties: { title: str, description: str, cluster_id: str, source_type: str, source_id: str, severity: str, reason: str }, required: ['title'] },
      TicketAssignRequest: { type: 'object', properties: { assignee_user_id: str, reason: str } },
      TicketCommentRequest: { type: 'object', properties: { body: str, reason: str }, required: ['body'] },
      TicketStatusRequest: { type: 'object', properties: { status: str, resolution: str, reason: str }, required: ['status'] },
      TicketCloseRequest: { type: 'object', properties: { resolution: str, reason: str } },
      AnomalyRuleUpsertRequest: { type: 'object', properties: { name: str, kind: str, scope_type: str, scope_id: str, config_json: freeObject, is_active: bool, reason: str }, required: ['name', 'kind'] },
      LogQueryFilter: { type: 'object', properties: { query: str, from: dt, to: dt, host: str, service: str, severity: str } },
      LogSearchRequest: { type: 'object', properties: { query: str, from: dt, to: dt, host: str, service: str, severity: str, limit: u32, offset: u64 } },
      LogContextRequest: { type: 'object', properties: { event_id: str, before: u32, after: u32 }, required: ['event_id'] },
      LogEventItem: { type: 'object', properties: { id: str, timestamp: dt, host: str, agent_id: str, source_type: str, source: str, service: str, severity: str, message: str, fingerprint: str, labels: { type: 'object', additionalProperties: { type: 'string' } }, fields_json: freeObject, raw: str }, required: ['id', 'timestamp', 'host', 'agent_id', 'source_type', 'source', 'service', 'severity', 'message', 'fingerprint', 'labels', 'fields_json', 'raw'] },
      LogSearchResponse: { type: 'object', properties: { items: { type: 'array', items: ref('LogEventItem') }, total: u64, limit: u32, offset: u64, took_ms: u32, request_id: str }, required: ['items', 'total', 'limit', 'offset', 'took_ms', 'request_id'] },
      LogEventResponse: { type: 'object', properties: { item: ref('LogEventItem'), took_ms: u32, request_id: str }, required: ['request_id'] },
      LogContextResponse: { type: 'object', properties: { anchor: ref('LogEventItem'), before: { type: 'array', items: ref('LogEventItem') }, after: { type: 'array', items: ref('LogEventItem') }, took_ms: u32, request_id: str }, required: ['before', 'after', 'took_ms', 'request_id'] },
      CountBucketItem: { type: 'object', properties: { key: str, count: u64 }, required: ['key', 'count'] },
      CountBucketsResponse: { type: 'object', properties: { items: { type: 'array', items: ref('CountBucketItem') }, request_id: str }, required: ['items', 'request_id'] },
      HistogramBucketItem: { type: 'object', properties: { bucket: str, count: u64 }, required: ['bucket', 'count'] },
      HistogramResponse: { type: 'object', properties: { items: { type: 'array', items: ref('HistogramBucketItem') }, request_id: str }, required: ['items', 'request_id'] },
      HeatmapBucketItem: { type: 'object', properties: { bucket: str, severity: str, count: u64 }, required: ['bucket', 'severity', 'count'] },
      HeatmapResponse: { type: 'object', properties: { items: { type: 'array', items: ref('HeatmapBucketItem') }, request_id: str }, required: ['items', 'request_id'] },
      PatternBucketItem: { type: 'object', properties: { fingerprint: str, sample_message: str, count: u64 }, required: ['fingerprint', 'sample_message', 'count'] },
      TopPatternsResponse: { type: 'object', properties: { items: { type: 'array', items: ref('PatternBucketItem') }, request_id: str }, required: ['items', 'request_id'] },
      LogAnomalyItem: { type: 'object', properties: { alert_instance_id: str, alert_rule_id: str, status: str, severity: str, title: str, fingerprint: str, host: str, service: str, triggered_at: dt, payload_json: freeObject }, required: ['alert_instance_id', 'alert_rule_id', 'status', 'severity', 'title', 'fingerprint', 'host', 'service', 'triggered_at', 'payload_json'] },
      LogAnomaliesResponse: { type: 'object', properties: { items: { type: 'array', items: ref('LogAnomalyItem') }, total: u64, limit: u32, offset: u64, request_id: str }, required: ['items', 'total', 'limit', 'offset', 'request_id'] },
      DashboardMetricItem: { type: 'object', properties: { key: str, label: str, value: str, change: { type: 'number' }, trend: str, description: str }, required: ['key', 'label', 'value', 'change', 'trend', 'description'] },
      DashboardActivityItem: { type: 'object', properties: { kind: str, title: str, description: str, timestamp: dt }, required: ['kind', 'title', 'description', 'timestamp'] },
      DashboardOverviewResponse: { type: 'object', properties: { metrics: { type: 'array', items: ref('DashboardMetricItem') }, active_hosts: u64, open_alerts: u64, deployment_jobs: u64, ingested_events: u64, recent_activity: { type: 'array', items: ref('DashboardActivityItem') }, log_histogram: { type: 'array', items: ref('HistogramBucketItem') }, top_services: { type: 'array', items: ref('CountBucketItem') }, top_hosts: { type: 'array', items: ref('CountBucketItem') }, request_id: str }, required: ['metrics', 'active_hosts', 'open_alerts', 'deployment_jobs', 'ingested_events', 'recent_activity', 'log_histogram', 'top_services', 'top_hosts', 'request_id'] },
      AlertRuleItem: { type: 'object', properties: { alert_rule_id: str, name: str, description: str, status: str, severity: str, scope_type: str, scope_id: str, condition_json: freeObject, created_at: dt, updated_at: dt, created_by: str, updated_by: str }, required: ['alert_rule_id', 'name', 'status', 'severity', 'scope_type', 'condition_json', 'created_at', 'updated_at', 'created_by', 'updated_by'] },
      AlertRuleMutationRequest: { type: 'object', properties: { name: str, description: str, status: str, severity: str, scope_type: str, scope_id: str, condition_json: freeObject, reason: str }, required: ['name', 'severity', 'scope_type', 'condition_json'] },
      AlertInstanceItem: { type: 'object', properties: { alert_instance_id: str, alert_rule_id: str, title: str, status: str, severity: str, triggered_at: dt, acknowledged_at: dt, resolved_at: dt, host: str, service: str, fingerprint: str, payload_json: freeObject }, required: ['alert_instance_id', 'alert_rule_id', 'title', 'status', 'severity', 'triggered_at', 'host', 'service', 'fingerprint', 'payload_json'] },
      AlertInstancesResponse: { type: 'object', properties: { items: { type: 'array', items: ref('AlertInstanceItem') }, paging: ref('PagingMeta'), request_id: str }, required: ['items', 'paging', 'request_id'] },
      AlertInstanceResponse: { type: 'object', properties: { item: ref('AlertInstanceItem'), request_id: str }, required: ['request_id'] },
      AlertRulesResponse: { type: 'object', properties: { items: { type: 'array', items: ref('AlertRuleItem') }, paging: ref('PagingMeta'), request_id: str }, required: ['items', 'paging', 'request_id'] },
      AlertRuleResponse: { type: 'object', properties: { item: ref('AlertRuleItem'), request_id: str }, required: ['request_id'] },
      PagingMeta: { type: 'object', properties: { limit: u32, offset: u64, total: u64 }, required: ['limit', 'offset', 'total'] },
      AuditEventItem: { type: 'object', properties: { audit_event_id: str, event_type: str, entity_type: str, entity_id: str, actor_id: str, actor_type: str, request_id: str, reason: str, payload_json: freeObject, created_at: dt }, required: ['audit_event_id', 'event_type', 'entity_type', 'entity_id', 'actor_id', 'actor_type', 'request_id', 'reason', 'payload_json', 'created_at'] },
      AuditEventsResponse: { type: 'object', properties: { items: { type: 'array', items: ref('AuditEventItem') }, paging: ref('PagingMeta'), request_id: str }, required: ['items', 'paging', 'request_id'] },
    },
  },
};

const paths = spec.paths;

const listOp = (tag, summary, schema, { paged = false, params = [], description } = {}) =>
  secure({
    tags: [tag],
    summary,
    ...(description ? { description } : {}),
    ...(params.length ? { parameters: params } : {}),
    responses: {
      200: response('OK.', listEnvelope(schema, paged), natsHeaders),
      ...publicErrors(400, 401, 403, 502, 503, 504),
    },
  });

const itemOp = (tag, summary, schema, { params = [], description } = {}) =>
  secure({
    tags: [tag],
    summary,
    ...(description ? { description } : {}),
    ...(params.length ? { parameters: params } : {}),
    responses: {
      200: response('OK.', itemEnvelope(schema), natsHeaders),
      ...publicErrors(400, 401, 403, 404, 502, 503, 504),
    },
  });

const writeOp = (tag, summary, requestSchema, responseSchema, status, { params = [], description } = {}) =>
  secure({
    tags: [tag],
    summary,
    ...(description ? { description } : {}),
    ...(params.length ? { parameters: params } : {}),
    requestBody: { required: true, content: jsonBody(requestSchema) },
    responses: {
      [status]: response(status === 201 ? 'Created.' : 'OK.', itemEnvelope(responseSchema), natsHeaders),
      ...publicErrors(400, 401, 403, 404, 502, 503, 504),
    },
  });

const successOp = (tag, summary, requestSchema, status = 200, params = []) =>
  secure({
    tags: [tag],
    summary,
    ...(params.length ? { parameters: params } : {}),
    ...(requestSchema ? { requestBody: { required: true, content: jsonBody(requestSchema) } } : {}),
    responses: {
      [status]: response('OK.', 'SuccessResponse', natsHeaders),
      ...publicErrors(400, 401, 403, 404, 502, 503, 504),
    },
  });

const placeholderOp = (tag, summary, description, params = []) =>
  secure({
    tags: [tag],
    summary,
    description,
    ...(params.length ? { parameters: params } : {}),
    responses: {
      501: {
        description: 'Boundary route is present, but the Rust runtime plane is not wired yet.',
        headers: runtimeHeaders,
        content: jsonBody('ErrorEnvelope'),
      },
      ...publicErrors(401),
    },
  });

paths['/'] = {
  get: {
    tags: ['system'],
    summary: 'Boundary discovery document',
    security: [],
    responses: {
      200: response('OK.', 'RootResponse', { 'X-Request-ID': headerRef('RequestId') }),
    },
  },
};
paths['/docs'] = {
  get: {
    tags: ['system'],
    summary: 'Redirect to embedded API explorer',
    security: [],
    responses: { 307: { description: 'Redirects to /docs/index.html.' } },
  },
};
paths['/openapi.json'] = {
  get: {
    tags: ['system'],
    summary: 'Serve OpenAPI JSON',
    security: [],
    responses: {
      200: {
        description: 'Current OpenAPI JSON document.',
        headers: { 'X-Request-ID': headerRef('RequestId') },
        content: { 'application/json': { schema: freeObject } },
      },
    },
  },
};
paths['/openapi.yaml'] = {
  get: {
    tags: ['system'],
    summary: 'Serve OpenAPI YAML',
    security: [],
    responses: {
      200: {
        description: 'Current OpenAPI YAML document.',
        headers: { 'X-Request-ID': headerRef('RequestId') },
        content: { 'application/yaml': { schema: str } },
      },
    },
  },
};
paths['/healthz'] = { get: { tags: ['system'], summary: 'Liveness probe', security: [], responses: { 200: response('OK.', 'StatusResponse', { 'X-Request-ID': headerRef('RequestId') }) } } };
paths['/readyz'] = { get: { tags: ['system'], summary: 'Readiness probe', security: [], responses: { 200: response('Ready.', 'StatusResponse', { 'X-Request-ID': headerRef('RequestId') }), 503: err('unavailable', 'Bridge is not ready.') } } };
paths['/version'] = { get: { tags: ['system'], summary: 'Service version', security: [], responses: { 200: response('OK.', 'VersionResponse', { 'X-Request-ID': headerRef('RequestId') }) } } };

paths['/auth/csrf'] = { get: { tags: ['auth'], summary: 'Issue CSRF token for compat auth flow', security: [], responses: { 200: response('CSRF token issued.', 'CsrfResponse', { 'X-Request-ID': headerRef('RequestId') }), 501: err('not_implemented', 'Compat auth stub is disabled.') } } };
paths['/auth/login'] = { post: { tags: ['auth'], summary: 'Compat login', security: [], requestBody: { required: true, content: jsonBody('CompatLoginRequest') }, responses: { 200: response('Logged in.', 'CompatSession', { 'X-Request-ID': headerRef('RequestId') }), ...publicErrors(400, 401, 403), 501: err('not_implemented', 'Compat auth stub is disabled.') } } };
paths['/auth/logout'] = { post: secure({ tags: ['auth'], summary: 'Compat logout', responses: { 200: response('Logged out.', 'SuccessResponse', { 'X-Request-ID': headerRef('RequestId') }), ...publicErrors(401, 403), 501: err('not_implemented', 'Compat auth stub is disabled.') } }) };
paths['/auth/me'] = { get: secure({ tags: ['auth'], summary: 'Current compat session', responses: { 200: response('Current session.', 'CompatSession', { 'X-Request-ID': headerRef('RequestId') }), ...publicErrors(401), 501: err('not_implemented', 'Compat auth stub is disabled.') } }) };
paths['/profile'] = { patch: secure({ tags: ['auth'], summary: 'Update compat profile', requestBody: { required: true, content: jsonBody('CompatProfileUpdateRequest') }, responses: { 200: response('Profile updated.', 'CompatSession', { 'X-Request-ID': headerRef('RequestId') }), ...publicErrors(400, 401, 403), 501: err('not_implemented', 'Compat auth stub is disabled.') } }) };

paths['/api/v1/me'] = { get: secure({ tags: ['auth'], summary: 'Boundary auth context', responses: { 200: response('Current auth context.', 'MeResponse', { 'X-Request-ID': headerRef('RequestId') }) } }) };
paths['/api/v1/auth/login'] = { post: paths['/auth/login'].post };
paths['/api/v1/auth/logout'] = { post: paths['/auth/logout'].post };
paths['/api/v1/auth/me'] = { get: paths['/auth/me'].get };

paths['/api/v1/agents'] = { get: listOp('agents', 'List agents', 'AgentItem') };
paths['/api/v1/agents/{id}'] = { get: itemOp('agents', 'Get agent', 'AgentItem', { params: [p('id', 'Agent ID.')] }) };
paths['/api/v1/agents/{id}/diagnostics'] = { get: listOp('agents', 'List diagnostics for an agent', 'AgentDiagnosticsItem', { params: [p('id', 'Agent ID.')] }) };
paths['/api/v1/agents/{id}/policy'] = { get: itemOp('agents', 'Get current policy bound to an agent', 'PolicyItem', { params: [p('id', 'Agent ID.')] }) };

paths['/api/v1/policies'] = { get: listOp('policies', 'List policies', 'PolicyItem'), post: writeOp('policies', 'Create policy', 'PolicyCreateRequest', 'PolicyItem', 201) };
paths['/api/v1/policies/{id}'] = { get: itemOp('policies', 'Get policy', 'PolicyItem', { params: [p('id', 'Policy ID.')] }), patch: writeOp('policies', 'Update policy', 'PolicyUpdateRequest', 'PolicyItem', 200, { params: [p('id', 'Policy ID.')] }) };
paths['/api/v1/policies/{id}/revisions'] = { get: secure({ tags: ['policies'], summary: 'List policy revisions', parameters: [p('id', 'Policy ID.')], responses: { 200: response('OK.', listEnvelope('PolicyRevisionItem'), natsHeaders), ...publicErrors(400, 401, 403, 404, 502, 503, 504) } }) };

paths['/api/v1/hosts'] = { get: listOp('inventory', 'List hosts', 'HostItem'), post: writeOp('inventory', 'Create host', 'HostUpsertRequest', 'HostItem', 201) };
paths['/api/v1/hosts/{id}'] = { get: itemOp('inventory', 'Get host', 'HostItem', { params: [p('id', 'Host ID.')] }), patch: writeOp('inventory', 'Update host', 'HostUpsertRequest', 'HostItem', 200, { params: [p('id', 'Host ID.')] }) };

paths['/api/v1/host-groups'] = { get: listOp('inventory', 'List host groups', 'HostGroupItem'), post: writeOp('inventory', 'Create host group', 'HostGroupUpsertRequest', 'HostGroupItem', 201) };
paths['/api/v1/host-groups/{id}'] = { get: itemOp('inventory', 'Get host group', 'HostGroupItem', { params: [p('id', 'Host group ID.')] }), patch: writeOp('inventory', 'Update host group', 'HostGroupUpsertRequest', 'HostGroupItem', 200, { params: [p('id', 'Host group ID.')] }) };
paths['/api/v1/host-groups/{id}/members'] = { post: writeOp('inventory', 'Add host group member', 'HostGroupMemberMutationRequest', 'HostGroupItem', 200, { params: [p('id', 'Host group ID.')] }) };
paths['/api/v1/host-groups/{id}/members/{hostId}'] = { delete: secure({ tags: ['inventory'], summary: 'Remove host group member', parameters: [p('id', 'Host group ID.'), p('hostId', 'Host ID.')], responses: { 200: response('OK.', itemEnvelope('HostGroupItem'), natsHeaders), ...publicErrors(400, 401, 403, 404, 502, 503, 504) } }) };

paths['/api/v1/credentials'] = { get: listOp('inventory', 'List credential profiles metadata', 'CredentialItem'), post: writeOp('inventory', 'Create credential profile metadata', 'CredentialCreateRequest', 'CredentialItem', 201) };
paths['/api/v1/credentials/{id}'] = { get: itemOp('inventory', 'Get credential profile metadata', 'CredentialItem', { params: [p('id', 'Credentials profile ID.')] }) };

paths['/api/v1/deployments'] = {
  get: secure({
    tags: ['deployments'],
    summary: 'List deployment jobs',
    parameters: [q('status', str, 'Deployment status filter.'), q('job_type', str, 'Deployment job type filter.'), q('requested_by', str, 'User or subject that requested the deployment.'), q('created_after', str, 'Filter jobs created at or after the given RFC3339 timestamp.'), q('created_before', str, 'Filter jobs created before the given RFC3339 timestamp.'), q('limit', u32, 'Maximum number of jobs to return.'), q('offset', u64, 'Pagination offset.')],
    responses: { 200: response('OK.', listEnvelope('DeploymentJobItem', true), natsHeaders), ...publicErrors(400, 401, 403, 502, 503, 504) },
  }),
  post: writeOp('deployments', 'Create deployment job', 'DeploymentUpsertRequest', 'DeploymentJobItem', 201),
};
paths['/api/v1/deployments/plan'] = { post: secure({ tags: ['deployments'], summary: 'Build deployment plan', requestBody: { required: true, content: jsonBody('DeploymentUpsertRequest') }, responses: { 200: response('Plan built.', itemEnvelope('DeploymentPlanItem'), natsHeaders), ...publicErrors(400, 401, 403, 502, 503, 504) } }) };
paths['/api/v1/deployments/{id}'] = { get: secure({ tags: ['deployments'], summary: 'Get deployment job with attempts, targets and steps', parameters: [p('id', 'Deployment job ID.')], responses: { 200: response('OK.', itemEnvelope('DeploymentJobItem', { attempts: { type: 'array', items: ref('DeploymentAttemptItem') }, targets: { type: 'array', items: ref('DeploymentTargetItem') }, steps: { type: 'array', items: ref('DeploymentStepItem') } }), natsHeaders), ...publicErrors(400, 401, 403, 404, 502, 503, 504) } }) };
paths['/api/v1/deployments/{id}/steps'] = { get: secure({ tags: ['deployments'], summary: 'List deployment steps', parameters: [p('id', 'Deployment job ID.')], responses: { 200: response('OK.', listEnvelope('DeploymentStepItem'), natsHeaders), ...publicErrors(400, 401, 403, 404, 502, 503, 504) } }) };
paths['/api/v1/deployments/{id}/targets'] = { get: secure({ tags: ['deployments'], summary: 'List deployment targets', parameters: [p('id', 'Deployment job ID.')], responses: { 200: response('OK.', listEnvelope('DeploymentTargetItem'), natsHeaders), ...publicErrors(400, 401, 403, 404, 502, 503, 504) } }) };
paths['/api/v1/deployments/{id}/retry'] = { post: writeOp('deployments', 'Retry deployment job', 'DeploymentRetryRequest', 'DeploymentJobItem', 200, { params: [p('id', 'Deployment job ID.')] }) };
paths['/api/v1/deployments/{id}/cancel'] = { post: writeOp('deployments', 'Cancel deployment job', 'DeploymentCancelRequest', 'DeploymentJobItem', 200, { params: [p('id', 'Deployment job ID.')] }) };

paths['/api/v1/clusters'] = { get: listOp('clusters', 'List clusters', 'ClusterItem', { paged: true, params: [q('limit', u32, 'Maximum number of clusters to return.'), q('offset', u64, 'Pagination offset.'), q('query', str, 'Free text search term.'), q('host_id', str, 'Filter clusters bound to a host.'), q('include_members', bool, 'Include host membership in list response.')] }), post: writeOp('clusters', 'Create cluster', 'ClusterUpsertRequest', 'ClusterItem', 201) };
paths['/api/v1/clusters/{id}'] = { get: itemOp('clusters', 'Get cluster', 'ClusterItem', { params: [p('id', 'Cluster ID.'), q('include_members', bool, 'Include host and agent bindings in the response.')] }), patch: writeOp('clusters', 'Update cluster', 'ClusterUpsertRequest', 'ClusterItem', 200, { params: [p('id', 'Cluster ID.')] }) };
paths['/api/v1/clusters/{id}/hosts'] = { post: writeOp('clusters', 'Bind host to cluster', 'ClusterHostMutationRequest', 'ClusterItem', 200, { params: [p('id', 'Cluster ID.')] }) };
paths['/api/v1/clusters/{id}/hosts/{hostId}'] = { delete: secure({ tags: ['clusters'], summary: 'Remove host from cluster', parameters: [p('id', 'Cluster ID.'), p('hostId', 'Host ID.')], responses: { 200: response('OK.', itemEnvelope('ClusterItem'), natsHeaders), ...publicErrors(400, 401, 403, 404, 502, 503, 504) } }) };

paths['/api/v1/roles'] = { get: listOp('roles', 'List roles', 'RoleItem', { paged: true, params: [q('limit', u32, 'Maximum number of roles.'), q('offset', u64, 'Pagination offset.'), q('query', str, 'Free text search term.')] }), post: writeOp('roles', 'Create role', 'RoleUpsertRequest', 'RoleItem', 201) };
paths['/api/v1/roles/{id}'] = { get: itemOp('roles', 'Get role', 'RoleItem', { params: [p('id', 'Role ID.')] }), patch: writeOp('roles', 'Update role', 'RoleUpsertRequest', 'RoleItem', 200, { params: [p('id', 'Role ID.')] }) };
paths['/api/v1/roles/{id}/permissions'] = { get: itemOp('roles', 'Get role permissions', 'RoleItem', { params: [p('id', 'Role ID.')] }), put: writeOp('roles', 'Replace role permissions', 'RolePermissionsRequest', 'RoleItem', 200, { params: [p('id', 'Role ID.')] }) };
paths['/api/v1/role-bindings'] = { get: listOp('roles', 'List role bindings', 'RoleBindingItem', { paged: true, params: [q('limit', u32, 'Maximum number of bindings.'), q('offset', u64, 'Pagination offset.'), q('query', str, 'Free text search term.'), q('user_id', str, 'Filter by user ID.'), q('role_id', str, 'Filter by role ID.'), q('scope_type', str, 'Filter by scope type.'), q('scope_id', str, 'Filter by scope ID.')] }), post: writeOp('roles', 'Create role binding', 'RoleBindingCreateRequest', 'RoleBindingItem', 201) };
paths['/api/v1/role-bindings/{id}'] = { delete: successOp('roles', 'Delete role binding', null, 200, [p('id', 'Role binding ID.')]) };

paths['/api/v1/integrations'] = { get: listOp('integrations', 'List integrations', 'IntegrationItem', { paged: true, params: [q('limit', u32, 'Maximum number of integrations.'), q('offset', u64, 'Pagination offset.'), q('query', str, 'Free text search term.')] }), post: writeOp('integrations', 'Create integration', 'IntegrationUpsertRequest', 'IntegrationItem', 201) };
paths['/api/v1/integrations/{id}'] = { get: itemOp('integrations', 'Get integration', 'IntegrationItem', { params: [p('id', 'Integration ID.')] }), patch: writeOp('integrations', 'Update integration', 'IntegrationUpsertRequest', 'IntegrationItem', 200, { params: [p('id', 'Integration ID.')] }) };
paths['/api/v1/integrations/{id}/bindings'] = { post: writeOp('integrations', 'Create integration binding', 'IntegrationBindingRequest', 'IntegrationBindingItem', 201, { params: [p('id', 'Integration ID.')] }) };
paths['/api/v1/integrations/{id}/bindings/{bindingId}'] = { delete: successOp('integrations', 'Delete integration binding', null, 200, [p('id', 'Integration ID.'), p('bindingId', 'Integration binding ID.')]) };

paths['/api/v1/tickets'] = { get: listOp('tickets', 'List tickets', 'TicketItem', { paged: true, params: [q('limit', u32, 'Maximum number of tickets.'), q('offset', u64, 'Pagination offset.'), q('query', str, 'Free text search term.'), q('cluster_id', str, 'Filter by cluster ID.'), q('status', str, 'Filter by ticket status.'), q('severity', str, 'Filter by severity.'), q('assignee_user_id', str, 'Filter by assignee user ID.')] }), post: writeOp('tickets', 'Create ticket', 'TicketCreateRequest', 'TicketItem', 201) };
paths['/api/v1/tickets/{id}'] = { get: itemOp('tickets', 'Get ticket', 'TicketItem', { params: [p('id', 'Ticket ID.')] }) };
paths['/api/v1/tickets/{id}/assign'] = { post: writeOp('tickets', 'Assign ticket', 'TicketAssignRequest', 'TicketItem', 200, { params: [p('id', 'Ticket ID.')] }) };
paths['/api/v1/tickets/{id}/unassign'] = { post: writeOp('tickets', 'Unassign ticket', 'TicketAssignRequest', 'TicketItem', 200, { params: [p('id', 'Ticket ID.')] }) };
paths['/api/v1/tickets/{id}/comments'] = { post: writeOp('tickets', 'Add ticket comment', 'TicketCommentRequest', 'TicketCommentItem', 201, { params: [p('id', 'Ticket ID.')] }) };
paths['/api/v1/tickets/{id}/status'] = { post: writeOp('tickets', 'Change ticket status', 'TicketStatusRequest', 'TicketItem', 200, { params: [p('id', 'Ticket ID.')] }) };
paths['/api/v1/tickets/{id}/close'] = { post: writeOp('tickets', 'Close ticket', 'TicketCloseRequest', 'TicketItem', 200, { params: [p('id', 'Ticket ID.')] }) };

paths['/api/v1/anomalies/rules'] = { get: listOp('anomalies', 'List anomaly rules', 'AnomalyRuleItem', { paged: true, params: [q('limit', u32, 'Maximum number of rules.'), q('offset', u64, 'Pagination offset.'), q('query', str, 'Free text search term.'), q('scope_type', str, 'Filter by scope type.'), q('scope_id', str, 'Filter by scope ID.')] }), post: writeOp('anomalies', 'Create anomaly rule', 'AnomalyRuleUpsertRequest', 'AnomalyRuleItem', 201) };
paths['/api/v1/anomalies/rules/{id}'] = { get: itemOp('anomalies', 'Get anomaly rule', 'AnomalyRuleItem', { params: [p('id', 'Anomaly rule ID.')] }), patch: writeOp('anomalies', 'Update anomaly rule', 'AnomalyRuleUpsertRequest', 'AnomalyRuleItem', 200, { params: [p('id', 'Anomaly rule ID.')] }) };
paths['/api/v1/anomalies/instances'] = { get: listOp('anomalies', 'List anomaly instances', 'AnomalyInstanceItem', { paged: true, params: [q('limit', u32, 'Maximum number of instances.'), q('offset', u64, 'Pagination offset.'), q('query', str, 'Free text search term.'), q('anomaly_rule_id', str, 'Filter by anomaly rule ID.'), q('cluster_id', str, 'Filter by cluster ID.'), q('status', str, 'Filter by status.')] }) };
paths['/api/v1/anomalies/instances/{id}'] = { get: itemOp('anomalies', 'Get anomaly instance', 'AnomalyInstanceItem', { params: [p('id', 'Anomaly instance ID.')] }) };

paths['/api/v1/logs/search'] = { post: secure({ tags: ['logs'], summary: 'Search logs', requestBody: { required: true, content: jsonBody('LogSearchRequest') }, responses: { 200: response('OK.', 'LogSearchResponse', natsHeaders), ...publicErrors(400, 401, 403, 502, 503, 504) } }) };
paths['/api/v1/logs/{eventId}'] = { get: secure({ tags: ['logs'], summary: 'Get one normalized log event', parameters: [p('eventId', 'Log event ID.')], responses: { 200: response('OK.', 'LogEventResponse', natsHeaders), ...publicErrors(400, 401, 403, 404, 502, 503, 504) } }) };
paths['/api/v1/logs/context'] = { post: secure({ tags: ['logs'], summary: 'Get log context around an event', requestBody: { required: true, content: jsonBody('LogContextRequest') }, responses: { 200: response('OK.', 'LogContextResponse', natsHeaders), ...publicErrors(400, 401, 403, 404, 502, 503, 504) } }) };
paths['/api/v1/logs/histogram'] = { get: secure({ tags: ['logs'], summary: 'Get histogram over log events', parameters: [q('query', str, 'Free-text query.'), q('from', dt, 'Inclusive RFC3339 timestamp.'), q('to', dt, 'Inclusive RFC3339 timestamp.'), q('host', str, 'Host filter.'), q('service', str, 'Service filter.'), q('severity', str, 'Severity filter.')], responses: { 200: response('OK.', 'HistogramResponse', natsHeaders), ...publicErrors(400, 401, 403, 502, 503, 504) } }) };
paths['/api/v1/logs/severity'] = { get: secure({ tags: ['logs'], summary: 'Get severity buckets', parameters: [q('query', str, 'Free-text query.'), q('from', dt, 'Inclusive RFC3339 timestamp.'), q('to', dt, 'Inclusive RFC3339 timestamp.'), q('host', str, 'Host filter.'), q('service', str, 'Service filter.'), q('severity', str, 'Severity filter.'), q('limit', u32, 'Maximum number of buckets.')], responses: { 200: response('OK.', 'CountBucketsResponse', natsHeaders), ...publicErrors(400, 401, 403, 502, 503, 504) } }) };
paths['/api/v1/logs/top-hosts'] = { get: secure({ tags: ['logs'], summary: 'Get top hosts by matching events', parameters: [q('query', str, 'Free-text query.'), q('from', dt, 'Inclusive RFC3339 timestamp.'), q('to', dt, 'Inclusive RFC3339 timestamp.'), q('host', str, 'Host filter.'), q('service', str, 'Service filter.'), q('severity', str, 'Severity filter.'), q('limit', u32, 'Maximum number of buckets.')], responses: { 200: response('OK.', 'CountBucketsResponse', natsHeaders), ...publicErrors(400, 401, 403, 502, 503, 504) } }) };
paths['/api/v1/logs/top-services'] = { get: secure({ tags: ['logs'], summary: 'Get top services by matching events', parameters: [q('query', str, 'Free-text query.'), q('from', dt, 'Inclusive RFC3339 timestamp.'), q('to', dt, 'Inclusive RFC3339 timestamp.'), q('host', str, 'Host filter.'), q('service', str, 'Service filter.'), q('severity', str, 'Severity filter.'), q('limit', u32, 'Maximum number of buckets.')], responses: { 200: response('OK.', 'CountBucketsResponse', natsHeaders), ...publicErrors(400, 401, 403, 502, 503, 504) } }) };
paths['/api/v1/logs/heatmap'] = { get: secure({ tags: ['logs'], summary: 'Get heatmap by time bucket and severity', parameters: [q('query', str, 'Free-text query.'), q('from', dt, 'Inclusive RFC3339 timestamp.'), q('to', dt, 'Inclusive RFC3339 timestamp.'), q('host', str, 'Host filter.'), q('service', str, 'Service filter.'), q('severity', str, 'Severity filter.')], responses: { 200: response('OK.', 'HeatmapResponse', natsHeaders), ...publicErrors(400, 401, 403, 502, 503, 504) } }) };
paths['/api/v1/logs/top-patterns'] = { get: secure({ tags: ['logs'], summary: 'Get top message patterns', parameters: [q('query', str, 'Free-text query.'), q('from', dt, 'Inclusive RFC3339 timestamp.'), q('to', dt, 'Inclusive RFC3339 timestamp.'), q('host', str, 'Host filter.'), q('service', str, 'Service filter.'), q('severity', str, 'Severity filter.'), q('limit', u32, 'Maximum number of patterns.')], responses: { 200: response('OK.', 'TopPatternsResponse', natsHeaders), ...publicErrors(400, 401, 403, 502, 503, 504) } }) };
paths['/api/v1/logs/anomalies'] = { get: secure({ tags: ['logs'], summary: 'List log-origin alert projections', parameters: [q('host', str, 'Host filter.'), q('service', str, 'Service filter.'), q('severity', str, 'Severity filter.'), q('limit', u32, 'Maximum number of items.'), q('offset', u64, 'Pagination offset.')], responses: { 200: response('OK.', 'LogAnomaliesResponse', natsHeaders), ...publicErrors(400, 401, 403, 502, 503, 504) } }) };
paths['/api/v1/dashboards/overview'] = { get: secure({ tags: ['dashboards'], summary: 'Get overview dashboard data', parameters: [q('from', dt, 'Inclusive RFC3339 timestamp.'), q('to', dt, 'Inclusive RFC3339 timestamp.')], responses: { 200: response('OK.', 'DashboardOverviewResponse', natsHeaders), ...publicErrors(400, 401, 403, 502, 503, 504) } }) };
paths['/api/v1/alerts'] = { get: secure({ tags: ['alerts'], summary: 'List alert instances', parameters: [q('status', str, 'Alert instance status filter.'), q('severity', str, 'Alert severity filter.'), q('host', str, 'Host filter.'), q('service', str, 'Service filter.'), q('limit', u32, 'Maximum number of items.'), q('offset', u64, 'Pagination offset.')], responses: { 200: response('OK.', 'AlertInstancesResponse', natsHeaders), ...publicErrors(400, 401, 403, 502, 503, 504) } }), post: secure({ tags: ['alerts'], summary: 'Create alert rule', requestBody: { required: true, content: jsonBody('AlertRuleMutationRequest') }, responses: { 200: response('OK.', 'AlertRuleResponse', natsHeaders), ...publicErrors(400, 401, 403, 502, 503, 504) } }) };
paths['/api/v1/alerts/rules'] = { get: secure({ tags: ['alerts'], summary: 'List alert rules', parameters: [q('query', str, 'Free-text query.'), q('status', str, 'Alert rule status filter.'), q('limit', u32, 'Maximum number of items.'), q('offset', u64, 'Pagination offset.')], responses: { 200: response('OK.', 'AlertRulesResponse', natsHeaders), ...publicErrors(400, 401, 403, 502, 503, 504) } }) };
paths['/api/v1/alerts/rules/{id}'] = { get: secure({ tags: ['alerts'], summary: 'Get alert rule', parameters: [p('id', 'Alert rule ID.')], responses: { 200: response('OK.', 'AlertRuleResponse', natsHeaders), ...publicErrors(400, 401, 403, 404, 502, 503, 504) } }) };
paths['/api/v1/alerts/{id}'] = { get: secure({ tags: ['alerts'], summary: 'Get alert instance', parameters: [p('id', 'Alert instance ID.')], responses: { 200: response('OK.', 'AlertInstanceResponse', natsHeaders), ...publicErrors(400, 401, 403, 404, 502, 503, 504) } }), patch: secure({ tags: ['alerts'], summary: 'Update alert rule', parameters: [p('id', 'Alert rule ID.')], requestBody: { required: true, content: jsonBody('AlertRuleMutationRequest') }, responses: { 200: response('OK.', 'AlertRuleResponse', natsHeaders), ...publicErrors(400, 401, 403, 404, 502, 503, 504) } }) };
paths['/api/v1/audit'] = { get: secure({ tags: ['audit'], summary: 'List cross-plane audit events', parameters: [q('event_type', str, 'Event type filter.'), q('entity_type', str, 'Entity type filter.'), q('entity_id', str, 'Entity ID filter.'), q('actor_id', str, 'Actor ID filter.'), q('limit', u32, 'Maximum number of items.'), q('offset', u64, 'Pagination offset.')], responses: { 200: response('OK.', 'AuditEventsResponse', natsHeaders), ...publicErrors(400, 401, 403, 502, 503, 504) } }) };

paths['/api/v1/stream/logs'] = { get: sse('stream', 'Logs SSE stream', 'Fan-out stream for ui.stream.logs subject.') };
paths['/api/v1/stream/deployments'] = { get: sse('stream', 'Deployments SSE stream', 'Fan-out stream for deployments.jobs.status and deployments.jobs.step subjects.') };
paths['/api/v1/stream/alerts'] = { get: sse('stream', 'Alerts SSE stream', 'Fan-out stream for ui.stream.alerts subject.') };
paths['/api/v1/stream/agents'] = { get: sse('stream', 'Agents SSE stream', 'Fan-out stream for ui.stream.agents subject.') };

function isScalar(value) {
  return value === null || ['string', 'number', 'boolean'].includes(typeof value);
}

function yamlScalar(value) {
  if (value === null) return 'null';
  if (typeof value === 'string') return JSON.stringify(value);
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  throw new Error(`unsupported scalar type: ${typeof value}`);
}

function toYAML(value, indent = 0) {
  const pad = '  '.repeat(indent);
  if (Array.isArray(value)) {
    if (!value.length) return `${pad}[]`;
    return value
      .map((item) => {
        if (isScalar(item)) return `${pad}- ${yamlScalar(item)}`;
        const rendered = toYAML(item, indent + 1).split('\n');
        return `${pad}- ${rendered[0].trimStart()}\n${rendered.slice(1).join('\n')}`;
      })
      .join('\n');
  }
  if (value && typeof value === 'object') {
    const entries = Object.entries(value);
    if (!entries.length) return `${pad}{}`;
    return entries
      .map(([key, item]) => {
        const qKey = JSON.stringify(key);
        if (isScalar(item)) return `${pad}${qKey}: ${yamlScalar(item)}`;
        return `${pad}${qKey}:\n${toYAML(item, indent + 1)}`;
      })
      .join('\n');
  }
  return `${pad}${yamlScalar(value)}`;
}

const jsonOutput = `${JSON.stringify(spec, null, 2)}\n`;
const yamlOutput = `${toYAML(spec)}\n`;
const jsonPath = path.join(docsDir, 'openapi.json');
const yamlPath = path.join(docsDir, 'openapi.yaml');

if (checkOnly) {
  const existingJson = fs.existsSync(jsonPath) ? fs.readFileSync(jsonPath, 'utf8') : '';
  const existingYaml = fs.existsSync(yamlPath) ? fs.readFileSync(yamlPath, 'utf8') : '';
  if (existingJson !== jsonOutput || existingYaml !== yamlOutput) {
    console.error('OpenAPI spec is out of date. Run `node edge_api/scripts/render-openapi.cjs`.');
    process.exit(1);
  }
  console.log('OpenAPI spec is up to date.');
} else {
  fs.mkdirSync(docsDir, { recursive: true });
  fs.writeFileSync(jsonPath, jsonOutput, 'utf8');
  fs.writeFileSync(yamlPath, yamlOutput, 'utf8');
  console.log('rendered edge_api/docs/openapi.json and edge_api/docs/openapi.yaml');
}
