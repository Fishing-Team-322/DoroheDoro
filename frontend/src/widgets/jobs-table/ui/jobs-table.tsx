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
import { getDeploymentStatusMeta, formatRelativeLabel } from "@/src/shared/lib/dashboard";
import { FilterBar, SearchInput, TableToolbar, ToneBadge, EmptyState } from "@/src/shared/ui";
import type { DeploymentJob, DeploymentStatus } from "@/src/shared/types/dashboard";

const statusOptions: Array<{ label: string; value: DeploymentStatus | "all" }> = [
  { label: "Все статусы", value: "all" },
  { label: "Ожидает", value: "pending" },
  { label: "Выполняется", value: "running" },
  { label: "Успешно", value: "success" },
  { label: "Ошибка", value: "failed" },
  { label: "Отменено", value: "canceled" },
];

export function JobsTable({
  jobs,
  loading = false,
}: {
  jobs: DeploymentJob[];
  loading?: boolean;
}) {
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState<DeploymentStatus | "all">("all");
  const [environment, setEnvironment] = useState<string>("all");

  const filteredJobs = useMemo(() => {
    const query = search.trim().toLowerCase();

    return jobs.filter((job) => {
      const matchesSearch =
        query.length === 0 ||
        [job.id, job.service, job.version, job.initiatedBy]
          .join(" ")
          .toLowerCase()
          .includes(query);
      const matchesStatus = status === "all" || job.status === status;
      const matchesEnvironment =
        environment === "all" || job.environment === environment;

      return matchesSearch && matchesStatus && matchesEnvironment;
    });
  }, [environment, jobs, search, status]);

  return (
    <div className="space-y-4">
      <TableToolbar
        title="Задания развёртывания"
        description="Операционный вид недавних релизов, повторов и инициаторов запуска."
      >
        <FilterBar>
          <div className="grid flex-1 gap-3 md:grid-cols-[minmax(0,2fr)_1fr_1fr]">
            <SearchInput
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Поиск по заданию, сервису или версии"
            />
            <Select
              value={status}
              onChange={(event) =>
                setStatus(event.target.value as DeploymentStatus | "all")
              }
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
        </FilterBar>
      </TableToolbar>

      <div className="overflow-hidden rounded-3xl border border-zinc-800 bg-zinc-950 shadow-sm shadow-black/20">
        <div className="overflow-x-auto">
          <Table>
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead>Задание</TableHead>
                <TableHead>Статус</TableHead>
                <TableHead>Окружение</TableHead>
                <TableHead>Прогресс</TableHead>
                <TableHead>Запуск</TableHead>
                <TableHead>Инициатор</TableHead>
                <TableHead className="text-right">Действие</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {loading ? (
                <TableRow>
                  <TableCell colSpan={7} className="py-10 text-center text-zinc-500">
                    Загрузка заданий...
                  </TableCell>
                </TableRow>
              ) : filteredJobs.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={7} className="p-6">
                    <EmptyState
                      title="Задания не найдены"
                      description="Попробуйте расширить поиск или сменить фильтр окружения."
                    />
                  </TableCell>
                </TableRow>
              ) : (
                filteredJobs.map((job) => {
                  const meta = getDeploymentStatusMeta(job.status);

                  return (
                    <TableRow key={job.id}>
                      <TableCell>
                        <div>
                          <p className="font-medium text-zinc-100">{job.service}</p>
                          <p className="text-xs text-zinc-500">
                            {job.id} - {job.version}
                          </p>
                        </div>
                      </TableCell>
                      <TableCell>
                        <ToneBadge tone={meta.tone}>{meta.label}</ToneBadge>
                      </TableCell>
                      <TableCell className="uppercase text-zinc-600">
                        {job.environment}
                      </TableCell>
                      <TableCell className="text-zinc-600">
                        {job.completedCount}/{job.targetCount} целей
                      </TableCell>
                      <TableCell className="text-zinc-500">
                        {formatRelativeLabel(job.startedAt)}
                      </TableCell>
                      <TableCell className="text-zinc-600">{job.initiatedBy}</TableCell>
                      <TableCell className="text-right">
                        <Button type="button" variant="ghost" size="sm">
                          Детали
                        </Button>
                      </TableCell>
                    </TableRow>
                  );
                })
              )}
            </TableBody>
          </Table>
        </div>
      </div>
    </div>
  );
}

