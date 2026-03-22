"use client";

import type { Locale } from "@/src/shared/config";
import { getSiteCopy, translateValueLabel } from "@/src/shared/lib/i18n";
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

function formatCount(value: number, locale: Locale) {
  return new Intl.NumberFormat(locale).format(value);
}

export function getAnomalyModeDefinitions(locale: Locale): AnomalyModeDefinition[] {
  const copy = getSiteCopy(locale).workbench.anomalies.modes;

  return [
    {
      id: "light",
      label: copy.light.label,
      subtitle: copy.light.subtitle,
      description: copy.light.description,
    },
    {
      id: "medium",
      label: copy.medium.label,
      subtitle: copy.medium.subtitle,
      description: copy.medium.description,
    },
    {
      id: "heavy",
      label: copy.heavy.label,
      subtitle: copy.heavy.subtitle,
      description: copy.heavy.description,
    },
  ];
}

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
  locale: Locale;
  agents: AgentItem[];
  alerts: AlertInstanceItem[];
  policies: PolicyItem[];
  deploymentStates: Array<{ status?: string }>;
  recentAuditCount: number;
}): SecurityFinding[] {
  const { locale } = input;
  const copy = getSiteCopy(locale).workbench.security.findings;
  const findings: SecurityFinding[] = [];

  const degradedAgents = input.agents.filter((item) => !isHealthyAgent(item));
  if (degradedAgents.length > 0) {
    findings.push({
      id: "agents-coverage",
      title: copy.agentsCoverage.title,
      severity: degradedAgents.length > 2 ? "critical" : "high",
      status: "open",
      summary: copy.agentsCoverage.summary(formatCount(degradedAgents.length, locale)),
      impact: copy.agentsCoverage.impact,
      evidence: degradedAgents
        .slice(0, 4)
        .map(
          (item) =>
            `${item.hostname} (${translateValueLabel(item.status, locale)})`
        ),
      recommendedAction: copy.agentsCoverage.recommendedAction,
      relatedRoute: {
        label: copy.agentsCoverage.relatedRouteLabel,
        href: "/infrastructure?tab=agents",
      },
    });
  }

  const openAlerts = input.alerts.filter((item) => isOpenAlertStatus(item.status));
  if (openAlerts.length > 0) {
    findings.push({
      id: "open-alerts",
      title: copy.openAlerts.title,
      severity: openAlerts.length >= 5 ? "critical" : "high",
      status: "open",
      summary: copy.openAlerts.summary(formatCount(openAlerts.length, locale)),
      impact: copy.openAlerts.impact,
      evidence: openAlerts
        .slice(0, 4)
        .map(
          (item) =>
            `${item.title} on ${
              item.host || item.service || copy.unscopedTarget
            }`
        ),
      recommendedAction: copy.openAlerts.recommendedAction,
      relatedRoute: {
        label: copy.openAlerts.relatedRouteLabel,
        href: "/security?tab=alerts",
      },
    });
  }

  const inactivePolicies = input.policies.filter((item) => !item.is_active);
  if (inactivePolicies.length > 0) {
    findings.push({
      id: "inactive-policies",
      title: copy.inactivePolicies.title,
      severity: "medium",
      status: "watching",
      summary: copy.inactivePolicies.summary(
        formatCount(inactivePolicies.length, locale)
      ),
      impact: copy.inactivePolicies.impact,
      evidence: inactivePolicies.slice(0, 4).map((item) => item.name),
      recommendedAction: copy.inactivePolicies.recommendedAction,
      relatedRoute: {
        label: copy.inactivePolicies.relatedRouteLabel,
        href: "/security?tab=policies",
      },
    });
  }

  const failingDeployments = input.deploymentStates.filter((item) =>
    isFailingDeploymentStatus(item.status)
  );
  if (failingDeployments.length > 0) {
    findings.push({
      id: "deployment-instability",
      title: copy.deploymentInstability.title,
      severity: "high",
      status: "open",
      summary: copy.deploymentInstability.summary(
        formatCount(failingDeployments.length, locale)
      ),
      impact: copy.deploymentInstability.impact,
      evidence: failingDeployments
        .slice(0, 4)
        .map(
          (item, index) =>
            `${copy.deploymentPrefix} ${index + 1}: ${
              translateValueLabel(item.status ?? copy.unknownStatus, locale)
            }`
        ),
      recommendedAction: copy.deploymentInstability.recommendedAction,
      relatedRoute: {
        label: copy.deploymentInstability.relatedRouteLabel,
        href: "/operations?tab=deployments",
      },
    });
  }

  if (input.recentAuditCount >= 8) {
    findings.push({
      id: "audit-burst",
      title: copy.auditBurst.title,
      severity: "medium",
      status: "watching",
      summary: copy.auditBurst.summary(formatCount(input.recentAuditCount, locale)),
      impact: copy.auditBurst.impact,
      evidence: [copy.auditBurst.evidence],
      recommendedAction: copy.auditBurst.recommendedAction,
      relatedRoute: {
        label: copy.auditBurst.relatedRouteLabel,
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

export async function getSecurityPostureData(
  locale: Locale
): Promise<SecurityPostureData> {
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
  const summaryCopy = getSiteCopy(locale).workbench.security.summary;

  return {
    generatedAt: new Date().toISOString(),
    summary: [
      {
        id: "open-alerts",
        label: summaryCopy.openAlerts.label,
        value: formatCount(openAlerts.length, locale),
        tone: openAlerts.length > 0 ? "danger" : "success",
        helperText: summaryCopy.openAlerts.helperText,
      },
      {
        id: "healthy-agents",
        label: summaryCopy.healthyAgents.label,
        value: `${formatCount(
          agentsResponse.items.length - unhealthyAgents.length,
          locale
        )}/${formatCount(
          agentsResponse.items.length,
          locale
        )}`,
        tone: unhealthyAgents.length > 0 ? "warning" : "success",
        helperText: summaryCopy.healthyAgents.helperText,
      },
      {
        id: "active-policies",
        label: summaryCopy.activePolicies.label,
        value: `${formatCount(activePolicies.length, locale)}/${formatCount(
          policiesResponse.items.length,
          locale
        )}`,
        tone: activePolicies.length === policiesResponse.items.length ? "success" : "warning",
        helperText: summaryCopy.activePolicies.helperText,
      },
      {
        id: "rollout-risk",
        label: summaryCopy.rolloutRisk.label,
        value: formatCount(failingDeployments.length, locale),
        tone: failingDeployments.length > 0 ? "danger" : "success",
        helperText: summaryCopy.rolloutRisk.helperText,
      },
    ],
    findings: buildSecurityFindings({
      locale,
      agents: agentsResponse.items,
      alerts: alertsResponse.items,
      policies: policiesResponse.items,
      deploymentStates: deploymentsResponse.items,
      recentAuditCount: auditResponse.items.length,
    }),
  };
}

export async function getAnomalyWorkbenchData(
  mode: AnomalyMode,
  locale: Locale
): Promise<AnomalyWorkbenchData> {
  const copy = getSiteCopy(locale).workbench.anomalies;
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
          ? copy.modes.light.explanation
          : mode === "medium"
            ? copy.modes.medium.explanation
            : copy.modes.heavy.explanation,
    } satisfies AnomalyRecord;
  });

  const timeline: AnomalyTimelineEntry[] = [
    ...anomalies.map((item) => ({
      id: `anomaly-${item.id}`,
      kind: "anomaly" as const,
      title: item.title,
      timestamp: item.triggeredAt,
      severity: item.severity,
      detail: `${item.host || copy.unknownHost} / ${
        item.service || copy.unknownService
      } ${copy.correlatedAlertsSuffix(item.matchingAlerts)}`,
      href: `/security?tab=alerts&alert=${item.alertId}`,
    })),
    ...openAlerts.slice(0, mode === "heavy" ? 10 : 6).map((item) => ({
      id: `alert-${item.alert_instance_id}`,
      kind: "alert" as const,
      title: item.title,
      timestamp: item.triggered_at,
      severity: item.severity,
      detail: `${item.host || copy.unknownHost} / ${
        item.service || copy.unknownService
      } ${copy.alertStillOpenSuffix}`,
      href: `/security?tab=alerts&alert=${item.alert_instance_id}`,
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
  deliveries: TelegramDeliveryProjection[],
  locale: Locale
) {
  const copy = getSiteCopy(locale).workbench.alerts;
  if (anomaly) {
    return copy.explanationWithAnomaly(
      alert.title,
      anomaly.host || getSiteCopy(locale).workbench.anomalies.unknownHost,
      deliveries.length
    );
  }

  return copy.explanationWithoutAnomaly(
    alert.title,
    rule?.name ?? (locale === "ru" ? "несопоставленное правило" : "an unmapped rule")
  );
}

export async function getAlertsWorkbenchData(
  locale: Locale
): Promise<AlertsWorkbenchData> {
  const [alertsResponse, rulesResponse, anomaliesResponse, agentsResponse, securityData] =
    await Promise.all([
      listAlerts({ limit: 30, offset: 0 }),
      listAlertRules({ limit: 30, offset: 0 }),
      listLogAnomalies({ limit: 30, offset: 0 }),
      listAgents(),
      getSecurityPostureData(locale),
    ]);

  const telegramInstances = loadTelegramInstances();
  const copy = getSiteCopy(locale).workbench.alerts;
  const commonCopy = getSiteCopy(locale).common;

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
      explanation: buildAlertExplanation(
        alert,
        rule,
        anomaly,
        deliveryStatus,
        locale
      ),
      sourceSignals: [
        { label: copy.rule, value: rule?.name ?? alert.alert_rule_id },
        {
          label: copy.severity,
          value: translateValueLabel(alert.severity, locale),
        },
        { label: copy.status, value: translateValueLabel(alert.status, locale) },
        { label: copy.host, value: alert.host || commonCopy.na },
        { label: copy.service, value: alert.service || commonCopy.na },
        { label: copy.fingerprint, value: alert.fingerprint || commonCopy.na },
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

function summarizeImage(targets: DeploymentTargetItem[], locale: Locale) {
  const copy = getSiteCopy(locale).workbench.deploymentImage;
  const artifact = targets.find((target) => target.artifact)?.artifact;
  if (!artifact) {
    return {
      imageLabel: copy.noArtifactLabel,
      imageSource: copy.noArtifactDescription,
    };
  }

  return {
    imageLabel: `${artifact.artifact_name || copy.imageFallbackName}:${artifact.version}`,
    imageSource: artifact.source_uri || copy.noSourceUri,
  };
}

export function deriveDeploymentImageFlow(
  detail: DeploymentDetailResponse,
  locale: Locale
): DeploymentImageFlow {
  const copy = getSiteCopy(locale).workbench.deploymentImage;
  const pullStep = findStep(detail.steps, ["pull", "fetch", "download"]);
  const startStep = findStep(detail.steps, ["start", "launch", "run"]);
  const healthStep = findStep(detail.steps, ["health", "probe", "ready"]);
  const rollbackStep = findStep(detail.steps, ["rollback"]);
  const image = summarizeImage(detail.targets, locale);

  return {
    installMode:
      detail.item.total_targets > 1 ? copy.rollingInstall : copy.singleInstall,
    rolloutState: detail.item.current_phase || detail.item.status || copy.unknown,
    imageLabel: image.imageLabel,
    imageSource: image.imageSource,
    affectedTargets: detail.item.total_targets,
    succeededTargets: detail.item.succeeded_targets,
    failedTargets: detail.item.failed_targets,
    phases: [
      {
        key: "pull",
        label: copy.phases.pull.label,
        status: stepStatusOrFallback(pullStep, detail, "pull"),
        detail: pullStep?.message || copy.phases.pull.detail,
      },
      {
        key: "start",
        label: copy.phases.start.label,
        status: stepStatusOrFallback(startStep, detail, "start"),
        detail: startStep?.message || copy.phases.start.detail,
      },
      {
        key: "health",
        label: copy.phases.health.label,
        status: stepStatusOrFallback(healthStep, detail, "health"),
        detail: healthStep?.message || copy.phases.health.detail,
      },
      {
        key: "rollback",
        label: copy.phases.rollback.label,
        status: stepStatusOrFallback(rollbackStep, detail, "rollback"),
        detail: rollbackStep?.message || copy.phases.rollback.detail,
      },
    ],
  };
}
