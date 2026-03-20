export type BadgeTone =
  | "neutral"
  | "info"
  | "positive"
  | "warning"
  | "danger";

export type HostStatus = "online" | "offline" | "degraded" | "enrolling";

export type PolicyStatus = "draft" | "active" | "archived";

export type DeploymentStatus =
  | "pending"
  | "running"
  | "success"
  | "failed"
  | "canceled";

export type AlertStatus = "active" | "acknowledged" | "resolved" | "muted";

export type Severity = "debug" | "info" | "warning" | "error" | "critical";

export type HealthState = "healthy" | "warning" | "critical" | "unknown";

export type Host = {
  id: string;
  name: string;
  environment: "prod" | "staging" | "dev";
  region: string;
  cluster: string;
  provider: "aws" | "gcp" | "azure" | "on-prem";
  ipAddress: string;
  os: string;
  status: HostStatus;
  health: HealthState;
  cpuLoad: number;
  memoryUsage: number;
  policyCount: number;
  lastSeenAt: string;
  tags: string[];
};

export type Policy = {
  id: string;
  name: string;
  scope: string;
  status: PolicyStatus;
  updatedAt: string;
  owner: string;
};

export type DeploymentJob = {
  id: string;
  service: string;
  environment: Host["environment"];
  version: string;
  initiatedBy: string;
  status: DeploymentStatus;
  durationMinutes: number;
  startedAt: string;
  targetCount: number;
  completedCount: number;
};

export type Alert = {
  id: string;
  title: string;
  source: string;
  status: AlertStatus;
  severity: Severity;
  environment: Host["environment"];
  host: string;
  triggeredAt: string;
  assignee?: string;
  summary: string;
};

export type LogRecord = {
  id: string;
  timestamp: string;
  severity: Severity;
  service: string;
  host: string;
  message: string;
  traceId: string;
  fields: Record<string, string | number | boolean>;
};

export type AgentDiagnostic = {
  id: string;
  agentId: string;
  host: string;
  state: "connected" | "lagging" | "disconnected";
  version: string;
  lastCheckAt: string;
  notes: string;
};

export type DashboardMetric = {
  id: string;
  label: string;
  value: string;
  change: number;
  trend: "up" | "down" | "flat";
  description?: string;
};

export type StatusMeta<TValue extends string> = Record<
  TValue,
  {
    label: string;
    tone: BadgeTone;
  }
>;
