"use client";

import { useSearchParams } from "next/navigation";
import { useEffect, useMemo, useState } from "react";
import { translateValueLabel, useI18n } from "@/src/shared/lib/i18n";
import {
  SectionCard,
} from "@/src/features/operations/ui/operations-ui";
import {
  getAlertsWorkbenchData,
  getSeverityTone,
  type AlertsWorkbenchData,
} from "@/src/shared/lib/operations-workbench";
import {
  Badge,
  Card,
  EmptyState,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { ErrorCard, LoadingCard } from "@/src/page-modules/common/ui/runtime-state";
import { AlertExplanationDrawer } from "./alert-explanation-drawer";

const copyByLocale = {
  en: {
    title: "Alerts",
    description:
      "Live alert instances plus a correlated operator detail view for anomaly, posture, binding, and delivery context.",
    loading: "Loading alerts...",
    error: "Failed to load alerts",
    metrics: {
      openAlerts: "Open alerts",
      rules: "Alert rules",
      telegramInstances: "Routed Telegram instances",
    },
    instances: {
      title: "Alert instances",
      description:
        "Pick an alert from the table to open the correlated detail panel.",
      columns: {
        title: "Title",
        status: "Status",
        severity: "Severity",
        host: "Host",
        service: "Service",
      },
      emptyTitle: "No alert instances",
      emptyDescription: "Triggered alerts will appear here.",
    },
    rules: {
      title: "Alert rules",
      description:
        "The list below preserves the existing rule inventory while the detail experience stays focused on live alert instances.",
      columns: {
        name: "Name",
        status: "Status",
        severity: "Severity",
        scope: "Scope",
      },
      emptyTitle: "No alert rules",
      emptyDescription:
        "Create rules through the API to start threshold evaluation.",
    },
  },
  ru: {
    title: "Алерты",
    description:
      "Живые alert-инстансы и коррелированная панель деталей оператора с контекстом аномалий, posture, binding и доставки.",
    loading: "Загрузка алертов...",
    error: "Не удалось загрузить алерты",
    metrics: {
      openAlerts: "Открытые алерты",
      rules: "Alert rules",
      telegramInstances: "Маршрутизированные Telegram-инстансы",
    },
    instances: {
      title: "Инстансы алертов",
      description:
        "Выберите алерт в таблице, чтобы открыть связанную панель деталей.",
      columns: {
        title: "Заголовок",
        status: "Статус",
        severity: "Severity",
        host: "Хост",
        service: "Сервис",
      },
      emptyTitle: "Нет инстансов алертов",
      emptyDescription: "Сработавшие алерты появятся здесь.",
    },
    rules: {
      title: "Правила алертов",
      description:
        "Список ниже сохраняет существующий инвентарь правил, пока detail-view сфокусирован на живых alert-инстансах.",
      columns: {
        name: "Имя",
        status: "Статус",
        severity: "Severity",
        scope: "Скоуп",
      },
      emptyTitle: "Нет правил алертов",
      emptyDescription:
        "Создайте правила через API, чтобы запустить оценку порогов.",
    },
  },
} as const;

function toBadgeVariant(value?: string) {
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

function isOpenStatus(value?: string) {
  const normalized = value?.trim().toLowerCase() ?? "";
  return !["resolved", "closed", "delivered"].includes(normalized);
}

export function AlertsPage({ embedded = false }: { embedded?: boolean } = {}) {
  const { dictionary, locale } = useI18n();
  const copy = copyByLocale[locale];
  const searchParams = useSearchParams();
  const alertParam = searchParams.get("alert");

  const [data, setData] = useState<AlertsWorkbenchData | null>(null);
  const [selectedAlertId, setSelectedAlertId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);
      try {
        const response = await getAlertsWorkbenchData(locale);
        if (cancelled) {
          return;
        }
        setData(response);
        setSelectedAlertId((current) => {
          const requested = response.alerts.find((item) => item.id === alertParam)?.id;
          if (requested) {
            return requested;
          }
          if (response.alerts.some((item) => item.id === current)) {
            return current;
          }
          return response.alerts[0]?.id ?? null;
        });
      } catch (loadError) {
        if (!cancelled) {
          setError(loadError instanceof Error ? loadError.message : copy.error);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void load();

    return () => {
      cancelled = true;
    };
  }, [alertParam, copy.error, locale]);

  useEffect(() => {
    if (!data || !alertParam) {
      return;
    }

    const requested = data.alerts.find((item) => item.id === alertParam);
    if (requested) {
      setSelectedAlertId(requested.id);
    }
  }, [data, alertParam]);

  const selectedAlert = useMemo(() => {
    return data?.alerts.find((item) => item.id === selectedAlertId) ?? null;
  }, [data?.alerts, selectedAlertId]);

  const openAlertsCount = data?.alerts.filter((item) => isOpenStatus(item.status)).length ?? 0;

  return (
    <div className={embedded ? "space-y-4" : "space-y-6"}>
      {!embedded ? (
        <PageHeader
          title={copy.title}
          description={copy.description}
          breadcrumbs={[
            { label: dictionary.common.dashboard, href: "#" },
            { label: copy.title },
          ]}
        />
      ) : null}

      {loading ? <LoadingCard label={copy.loading} /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error && data ? (
        <>
          <section className="grid gap-4 md:grid-cols-3">
            <Card className="space-y-2 p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.metrics.openAlerts}
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {openAlertsCount}
              </p>
            </Card>
            <Card className="space-y-2 p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.metrics.rules}
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {data.rules.length}
              </p>
            </Card>
            <Card className="space-y-2 p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.metrics.telegramInstances}
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {data.telegramInstances.filter((item) => item.enabled).length}
              </p>
            </Card>
          </section>

          <section className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1.05fr)]">
            <SectionCard
              title={copy.instances.title}
              description={copy.instances.description}
            >
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{copy.instances.columns.title}</TableHead>
                    <TableHead>{copy.instances.columns.status}</TableHead>
                    <TableHead>{copy.instances.columns.severity}</TableHead>
                    <TableHead>{copy.instances.columns.host}</TableHead>
                    <TableHead>{copy.instances.columns.service}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {data.alerts.length === 0 ? (
                    <TableRow>
                      <TableCell colSpan={5}>
                        <EmptyState
                          variant="flush"
                          title={copy.instances.emptyTitle}
                          description={copy.instances.emptyDescription}
                        />
                      </TableCell>
                    </TableRow>
                  ) : (
                    data.alerts.map((item) => (
                      <TableRow
                        key={item.id}
                        className={
                          item.id === selectedAlertId
                            ? "bg-[color:rgba(56,189,248,0.08)]"
                            : undefined
                        }
                        onClick={() => setSelectedAlertId(item.id)}
                      >
                        <TableCell className="font-medium text-[color:var(--foreground)]">
                          {item.title}
                        </TableCell>
                        <TableCell>
                          <Badge>{translateValueLabel(item.status, locale)}</Badge>
                        </TableCell>
                        <TableCell>
                          <Badge variant={toBadgeVariant(item.severity)}>
                            {translateValueLabel(item.severity, locale)}
                          </Badge>
                        </TableCell>
                        <TableCell>{item.host || "n/a"}</TableCell>
                        <TableCell>{item.service || "n/a"}</TableCell>
                      </TableRow>
                    ))
                  )}
                </TableBody>
              </Table>
            </SectionCard>

            <AlertExplanationDrawer alert={selectedAlert} locale={locale} />
          </section>

          <SectionCard
            title={copy.rules.title}
            description={copy.rules.description}
          >
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{copy.rules.columns.name}</TableHead>
                  <TableHead>{copy.rules.columns.status}</TableHead>
                  <TableHead>{copy.rules.columns.severity}</TableHead>
                  <TableHead>{copy.rules.columns.scope}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {data.rules.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={4}>
                      <EmptyState
                        variant="flush"
                        title={copy.rules.emptyTitle}
                        description={copy.rules.emptyDescription}
                      />
                    </TableCell>
                  </TableRow>
                ) : (
                  data.rules.map((rule) => (
                    <TableRow key={rule.alert_rule_id}>
                      <TableCell className="font-medium text-[color:var(--foreground)]">
                        {rule.name}
                      </TableCell>
                      <TableCell>{translateValueLabel(rule.status, locale)}</TableCell>
                      <TableCell>{translateValueLabel(rule.severity, locale)}</TableCell>
                      <TableCell>
                        {rule.scope_type}
                        {rule.scope_id ? `:${rule.scope_id}` : ""}
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </SectionCard>
        </>
      ) : null}
    </div>
  );
}
