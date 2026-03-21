"use client";

import {
  listAgents,
  listAlerts,
  listAlertRules,
  listAudit,
  listDeployments,
  listLogAnomalies,
  listPolicies,
  type AgentItem,
  type AlertInstanceItem,
  type AlertRuleItem,
  type DeploymentDetailResponse,
  type DeploymentStepItem,
  type DeploymentTargetItem,
  type LogAnomalyItem,
  type PolicyItem,
} from "@/src/shared/lib/runtime-api";
import {
  loadTelegramInstances,
  projectTelegramDeliveries,
  type TelegramDeliveryProjection,
  type TelegramInstance,
} from "@/src/shared/lib/telegram-integrations-store";

export type SeverityTone = "default" | "success" | "warning" | "danger";

export type SecuritySummaryItem = {
  id: string;
  label: string;
  value: string;
  tone: SeverityTone;
  helperText: string;
};

export type SecurityFindingSeverity = "low" | "medium" | "high" | "critical";

export type SecurityFinding = {
  id: string;
  title: string;
  severity: SecurityFindingSeverity;
  status: "open" | "watching";
  summary: string;
  impact: string;
  evidence: string[];
  recommendedAction: string;
  relatedRoute?: {
    label: string;
    href: string;
  };
};

export type SecurityPostureData = {
  generatedAt: string;
  summary: SecuritySummaryItem[];
  findings: SecurityFinding[];
};

export type AnomalyMode = "light" | "medium" | "heavy";

export type AnomalyModeDefinition = {
  id: AnomalyMode;
  label: string;
  subtitle: string;
  description: string;
};

export type AnomalyRecord = {
  id: string;
  title: string;
  severity: string;
  status: string;
  triggeredAt: string;
  host: string;
  service: string;
  fingerprint: string;
  alertId: string;
  explanation: string;
  matchingAlerts: number;
};

export type AnomalyTimelineEntry = {
  id: string;
  kind: "anomaly" | "alert";
  title: string;
  timestamp: string;
  severity: string;
  detail: string;
  href?: string;
};

export type AnomalyWorkbenchData = {
  generatedAt: string;
  openAlerts: number;
  anomalies: AnomalyRecord[];
  timeline: AnomalyTimelineEntry[];
};

export type AlertSourceSignal = {
  label: string;
  value: string;
  detail?: string;
};

export type AlertDetailModel = {
  id: string;
  title: string;
  severity: string;
  status: string;
  triggeredAt: string;
  host: string;
  service: string;
  explanation: string;
  ruleName?: string;
  sourceSignals: AlertSourceSignal[];
  anomaly?: {
    id: string;
    title: string;
    triggeredAt: string;
    severity: string;
  };
  securityFinding?: SecurityFinding;
  clusterBindings: Array<{
    instanceName: string;
    cluster: string;
    routeLabel: string;
    chatId: string;
  }>;
  deliveryStatus: TelegramDeliveryProjection[];
  payload: Record<string, unknown>;
};

export type AlertsWorkbenchData = {
  generatedAt: string;
  alerts: AlertDetailModel[];
  rules: AlertRuleItem[];
  telegramInstances: TelegramInstance[];
};

export type DeploymentRolloutPhase = {
  key: "pull" | "start" | "health" | "rollback";
  label: string;
  status: string;
  detail: string;
};

export type DeploymentImageFlow = {
  installMode: string;
  rolloutState: string;
  imageLabel: string;
  imageSource: string;
  affectedTargets: number;
  succeededTargets: number;
  failedTargets: number;
  phases: DeploymentRolloutPhase[];
};

export const anomalyModeDefinitions: AnomalyModeDefinition[] = [
  {
    id: "light",
    label: "Light",
    subtitle: "Fast operator scan",
    description:
      "Shows the noisiest recent anomalies with a short correlation window. Good for triage and demos.",
  },
  {
    id: "medium",
    label: "Medium",
    subtitle: "Balanced signal density",
    description:
      "Balances coverage and readability by keeping the latest anomalies plus correlated open alerts.",
  },
  {
    id: "heavy",
    label: "Heavy",
    subtitle: "Deep correlation sweep",
    description:
      "Keeps a longer timeline and fuller alert context. This is still a frontend operator lens until backend-side anomaly mode contracts arrive.",
  },
];

function normalizeSeverity(value?: string): SecurityFindingSeverity {
  const normalized = value?.trim().toLowerCase() ?? "";
  if (normalized === "critical" || normalized === "fatal") {
    return "critical";
  }
  if (normalized === "error" || normalized === "high") {
    return "high";
  }
  if (normalized === "warning" || normalized === "warn" || normalized === "medium") {
    return "medium";
  }
  return "low";
}

export function getSeverityTone(value?: string): SeverityTone {
  const severity = normalizeSeverity(value);
  if (severity === "critical" || severity === "high") {
    return "danger";
  }
  if (severity === "medium") {
    return "warning";
  }
  return "default";
}

function formatCount(value: number) {
  return new Intl.NumberFormat().format(value);
}

function isOpenAlertStatus(value?: string) {
  const normalized = value?.trim().toLowerCase() ?? "";
  return !["resolved", "closed", "delivered"].includes(normalized);
}

function isHealthyAgent(agent: AgentItem) {
  const status = agent.status.trim().toLowerCase();
  return status === "online" || status === "healthy" || status === "ready";
}

function isFailingDeploymentStatus(value?: string) {
  const normalized = value?.trim().toLowerCase() ?? "";
  return ["failed", "error", "rollback", "cancelled"].includes(normalized);
}

function buildSecurityFindings(input: {
  agents: AgentItem[];
  alerts: AlertInstanceItem[];
  policies: PolicyItem[];
  deploymentStates: Array<{ status?: string }>;
  recentAuditCount: number;
}): SecurityFinding[] {
  const findings: SecurityFinding[] = [];

  const degradedAgents = input.agents.filter((item) => !isHealthyAgent(item));
  if (degradedAgents.length > 0) {
    findings.push({
      id: "agents-coverage",
      title: "Agent coverage requires attention",
      severity: degradedAgents.length > 2 ? "critical" : "high",
      status: "open",
      summary: `${formatCount(degradedAgents.length)} agent(s) are not healthy or online.`,
      impact:
        "Alerting and posture calculations may miss signals from affected hosts while these agents remain degraded.",
      evidence: degradedAgents
        .slice(0, 4)
        .map((item) => `${item.hostname} (${item.status})`),
      recommendedAction:
        "Inspect affected agents, confirm last-seen timestamps, and restore telemetry before broad rollouts.",
      relatedRoute: {
        label: "Open agents",
        href: "/agents",
      },
    });
  }

  const openAlerts = input.alerts.filter((item) => isOpenAlertStatus(item.status));
  if (openAlerts.length > 0) {
    findings.push({
      id: "open-alerts",
      title: "Open operator alerts are accumulating",
      severity: openAlerts.length >= 5 ? "critical" : "high",
      status: "open",
      summary: `${formatCount(openAlerts.length)} alert(s) still require operator follow-up.`,
      impact:
        "The console already shows active alert pressure, which increases the chance of delayed acknowledgements.",
      evidence: openAlerts
        .slice(0, 4)
        .map((item) => `${item.title} on ${item.host || item.service || "unscoped target"}`),
      recommendedAction:
        "Review correlated anomalies and delivery routing, then acknowledge or resolve stale alerts.",
      relatedRoute: {
        label: "Open alerts",
        href: "/alerts",
      },
    });
  }

  const inactivePolicies = input.policies.filter((item) => !item.is_active);
  if (inactivePolicies.length > 0) {
    findings.push({
      id: "inactive-policies",
      title: "Inactive policies detected",
      severity: "medium",
      status: "watching",
      summary: `${formatCount(inactivePolicies.length)} policy profile(s) are inactive.`,
      impact:
        "Inactive policy revisions reduce coverage for drift detection and deployment guardrails.",
      evidence: inactivePolicies.slice(0, 4).map((item) => item.name),
      recommendedAction:
        "Confirm whether inactive policies are intentional, and reactivate or archive the stale entries.",
      relatedRoute: {
        label: "Open policies",
        href: "/policies",
      },
    });
  }

  const failingDeployments = input.deploymentStates.filter((item) =>
    isFailingDeploymentStatus(item.status)
  );
  if (failingDeployments.length > 0) {
    findings.push({
      id: "deployment-instability",
      title: "Recent rollout instability",
      severity: "high",
      status: "open",
      summary: `${formatCount(failingDeployments.length)} deployment job(s) show failed or rollback states.`,
      impact:
        "Rollback-heavy deployment activity often precedes noisy alerts and degraded host trust.",
      evidence: failingDeployments
        .slice(0, 4)
        .map((item, index) => `Deployment ${index + 1}: ${item.status ?? "unknown"}`),
      recommendedAction:
        "Inspect rollout phases, confirm image health, and avoid widening the blast radius until the failing jobs stabilize.",
      relatedRoute: {
        label: "Open deployments",
        href: "/deployments",
      },
    });
  }

  if (input.recentAuditCount >= 8) {
    findings.push({
      id: "audit-burst",
      title: "High control-plane change activity",
      severity: "medium",
      status: "watching",
      summary: `${formatCount(input.recentAuditCount)} recent audit events were observed in the runtime log.`,
      impact:
        "Dense change windows make root-cause analysis harder when alerts and anomalies begin stacking.",
      evidence: ["Recent audit volume exceeded the dashboard watch threshold."],
      recommendedAction:
        "Confirm whether the current change window is expected and coordinate alert ownership before additional mutations.",
      relatedRoute: {
        label: "Open audit",
        href: "/audit",
      },
    });
  }

  return findings.sort((left, right) => {
    const order: Record<SecurityFindingSeverity, number> = {
      critical: 4,
      high: 3,
      medium: 2,
      low: 1,
    };
    return order[right.severity] - order[left.severity];
  });
}

export async function getSecurityPostureData(): Promise<SecurityPostureData> {
  const [agentsResponse, alertsResponse, policiesResponse, deploymentsResponse, auditResponse] =
    await Promise.all([
      listAgents(),
      listAlerts({ limit: 30, offset: 0 }),
      listPolicies(),
      listDeployments(),
      listAudit({ limit: 20, offset: 0 }),
    ]);

  const openAlerts = alertsResponse.items.filter((item) => isOpenAlertStatus(item.status));
  const unhealthyAgents = agentsResponse.items.filter((item) => !isHealthyAgent(item));
  const activePolicies = policiesResponse.items.filter((item) => item.is_active);
  const failingDeployments = deploymentsResponse.items.filter((item) =>
    isFailingDeploymentStatus(item.status)
  );

  return {
    generatedAt: new Date().toISOString(),
    summary: [
      {
        id: "open-alerts",
        label: "Open alerts",
        value: formatCount(openAlerts.length),
        tone: openAlerts.length > 0 ? "danger" : "success",
        helperText: "Current runtime alert pressure derived from alert instances.",
      },
      {
        id: "healthy-agents",
        label: "Healthy agents",
        value: `${formatCount(agentsResponse.items.length - unhealthyAgents.length)}/${formatCount(
          agentsResponse.items.length
        )}`,
        tone: unhealthyAgents.length > 0 ? "warning" : "success",
        helperText: "Coverage ratio based on the agent registry health states.",
      },
      {
        id: "active-policies",
        label: "Active policies",
        value: `${formatCount(activePolicies.length)}/${formatCount(
          policiesResponse.items.length
        )}`,
        tone: activePolicies.length === policiesResponse.items.length ? "success" : "warning",
        helperText: "Policy posture is inferred from active policy metadata in the runtime API.",
      },
      {
        id: "rollout-risk",
        label: "Failing rollouts",
        value: formatCount(failingDeployments.length),
        tone: failingDeployments.length > 0 ? "danger" : "success",
        helperText: "Rollback and failed deployment jobs that may amplify operator load.",
      },
    ],
    findings: buildSecurityFindings({
      agents: agentsResponse.items,
      alerts: alertsResponse.items,
      policies: policiesResponse.items,
      deploymentStates: deploymentsResponse.items,
      recentAuditCount: auditResponse.items.length,
    }),
  };
}

export async function getAnomalyWorkbenchData(
  mode: AnomalyMode
): Promise<AnomalyWorkbenchData> {
  const limit = mode === "light" ? 6 : mode === "medium" ? 10 : 16;
  const [anomaliesResponse, alertsResponse] = await Promise.all([
    listLogAnomalies({ limit, offset: 0 }),
    listAlerts({ limit: 20, offset: 0 }),
  ]);

  const openAlerts = alertsResponse.items.filter((item) => isOpenAlertStatus(item.status));

  const anomalies = anomaliesResponse.items.map((item) => {
    const matchingAlerts = openAlerts.filter(
      (alert) =>
        alert.alert_instance_id === item.alert_instance_id ||
        alert.fingerprint === item.fingerprint ||
        (alert.host === item.host && alert.service === item.service)
    );

    return {
      id: item.alert_instance_id,
      title: item.title,
      severity: item.severity,
      status: item.status,
      triggeredAt: item.triggered_at,
      host: item.host,
      service: item.service,
      fingerprint: item.fingerprint,
      alertId: item.alert_instance_id,
      matchingAlerts: matchingAlerts.length,
      explanation:
        mode === "light"
          ? "Fast triage mode keeps the immediate alert correlation only."
          : mode === "medium"
            ? "Balanced mode keeps the anomaly plus nearby open alerts for operator review."
            : "Heavy mode keeps a wider operator correlation window. Backend-side anomaly mode control is still pending, so this is a frontend lens.",
    } satisfies AnomalyRecord;
  });

  const timeline: AnomalyTimelineEntry[] = [
    ...anomalies.map((item) => ({
      id: `anomaly-${item.id}`,
      kind: "anomaly" as const,
      title: item.title,
      timestamp: item.triggeredAt,
      severity: item.severity,
      detail: `${item.host || "unknown host"} / ${item.service || "unknown service"} with ${item.matchingAlerts} correlated open alert(s).`,
      href: `/alerts?alert=${item.alertId}`,
    })),
    ...openAlerts.slice(0, mode === "heavy" ? 10 : 6).map((item) => ({
      id: `alert-${item.alert_instance_id}`,
      kind: "alert" as const,
      title: item.title,
      timestamp: item.triggered_at,
      severity: item.severity,
      detail: `${item.host || "unknown host"} / ${item.service || "unknown service"} is still open.`,
      href: `/alerts?alert=${item.alert_instance_id}`,
    })),
  ].sort((left, right) => {
    return new Date(right.timestamp).getTime() - new Date(left.timestamp).getTime();
  });

  return {
    generatedAt: new Date().toISOString(),
    openAlerts: openAlerts.length,
    anomalies,
    timeline,
  };
}

function selectSecurityContext(
  alert: AlertInstanceItem,
  findings: SecurityFinding[],
  agents: AgentItem[]
) {
  const degradedHostAgent = agents.find(
    (agent) => agent.hostname === alert.host && !isHealthyAgent(agent)
  );

  if (degradedHostAgent) {
    return findings.find((item) => item.id === "agents-coverage");
  }

  return (
    findings.find((item) => item.id === "open-alerts") ??
    findings.find((item) => item.id === "deployment-instability") ??
    findings.find((item) => item.id === "inactive-policies")
  );
}

function buildAlertExplanation(
  alert: AlertInstanceItem,
  rule: AlertRuleItem | undefined,
  anomaly: LogAnomalyItem | undefined,
  deliveries: TelegramDeliveryProjection[]
) {
  if (anomaly) {
    return `Alert ${alert.title} is backed by a log anomaly on ${anomaly.host || "an unknown host"} and currently routes through ${deliveries.length} delivery path(s).`;
  }

  return `Alert ${alert.title} is active under ${rule?.name ?? "an unmapped rule"} and is using frontend delivery projections until a dedicated delivery-status backend contract is published.`;
}

export async function getAlertsWorkbenchData(): Promise<AlertsWorkbenchData> {
  const [alertsResponse, rulesResponse, anomaliesResponse, agentsResponse, securityData] =
    await Promise.all([
      listAlerts({ limit: 30, offset: 0 }),
      listAlertRules({ limit: 30, offset: 0 }),
      listLogAnomalies({ limit: 30, offset: 0 }),
      listAgents(),
      getSecurityPostureData(),
    ]);

  const telegramInstances = loadTelegramInstances();

  const alerts = alertsResponse.items.map((alert) => {
    const rule = rulesResponse.items.find((item) => item.alert_rule_id === alert.alert_rule_id);
    const anomaly =
      anomaliesResponse.items.find(
        (item) =>
          item.alert_instance_id === alert.alert_instance_id ||
          item.fingerprint === alert.fingerprint
      ) ?? undefined;
    const deliveryStatus = projectTelegramDeliveries(telegramInstances, {
      host: alert.host,
      service: alert.service,
      severity: alert.severity,
      status: alert.status,
    });
    const securityFinding = selectSecurityContext(
      alert,
      securityData.findings,
      agentsResponse.items
    );

    return {
      id: alert.alert_instance_id,
      title: alert.title,
      severity: alert.severity,
      status: alert.status,
      triggeredAt: alert.triggered_at,
      host: alert.host,
      service: alert.service,
      ruleName: rule?.name,
      explanation: buildAlertExplanation(alert, rule, anomaly, deliveryStatus),
      sourceSignals: [
        { label: "Rule", value: rule?.name ?? alert.alert_rule_id },
        { label: "Severity", value: alert.severity },
        { label: "Status", value: alert.status },
        { label: "Host", value: alert.host || "n/a" },
        { label: "Service", value: alert.service || "n/a" },
        { label: "Fingerprint", value: alert.fingerprint || "n/a" },
      ],
      anomaly: anomaly
        ? {
            id: anomaly.alert_instance_id,
            title: anomaly.title,
            triggeredAt: anomaly.triggered_at,
            severity: anomaly.severity,
          }
        : undefined,
      securityFinding,
      clusterBindings: deliveryStatus.map((item) => ({
        instanceName: item.instanceName,
        cluster: item.cluster,
        routeLabel: item.routeLabel,
        chatId: item.chatId,
      })),
      deliveryStatus,
      payload: alert.payload_json,
    } satisfies AlertDetailModel;
  });

  return {
    generatedAt: new Date().toISOString(),
    alerts,
    rules: rulesResponse.items,
    telegramInstances,
  };
}

function findStep(
  steps: DeploymentStepItem[],
  tokens: string[]
): DeploymentStepItem | undefined {
  return steps.find((step) => {
    const normalized = step.step_name.trim().toLowerCase();
    return tokens.some((token) => normalized.includes(token));
  });
}

function stepStatusOrFallback(
  step: DeploymentStepItem | undefined,
  detail: DeploymentDetailResponse,
  kind: DeploymentRolloutPhase["key"]
) {
  if (step) {
    return step.status;
  }

  if (kind === "rollback") {
    return detail.item.failed_targets > 0 ? "ready" : "idle";
  }

  if (detail.item.succeeded_targets > 0 && detail.item.failed_targets === 0) {
    return "success";
  }

  if (detail.item.running_targets > 0) {
    return "running";
  }

  if (detail.item.pending_targets > 0) {
    return "pending";
  }

  if (detail.item.failed_targets > 0) {
    return "failed";
  }

  return "unknown";
}

function summarizeImage(targets: DeploymentTargetItem[]) {
  const artifact = targets.find((target) => target.artifact)?.artifact;
  if (!artifact) {
    return {
      imageLabel: "No artifact metadata yet",
      imageSource: "The runtime API has not returned a deployment artifact for the selected job.",
    };
  }

  return {
    imageLabel: `${artifact.artifact_name || "image"}:${artifact.version}`,
    imageSource: artifact.source_uri || "No source URI returned",
  };
}

export function deriveDeploymentImageFlow(
  detail: DeploymentDetailResponse
): DeploymentImageFlow {
  const pullStep = findStep(detail.steps, ["pull", "fetch", "download"]);
  const startStep = findStep(detail.steps, ["start", "launch", "run"]);
  const healthStep = findStep(detail.steps, ["health", "probe", "ready"]);
  const rollbackStep = findStep(detail.steps, ["rollback"]);
  const image = summarizeImage(detail.targets);

  return {
    installMode:
      detail.item.total_targets > 1 ? "Rolling image install" : "Single-target image install",
    rolloutState: detail.item.current_phase || detail.item.status || "unknown",
    imageLabel: image.imageLabel,
    imageSource: image.imageSource,
    affectedTargets: detail.item.total_targets,
    succeededTargets: detail.item.succeeded_targets,
    failedTargets: detail.item.failed_targets,
    phases: [
      {
        key: "pull",
        label: "Pull",
        status: stepStatusOrFallback(pullStep, detail, "pull"),
        detail:
          pullStep?.message ||
          "Pulls the deployment image or package onto the target host.",
      },
      {
        key: "start",
        label: "Start",
        status: stepStatusOrFallback(startStep, detail, "start"),
        detail:
          startStep?.message ||
          "Starts the workload or service with the requested image revision.",
      },
      {
        key: "health",
        label: "Health",
        status: stepStatusOrFallback(healthStep, detail, "health"),
        detail:
          healthStep?.message ||
          "Verifies probes and post-start runtime health before promotion.",
      },
      {
        key: "rollback",
        label: "Rollback",
        status: stepStatusOrFallback(rollbackStep, detail, "rollback"),
        detail:
          rollbackStep?.message ||
          "Prepared rollback state when any target fails health or startup.",
      },
    ],
  };
}
