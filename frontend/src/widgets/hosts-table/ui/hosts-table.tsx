"use client";

import { useMemo, useState } from "react";
import {
  Button,
  EmptyState,
  HealthBadge,
  SearchInput,
  Select,
  StatusBadge,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/src/shared/ui";
import { environmentOptions } from "@/src/shared/constants/dashboard";
import { formatPercent, formatRelativeLabel } from "@/src/shared/lib/dashboard";
import { cn } from "@/src/shared/lib/cn";
import type { Host, HostStatus } from "@/src/shared/types/dashboard";

type HostsTableProps = {
  hosts: Host[];
  loading?: boolean;
  selectedHostId?: string;
  onSelectHost?: (host: Host) => void;
};

const CONTENT_INSET = "px-4 md:px-6";

const statusOptions: Array<{ label: string; value: HostStatus | "all" }> = [
  { label: "Все статусы", value: "all" },
  { label: "В сети", value: "online" },
  { label: "Недоступен", value: "offline" },
  { label: "Снижен", value: "degraded" },
  { label: "Подключается", value: "enrolling" },
];

export function HostsTable({
  hosts,
  loading = false,
  selectedHostId,
  onSelectHost,
}: HostsTableProps) {
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState<HostStatus | "all">("all");
  const [environment, setEnvironment] = useState<string>("all");

  const filteredHosts = useMemo(() => {
    const query = search.trim().toLowerCase();

    return hosts.filter((host) => {
      const matchesSearch =
        query.length === 0 ||
        [host.name, host.cluster, host.region, host.ipAddress, host.provider]
          .join(" ")
          .toLowerCase()
          .includes(query);

      const matchesStatus = status === "all" || host.status === status;
      const matchesEnvironment = environment === "all" || host.environment === environment;

      return matchesSearch && matchesStatus && matchesEnvironment;
    });
  }, [environment, hosts, search, status]);

  return (
    <section className="min-w-0">
      <div className={cn(CONTENT_INSET, "border-b border-[color:var(--border)] py-4")}>
        <div className="flex min-w-0 flex-col gap-4">
          <div className="flex min-w-0 flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
            <div className="min-w-0 space-y-1">
              <h3 className="text-base font-semibold text-[color:var(--foreground)]">
                Инвентарь хостов
              </h3>
              <p className="text-sm text-[color:var(--muted-foreground)]">
                Фильтруйте парк по статусу, окружению или идентификатору хоста.
              </p>
            </div>

            <div className="flex flex-wrap items-center gap-2 lg:justify-end">
              <Button type="button" variant="outline">
                Экспорт
              </Button>
            </div>
          </div>

          <div className="grid min-w-0 gap-3 md:grid-cols-[minmax(0,2fr)_1fr_1fr]">
            <SearchInput
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Поиск по хосту, региону, кластеру или IP"
            />
            <Select
              value={status}
              onChange={(event) => setStatus(event.target.value as HostStatus | "all")}
              options={statusOptions}
              placeholder="Выберите статус"
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
        </div>
      </div>

      <div className="min-w-0 overflow-x-auto">
        <Table>
          <TableHeader>
            <TableRow className="hover:bg-transparent">
              <TableHead className="px-4 py-4 md:px-6">Хост</TableHead>
              <TableHead className="px-4 py-4 md:px-6">Статус</TableHead>
              <TableHead className="px-4 py-4 md:px-6">Состояние</TableHead>
              <TableHead className="px-4 py-4 md:px-6">Окружение</TableHead>
              <TableHead className="px-4 py-4 md:px-6">Кластер</TableHead>
              <TableHead className="px-4 py-4 md:px-6">Нагрузка</TableHead>
              <TableHead className="px-4 py-4 md:px-6">Последний сигнал</TableHead>
              <TableHead className="px-4 py-4 text-right md:px-6">Действие</TableHead>
            </TableRow>
          </TableHeader>

          <TableBody>
            {loading ? (
              <TableRow>
                <TableCell
                  colSpan={8}
                  className="px-4 py-10 text-center text-[color:var(--muted-foreground)] md:px-6"
                >
                  Загрузка хостов...
                </TableCell>
              </TableRow>
            ) : filteredHosts.length === 0 ? (
              <TableRow>
                <TableCell colSpan={8} className="p-0">
                  <EmptyState
                    variant="flush"
                    title="Нет хостов по выбранным фильтрам"
                    description="Измените поисковый запрос или расширьте фильтр по статусу."
                  />
                </TableCell>
              </TableRow>
            ) : (
              filteredHosts.map((host) => (
                <TableRow
                  key={host.id}
                  className={
                    selectedHostId === host.id
                      ? "bg-[color:rgba(56,189,248,0.08)]"
                      : undefined
                  }
                >
                  <TableCell className="px-4 py-4 md:px-6">
                    <div className="min-w-0">
                      <p className="truncate font-medium text-[color:var(--foreground)]">
                        {host.name}
                      </p>
                      <p className="mt-1 text-xs text-[color:var(--muted-foreground)]">
                        {host.ipAddress}
                      </p>
                    </div>
                  </TableCell>

                  <TableCell className="px-4 py-4 md:px-6">
                    <StatusBadge status={host.status} />
                  </TableCell>

                  <TableCell className="px-4 py-4 md:px-6">
                    <HealthBadge health={host.health} />
                  </TableCell>

                  <TableCell className="px-4 py-4 uppercase text-[color:var(--muted-foreground)] md:px-6">
                    {host.environment}
                  </TableCell>

                  <TableCell className="px-4 py-4 md:px-6">
                    <div className="min-w-0">
                      <p className="truncate font-medium text-[color:var(--foreground)]">
                        {host.cluster}
                      </p>
                      <p className="mt-1 text-xs text-[color:var(--muted-foreground)]">
                        {host.region}
                      </p>
                    </div>
                  </TableCell>

                  <TableCell className="px-4 py-4 text-[color:var(--muted-foreground)] md:px-6">
                    {formatPercent(host.cpuLoad)} CPU / {formatPercent(host.memoryUsage)} RAM
                  </TableCell>

                  <TableCell className="px-4 py-4 text-[color:var(--muted-foreground)] md:px-6">
                    {formatRelativeLabel(host.lastSeenAt)}
                  </TableCell>

                  <TableCell className="px-4 py-4 text-right md:px-6">
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => onSelectHost?.(host)}
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
    </section>
  );
}
