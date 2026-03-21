"use client";

import Link from "next/link";
import type { Locale } from "@/src/shared/config";
import { Badge, Button, Card, EmptyState } from "@/src/shared/ui";
import { withLocalePath } from "@/src/shared/lib/i18n";
import {
  JsonPreview,
  SectionCard,
  formatDateTime,
} from "@/src/features/operations/ui/operations-ui";
import {
  getSeverityTone,
  type AlertDetailModel,
} from "@/src/shared/lib/operations-workbench";

function toBadgeVariant(value?: string) {
  const normalized = value?.trim().toLowerCase() ?? "";
  if (normalized === "blocked") {
    return "danger";
  }
  if (normalized === "queued") {
    return "warning";
  }
  if (normalized === "delivered") {
    return "success";
  }
  const tone = getSeverityTone(value);
  if (tone === "danger") {
    return "danger";
  }
  if (tone === "warning") {
    return "warning";
  }
  if (tone === "success") {
    return "success";
  }
  return "default";
}

export function AlertExplanationDrawer({
  alert,
  locale,
}: {
  alert: AlertDetailModel | null;
  locale: Locale;
}) {
  if (!alert) {
    return (
      <Card>
        <EmptyState
          variant="flush"
          title="No alert selected"
          description="Choose an alert instance to inspect explanation, source signals, cluster bindings, and delivery status."
        />
      </Card>
    );
  }

  return (
    <SectionCard
      title="Alert detail"
      description="Unifies anomaly context, security posture, cluster binding, and projected delivery status in one operator panel."
      action={
        alert.securityFinding?.relatedRoute ? (
          <Link href={withLocalePath(locale, alert.securityFinding.relatedRoute.href)}>
            <Button variant="outline" size="sm" className="h-10 px-4">
              {alert.securityFinding.relatedRoute.label}
            </Button>
          </Link>
        ) : null
      }
    >
      <div className="space-y-5">
        <Card className="space-y-3 p-4">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant={toBadgeVariant(alert.severity)}>{alert.severity}</Badge>
            <Badge>{alert.status}</Badge>
          </div>
          <p className="text-xl font-semibold text-[color:var(--foreground)]">
            {alert.title}
          </p>
          <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
            {alert.explanation}
          </p>
          <p className="text-xs uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
            Triggered {formatDateTime(alert.triggeredAt)}
          </p>
        </Card>

        <div className="grid gap-4 md:grid-cols-2">
          <Card className="space-y-3 p-4">
            <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
              Source signals
            </h3>
            <div className="space-y-2">
              {alert.sourceSignals.map((item) => (
                <div
                  key={`${item.label}-${item.value}`}
                  className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface-subtle)] px-3 py-2"
                >
                  <p className="text-xs uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                    {item.label}
                  </p>
                  <p className="mt-1 text-sm text-[color:var(--foreground)]">
                    {item.value}
                  </p>
                </div>
              ))}
            </div>
          </Card>

          <Card className="space-y-3 p-4">
            <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
              Correlation
            </h3>
            {alert.anomaly ? (
              <div className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface-subtle)] p-3">
                <div className="flex flex-wrap items-center gap-2">
                  <Badge variant={toBadgeVariant(alert.anomaly.severity)}>
                    {alert.anomaly.severity}
                  </Badge>
                  <span className="text-sm font-medium text-[color:var(--foreground)]">
                    {alert.anomaly.title}
                  </span>
                </div>
                <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                  Linked anomaly at {formatDateTime(alert.anomaly.triggeredAt)}
                </p>
              </div>
            ) : (
              <p className="text-sm text-[color:var(--muted-foreground)]">
                No correlated anomaly item was returned for this alert.
              </p>
            )}

            {alert.securityFinding ? (
              <div className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface-subtle)] p-3">
                <div className="flex flex-wrap items-center gap-2">
                  <Badge variant={toBadgeVariant(alert.securityFinding.severity)}>
                    {alert.securityFinding.severity}
                  </Badge>
                  <span className="text-sm font-medium text-[color:var(--foreground)]">
                    {alert.securityFinding.title}
                  </span>
                </div>
                <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                  {alert.securityFinding.summary}
                </p>
              </div>
            ) : null}
          </Card>
        </div>

        <div className="grid gap-4 md:grid-cols-2">
          <Card className="space-y-3 p-4">
            <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
              Cluster bindings
            </h3>
            {alert.clusterBindings.length === 0 ? (
              <p className="text-sm text-[color:var(--muted-foreground)]">
                No cluster binding matched this alert.
              </p>
            ) : (
              <div className="space-y-2">
                {alert.clusterBindings.map((binding) => (
                  <div
                    key={`${binding.instanceName}-${binding.routeLabel}-${binding.chatId}`}
                    className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface-subtle)] px-3 py-2"
                  >
                    <p className="text-sm font-medium text-[color:var(--foreground)]">
                      {binding.instanceName}
                    </p>
                    <p className="text-sm text-[color:var(--muted-foreground)]">
                      {binding.cluster} / {binding.routeLabel} / {binding.chatId}
                    </p>
                  </div>
                ))}
              </div>
            )}
          </Card>

          <Card className="space-y-3 p-4">
            <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
              Delivery status
            </h3>
            <div className="space-y-2">
              {alert.deliveryStatus.map((delivery) => (
                <div
                  key={`${delivery.instanceId}-${delivery.routeLabel}-${delivery.chatId}`}
                  className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface-subtle)] p-3"
                >
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge variant={toBadgeVariant(delivery.status)}>{delivery.status}</Badge>
                    <span className="text-sm font-medium text-[color:var(--foreground)]">
                      {delivery.instanceName}
                    </span>
                  </div>
                  <p className="mt-2 text-sm text-[color:var(--foreground)]">
                    {delivery.cluster} / {delivery.routeLabel} / {delivery.chatId}
                  </p>
                  <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                    {delivery.detail}
                  </p>
                </div>
              ))}
            </div>
          </Card>
        </div>

        <Card className="space-y-3 p-4">
          <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
            Raw payload
          </h3>
          <JsonPreview value={alert.payload} />
        </Card>
      </div>
    </SectionCard>
  );
}
