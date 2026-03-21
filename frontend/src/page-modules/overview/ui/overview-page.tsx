"use client";

import { alerts, deploymentJobs, hosts, overviewMetrics } from "@/src/mocks/data/dashboard";
import {
  countAlertsByStatus,
  countHostsByStatus,
  countJobsByStatus,
  formatRelativeLabel,
} from "@/src/shared/lib/dashboard";
import { cn } from "@/src/shared/lib/cn";
import { useI18n } from "@/src/shared/lib/i18n";
import { Button, ToneBadge } from "@/src/shared/ui";

const PAGE_INSET = "px-4 md:px-6";
const SECTION_PAD_Y = "py-5";

const logBars = [12, 18, 14, 22, 24, 20, 26, 32, 28, 34, 31, 39];

const logStreams = [
  {
    id: "gateway",
    stream: "edge-gateway",
    volume: "4.8M",
    latency: "182ms",
    dropRate: "0.04%",
  },
  {
    id: "collector",
    stream: "pulse-collector",
    volume: "3.1M",
    latency: "126ms",
    dropRate: "0.02%",
  },
  {
    id: "resolver",
    stream: "dns-resolver",
    volume: "2.4M",
    latency: "201ms",
    dropRate: "0.07%",
  },
];

export function OverviewPage() {
  const { dictionary, locale } = useI18n();
  const copy = dictionary.overview;
  const hostSummary = countHostsByStatus(hosts);
  const alertSummary = countAlertsByStatus(alerts);
  const jobSummary = countJobsByStatus(deploymentJobs);
  const maxBar = Math.max(...logBars, 1);

  const localizedMetrics = overviewMetrics.map((metric) => ({
    ...metric,
    ...copy.metrics[metric.id as keyof typeof copy.metrics],
  }));

  const activityItems = [
    {
      id: "activity-1",
      ...copy.activity.items[0],
      timestamp: deploymentJobs[0].startedAt,
    },
    {
      id: "activity-2",
      ...copy.activity.items[1],
      timestamp: alerts[0].triggeredAt,
    },
    {
      id: "activity-3",
      ...copy.activity.items[2],
      timestamp: alerts[2].triggeredAt,
    },
  ];

  return (
    <div className="min-w-0">
      <header
        className={cn(
          PAGE_INSET,
          "border-b border-[color:var(--border)] pb-5 pt-4 md:pb-6 md:pt-16"
        )}
      >
        <nav aria-label="Breadcrumb">
          <ol className="flex flex-wrap items-center gap-2 text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
            <li className="flex items-center gap-2">
              <span>{dictionary.common.dashboard}</span>
              <span className="text-[color:var(--border-strong)]">/</span>
            </li>
            <li className="text-[color:var(--foreground)]">{copy.title}</li>
          </ol>
        </nav>

        <div className="mt-4 flex min-w-0 flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
          <div className="min-w-0 max-w-4xl space-y-2">
            <h1 className="text-[28px] font-semibold tracking-tight text-[color:var(--foreground)]">
              {copy.title}
            </h1>
            <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
              {copy.description}
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2 lg:justify-end">
            <Button type="button" variant="outline">
              {copy.createReport}
            </Button>
          </div>
        </div>
      </header>

      <section
        aria-label={copy.metricsAriaLabel}
        className={cn(
          "grid min-w-0 border-b border-[color:var(--border)] sm:grid-cols-2 xl:grid-cols-4",
          "[&>*:last-child]:border-b-0",
          "sm:[&>*:nth-child(odd)]:border-r sm:[&>*:nth-last-child(-n+2)]:border-b-0",
          "xl:[&>*]:border-b-0 xl:[&>*]:border-r xl:[&>*:last-child]:border-r-0"
        )}
      >
        {localizedMetrics.map((metric) => (
          <div
            key={metric.id}
            className={cn(
              PAGE_INSET,
              "min-w-0 border-b border-[color:var(--border)] py-5"
            )}
          >
            <p className="text-sm font-medium text-[color:var(--muted-foreground)]">
              {metric.label}
            </p>
            <div className="mt-2 text-3xl font-semibold tracking-tight text-[color:var(--foreground)]">
              {metric.value}
            </div>
            <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
              {metric.description}
            </p>
            <div className="mt-3">
              <ToneBadge
                tone={
                  metric.trend === "up"
                    ? "positive"
                    : metric.trend === "down"
                      ? "danger"
                      : "neutral"
                }
              >
                {(metric.change > 0 ? "+" : "") + metric.change.toFixed(1)}%
              </ToneBadge>
            </div>
          </div>
        ))}
      </section>

      <div className="grid min-w-0 xl:grid-cols-[minmax(0,1fr)_420px]">
        <div className="min-w-0">
          <section className={cn(PAGE_INSET, SECTION_PAD_Y)}>
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-tight text-[color:var(--foreground)]">
                {copy.operations.title}
              </h2>
              <p className="max-w-3xl text-sm text-[color:var(--muted-foreground)]">
                {copy.operations.description}
              </p>
            </div>
          </section>

          <section
            aria-label={copy.operations.title}
            className={cn(
              "grid min-w-0 border-y border-[color:var(--border)] md:grid-cols-3",
              "[&>*:last-child]:border-b-0",
              "md:[&>*]:border-b-0 md:[&>*]:border-r md:[&>*:last-child]:border-r-0"
            )}
          >
            <div
              className={cn(
                PAGE_INSET,
                "min-w-0 border-b border-[color:var(--border)] py-4"
              )}
            >
              <p className="text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
                {copy.operations.managedHosts}
              </p>
              <p className="mt-2 text-2xl font-semibold text-[color:var(--foreground)]">
                {hosts.length}
              </p>
              <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                {hostSummary.online} {copy.operations.onlineSuffix}, {hostSummary.offline}{" "}
                {copy.operations.offlineSuffix}
              </p>
            </div>

            <div
              className={cn(
                PAGE_INSET,
                "min-w-0 border-b border-[color:var(--border)] py-4"
              )}
            >
              <p className="text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
                {copy.operations.openIncidents}
              </p>
              <p className="mt-2 text-2xl font-semibold text-[color:var(--foreground)]">
                {alertSummary.active + alertSummary.acknowledged}
              </p>
              <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                {copy.operations.activeAndAcknowledged}
              </p>
            </div>

            <div
              className={cn(
                PAGE_INSET,
                "min-w-0 border-b border-[color:var(--border)] py-4"
              )}
            >
              <p className="text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
                {copy.operations.jobsToday}
              </p>
              <p className="mt-2 text-2xl font-semibold text-[color:var(--foreground)]">
                {deploymentJobs.length}
              </p>
              <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                {jobSummary.success} {copy.operations.successfulSuffix}, {jobSummary.failed}{" "}
                {copy.operations.failedSuffix}
              </p>
            </div>
          </section>

          <section className="min-w-0">
            <div className={cn(PAGE_INSET, SECTION_PAD_Y)}>
              <div className="space-y-1">
                <h3 className="text-base font-semibold text-[color:var(--foreground)]">
                  {copy.logs.title}
                </h3>
                <p className="max-w-3xl text-sm text-[color:var(--muted-foreground)]">
                  {copy.logs.description}
                </p>
              </div>
            </div>

            <div className="border-y border-[color:var(--border)]">
              <div className={cn(PAGE_INSET, "py-5")}>
                <div className="flex h-44 items-end gap-2">
                  {logBars.map((point, index) => (
                    <div key={index} className="flex min-w-0 flex-1 items-end">
                      <div
                        className="w-full rounded-[2px] bg-gradient-to-t from-sky-500 to-cyan-300"
                        style={{ height: `${Math.max((point / maxBar) * 100, 12)}%` }}
                      />
                    </div>
                  ))}
                </div>
              </div>

              <div className="border-t border-[color:var(--border)]">
                <div
                  className={cn(
                    PAGE_INSET,
                    "hidden md:grid md:grid-cols-[minmax(0,1.5fr)_120px_120px_120px] md:gap-4 md:py-3 md:text-xs md:uppercase md:tracking-[0.16em] md:text-[color:var(--muted-foreground)]"
                  )}
                >
                  <span>{copy.logs.stream}</span>
                  <span>{copy.logs.volume}</span>
                  <span>{copy.logs.latency}</span>
                  <span>{copy.logs.drops}</span>
                </div>

                {logStreams.map((stream) => (
                  <div
                    key={stream.id}
                    className={cn(
                      PAGE_INSET,
                      "grid min-w-0 gap-x-4 gap-y-3 border-t border-[color:var(--border)] py-3 text-sm",
                      "grid-cols-2 md:grid-cols-[minmax(0,1.5fr)_120px_120px_120px]"
                    )}
                  >
                    <div className="min-w-0">
                      <p className="text-[10px] uppercase tracking-[0.16em] text-[color:var(--muted-foreground)] md:hidden">
                        {copy.logs.stream}
                      </p>
                      <p className="truncate font-medium text-[color:var(--foreground)]">
                        {stream.stream}
                      </p>
                    </div>

                    <div>
                      <p className="text-[10px] uppercase tracking-[0.16em] text-[color:var(--muted-foreground)] md:hidden">
                        {copy.logs.volume}
                      </p>
                      <p className="text-[color:var(--foreground)]">{stream.volume}</p>
                    </div>

                    <div>
                      <p className="text-[10px] uppercase tracking-[0.16em] text-[color:var(--muted-foreground)] md:hidden">
                        {copy.logs.latency}
                      </p>
                      <p className="text-[color:var(--foreground)]">{stream.latency}</p>
                    </div>

                    <div>
                      <p className="text-[10px] uppercase tracking-[0.16em] text-[color:var(--muted-foreground)] md:hidden">
                        {copy.logs.drops}
                      </p>
                      <p className="text-[color:var(--muted-foreground)]">
                        {stream.dropRate}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </section>
        </div>

        <aside className="min-w-0 border-t border-[color:var(--border)] xl:border-l xl:border-t-0">
          <div className={cn(PAGE_INSET, SECTION_PAD_Y)}>
            <div className="space-y-1">
              <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                {copy.activity.title}
              </h2>
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.activity.description}
              </p>
            </div>
          </div>

          <div className="border-t border-[color:var(--border)]">
            {activityItems.map((item) => (
              <section
                key={item.id}
                className={cn(
                  PAGE_INSET,
                  "border-b border-[color:var(--border)] py-4 last:border-b-0"
                )}
              >
                <p className="text-sm font-medium text-[color:var(--foreground)]">
                  {item.title}
                </p>
                <p className="mt-2 text-sm leading-6 text-[color:var(--muted-foreground)]">
                  {item.description}
                </p>
                <p className="mt-3 text-xs uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                  {formatRelativeLabel(item.timestamp, locale)}
                </p>
              </section>
            ))}
          </div>
        </aside>
      </div>
    </div>
  );
}
