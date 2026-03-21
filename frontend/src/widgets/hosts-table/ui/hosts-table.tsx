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
import { environmentValues } from "@/src/shared/constants/dashboard";
import { cn } from "@/src/shared/lib/cn";
import { formatPercent, formatRelativeLabel } from "@/src/shared/lib/dashboard";
import { useI18n } from "@/src/shared/lib/i18n";
import type { Host, HostStatus } from "@/src/shared/types/dashboard";

type HostsTableProps = {
  hosts: Host[];
  loading?: boolean;
  selectedHostId?: string;
  onSelectHost?: (host: Host) => void;
};

const CONTENT_INSET = "px-4 md:px-6";

export function HostsTable({
  hosts,
  loading = false,
  selectedHostId,
  onSelectHost,
}: HostsTableProps) {
  const { dictionary, locale } = useI18n();
  const copy = dictionary.inventory.table;
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState<HostStatus | "all">("all");
  const [environment, setEnvironment] = useState<string>("all");

  const statusOptions = useMemo<Array<{ label: string; value: HostStatus | "all" }>>(
    () => [
      { label: copy.statusOptions.all, value: "all" },
      { label: copy.statusOptions.online, value: "online" },
      { label: copy.statusOptions.offline, value: "offline" },
      { label: copy.statusOptions.degraded, value: "degraded" },
      { label: copy.statusOptions.enrolling, value: "enrolling" },
    ],
    [copy.statusOptions]
  );

  const environmentOptions = useMemo(
    () =>
      environmentValues.map((value) => ({
        value,
        label: dictionary.filters.environment[value],
      })),
    [dictionary.filters.environment]
  );

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
                {copy.title}
              </h3>
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.description}
              </p>
            </div>

            <div className="flex flex-wrap items-center gap-2 lg:justify-end">
              <Button type="button" variant="outline">
                {copy.export}
              </Button>
            </div>
          </div>

          <div className="grid min-w-0 gap-3 md:grid-cols-[minmax(0,2fr)_1fr_1fr]">
            <SearchInput
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder={copy.searchPlaceholder}
            />
            <Select
              value={status}
              onChange={(event) => setStatus(event.target.value as HostStatus | "all")}
              options={statusOptions}
              placeholder={copy.statusPlaceholder}
              selectSize="md"
            />
            <Select
              value={environment}
              onChange={(event) => setEnvironment(event.target.value)}
              options={environmentOptions}
              placeholder={copy.environmentPlaceholder}
              selectSize="md"
            />
          </div>
        </div>
      </div>

      <div className="min-w-0 overflow-x-auto">
        <Table>
          <TableHeader>
            <TableRow className="hover:bg-transparent">
              <TableHead className="px-4 py-4 md:px-6">{copy.headers.host}</TableHead>
              <TableHead className="px-4 py-4 md:px-6">{copy.headers.status}</TableHead>
              <TableHead className="px-4 py-4 md:px-6">{copy.headers.health}</TableHead>
              <TableHead className="px-4 py-4 md:px-6">
                {copy.headers.environment}
              </TableHead>
              <TableHead className="px-4 py-4 md:px-6">{copy.headers.cluster}</TableHead>
              <TableHead className="px-4 py-4 md:px-6">{copy.headers.load}</TableHead>
              <TableHead className="px-4 py-4 md:px-6">{copy.headers.lastSeen}</TableHead>
              <TableHead className="px-4 py-4 text-right md:px-6">
                {copy.headers.action}
              </TableHead>
            </TableRow>
          </TableHeader>

          <TableBody>
            {loading ? (
              <TableRow>
                <TableCell
                  colSpan={8}
                  className="px-4 py-10 text-center text-[color:var(--muted-foreground)] md:px-6"
                >
                  {copy.loading}
                </TableCell>
              </TableRow>
            ) : filteredHosts.length === 0 ? (
              <TableRow>
                <TableCell colSpan={8} className="p-0">
                  <EmptyState
                    variant="flush"
                    title={copy.emptyTitle}
                    description={copy.emptyDescription}
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
                    {formatRelativeLabel(host.lastSeenAt, locale)}
                  </TableCell>

                  <TableCell className="px-4 py-4 text-right md:px-6">
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => onSelectHost?.(host)}
                    >
                      {copy.open}
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
