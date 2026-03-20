"use client";

import { alerts, deploymentJobs, hosts, overviewMetrics } from "@/src/mocks/data/dashboard";
import {
  countAlertsByStatus,
  countHostsByStatus,
  countJobsByStatus,
  formatRelativeLabel,
} from "@/src/shared/lib/dashboard";
import { cn } from "@/src/shared/lib/cn";
import { Button, ConsolePage, ToneBadge } from "@/src/shared/ui";

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
  const hostSummary = countHostsByStatus(hosts);
  const alertSummary = countAlertsByStatus(alerts);
  const jobSummary = countJobsByStatus(deploymentJobs);
  const maxBar = Math.max(...logBars, 1);

  const activityItems = [
    {
      id: "activity-1",
      title: "Задание job-1201 все еще разворачивается",
      description: "Релиз edge gateway завершил 11 из 18 целей.",
      timestamp: deploymentJobs[0].startedAt,
    },
    {
      id: "activity-2",
      title: "Критическое оповещение по хранилищу на db-shadow-1",
      description: "Дежурный по БД подтвердил рост риска заполнения диска.",
      timestamp: alerts[0].triggeredAt,
    },
    {
      id: "activity-3",
      title: "Пульс collector восстановился на стейдже",
      description: "Кратковременная задержка исчезла автоматически после перезапуска.",
      timestamp: alerts[2].triggeredAt,
    },
  ];

  return (
    <ConsolePage>
      <div className="min-w-0">
        <header className={cn(PAGE_INSET, "border-b border-[color:var(--border)] pb-5 pt-4 md:pb-6 md:pt-16")}>
          <nav aria-label="Breadcrumb">
            <ol className="flex flex-wrap items-center gap-2 text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
              <li className="flex items-center gap-2">
                <span>Панель</span>
                <span className="text-[color:var(--border-strong)]">/</span>
              </li>
              <li className="text-[color:var(--foreground)]">Обзор</li>
            </ol>
          </nav>

          <div className="mt-4 flex min-w-0 flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
            <div className="min-w-0 max-w-4xl space-y-2">
              <h1 className="text-[28px] font-semibold tracking-tight text-[color:var(--foreground)]">
                Обзор
              </h1>
              <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
                Операционный workspace по инфраструктуре, активным инцидентам, релизам и
                текущему потоку событий.
              </p>
            </div>

            <div className="flex flex-wrap items-center gap-2 lg:justify-end">
              <Button type="button" variant="outline">
                Создать отчет
              </Button>
            </div>
          </div>
        </header>

        <section
          aria-label="Сводные метрики"
          className={cn(
            "grid min-w-0 border-b border-[color:var(--border)] sm:grid-cols-2 xl:grid-cols-4",
            "[&>*:last-child]:border-b-0",
            "sm:[&>*:nth-child(odd)]:border-r sm:[&>*:nth-last-child(-n+2)]:border-b-0",
            "xl:[&>*]:border-b-0 xl:[&>*]:border-r xl:[&>*:last-child]:border-r-0"
          )}
        >
          {overviewMetrics.map((metric) => (
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
                  Операционный контур
                </h2>
                <p className="max-w-3xl text-sm text-[color:var(--muted-foreground)]">
                  Сводка по хостам, инцидентам и журналам как часть единого рабочего полотна.
                </p>
              </div>
            </section>

            <section
              aria-label="Сводка по операционному контуру"
              className={cn(
                "grid min-w-0 border-y border-[color:var(--border)] md:grid-cols-3",
                "[&>*:last-child]:border-b-0",
                "md:[&>*]:border-b-0 md:[&>*]:border-r md:[&>*:last-child]:border-r-0"
              )}
            >
              <div className={cn(PAGE_INSET, "min-w-0 border-b border-[color:var(--border)] py-4")}>
                <p className="text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
                  Управляемые хосты
                </p>
                <p className="mt-2 text-2xl font-semibold text-[color:var(--foreground)]">
                  {hosts.length}
                </p>
                <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                  {hostSummary.online} в сети, {hostSummary.offline} недоступны
                </p>
              </div>

              <div className={cn(PAGE_INSET, "min-w-0 border-b border-[color:var(--border)] py-4")}>
                <p className="text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
                  Открытые инциденты
                </p>
                <p className="mt-2 text-2xl font-semibold text-[color:var(--foreground)]">
                  {alertSummary.active + alertSummary.acknowledged}
                </p>
                <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                  Активные и подтвержденные оповещения
                </p>
              </div>

              <div className={cn(PAGE_INSET, "min-w-0 border-b border-[color:var(--border)] py-4")}>
                <p className="text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
                  Задания за сегодня
                </p>
                <p className="mt-2 text-2xl font-semibold text-[color:var(--foreground)]">
                  {deploymentJobs.length}
                </p>
                <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                  {jobSummary.success} успешных, {jobSummary.failed} с ошибкой
                </p>
              </div>
            </section>

            <section className="min-w-0">
              <div className={cn(PAGE_INSET, SECTION_PAD_Y)}>
                <div className="space-y-1">
                  <h3 className="text-base font-semibold text-[color:var(--foreground)]">
                    Поток журналов
                  </h3>
                  <p className="max-w-3xl text-sm text-[color:var(--muted-foreground)]">
                    Плотная рабочая область для мониторинга объема сигналов, задержек и
                    потерь.
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
                    <span>Поток</span>
                    <span>Объем</span>
                    <span>P95</span>
                    <span>Потери</span>
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
                          Поток
                        </p>
                        <p className="truncate font-medium text-[color:var(--foreground)]">
                          {stream.stream}
                        </p>
                      </div>

                      <div>
                        <p className="text-[10px] uppercase tracking-[0.16em] text-[color:var(--muted-foreground)] md:hidden">
                          Объем
                        </p>
                        <p className="text-[color:var(--foreground)]">{stream.volume}</p>
                      </div>

                      <div>
                        <p className="text-[10px] uppercase tracking-[0.16em] text-[color:var(--muted-foreground)] md:hidden">
                          P95
                        </p>
                        <p className="text-[color:var(--foreground)]">{stream.latency}</p>
                      </div>

                      <div>
                        <p className="text-[10px] uppercase tracking-[0.16em] text-[color:var(--muted-foreground)] md:hidden">
                          Потери
                        </p>
                        <p className="text-[color:var(--muted-foreground)]">{stream.dropRate}</p>
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
                  Последняя активность
                </h2>
                <p className="text-sm text-[color:var(--muted-foreground)]">
                  Свежие операционные события по платформе.
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
                    {formatRelativeLabel(item.timestamp)}
                  </p>
                </section>
              ))}
            </div>
          </aside>
        </div>
      </div>
    </ConsolePage>
  );
}
