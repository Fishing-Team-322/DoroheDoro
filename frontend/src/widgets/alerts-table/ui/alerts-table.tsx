"use client";

import { useMemo, useState } from "react";
import {
  Button,
  Select,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/src/shared/ui";
import { environmentOptions } from "@/src/shared/constants/dashboard";
import { formatRelativeLabel } from "@/src/shared/lib/dashboard";
import { FilterBar, SearchInput, SeverityBadge, StatusBadge, TableToolbar, EmptyState } from "@/src/shared/ui";
import type { Alert, AlertStatus, Severity } from "@/src/shared/types/dashboard";

const statusOptions: Array<{ label: string; value: AlertStatus | "all" }> = [
  { label: "Все статусы", value: "all" },
  { label: "Активно", value: "active" },
  { label: "Подтверждено", value: "acknowledged" },
  { label: "Решено", value: "resolved" },
  { label: "Приглушено", value: "muted" },
];

const severityOptions: Array<{ label: string; value: Severity | "all" }> = [
  { label: "Все уровни", value: "all" },
  { label: "Отладка", value: "debug" },
  { label: "Инфо", value: "info" },
  { label: "Предупреждение", value: "warning" },
  { label: "Ошибка", value: "error" },
  { label: "Критично", value: "critical" },
];

export function AlertsTable({
  alerts,
  loading = false,
  selectedAlertId,
  onSelectAlert,
}: {
  alerts: Alert[];
  loading?: boolean;
  selectedAlertId?: string;
  onSelectAlert?: (alert: Alert) => void;
}) {
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState<AlertStatus | "all">("all");
  const [severity, setSeverity] = useState<Severity | "all">("all");
  const [environment, setEnvironment] = useState<string>("all");

  const filteredAlerts = useMemo(() => {
    const query = search.trim().toLowerCase();

    return alerts.filter((alert) => {
      const matchesSearch =
        query.length === 0 ||
        [alert.id, alert.title, alert.source, alert.host, alert.summary]
          .join(" ")
          .toLowerCase()
          .includes(query);

      return (
        matchesSearch &&
        (status === "all" || alert.status === status) &&
        (severity === "all" || alert.severity === severity) &&
        (environment === "all" || alert.environment === environment)
      );
    });
  }, [alerts, environment, search, severity, status]);

  return (
    <div className="space-y-4">
      <TableToolbar
        title="Поток оповещений"
        description="Изучайте инциденты по важности, владельцу и окружению."
      >
        <FilterBar className="items-stretch">
          <div className="grid flex-1 gap-3 xl:grid-cols-[minmax(0,2fr)_1fr_1fr_1fr]">
            <SearchInput
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Поиск по заголовку, источнику, хосту или описанию"
            />
            <Select
              value={status}
              onChange={(event) => setStatus(event.target.value as AlertStatus | "all")}
              options={statusOptions}
              placeholder="Выберите статус"
              selectSize="md"
            />
            <Select
              value={severity}
              onChange={(event) => setSeverity(event.target.value as Severity | "all")}
              options={severityOptions}
              placeholder="Выберите уровень"
              selectSize="md"
            />
            <Select
              value={environment}
              onChange={(event) => setEnvironment(event.target.value)}
              options={environmentOptions}
              placeholder="Выберите окружение"
              selectSize="md"
            />
          </div>
        </FilterBar>
      </TableToolbar>

      <div className="overflow-hidden rounded-3xl border border-zinc-800 bg-zinc-950 shadow-sm shadow-black/20">
        <div className="overflow-x-auto">
          <Table>
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead>Оповещение</TableHead>
                <TableHead>Важность</TableHead>
                <TableHead>Статус</TableHead>
                <TableHead>Окружение</TableHead>
                <TableHead>Ответственный</TableHead>
                <TableHead>Время</TableHead>
                <TableHead className="text-right">Действие</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {loading ? (
                <TableRow>
                  <TableCell colSpan={7} className="py-10 text-center text-zinc-500">
                    Загрузка оповещений...
                  </TableCell>
                </TableRow>
              ) : filteredAlerts.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={7} className="p-6">
                    <EmptyState
                      title="В этом представлении оповещений нет"
                      description="По текущим фильтрам всё спокойно."
                    />
                  </TableCell>
                </TableRow>
              ) : (
                filteredAlerts.map((alert) => (
                  <TableRow
                    key={alert.id}
                    className={selectedAlertId === alert.id ? "bg-rose-50" : undefined}
                  >
                    <TableCell>
                      <div>
                        <p className="font-medium text-zinc-100">{alert.title}</p>
                        <p className="text-xs text-zinc-500">
                          {alert.source} - {alert.host}
                        </p>
                      </div>
                    </TableCell>
                    <TableCell>
                      <SeverityBadge severity={alert.severity} />
                    </TableCell>
                    <TableCell>
                      <StatusBadge status={alert.status} />
                    </TableCell>
                    <TableCell className="uppercase text-zinc-600">
                      {alert.environment}
                    </TableCell>
                    <TableCell className="text-zinc-600">
                      {alert.assignee ?? "Не назначен"}
                    </TableCell>
                    <TableCell className="text-zinc-500">
                      {formatRelativeLabel(alert.triggeredAt)}
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        onClick={() => onSelectAlert?.(alert)}
                      >
                        Открыть
                      </Button>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </div>
      </div>
    </div>
  );
}

