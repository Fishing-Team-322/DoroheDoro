"use client";

import { useDeferredValue, useState } from "react";
import Link from "next/link";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
import { Button, TableCell, TableRow } from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { listAgents } from "../api";
import { useApiQuery } from "../model";
import {
  DataTable,
  ErrorState,
  LabelMap,
  LoadingState,
  PageStack,
  RequestMetaLine,
  SearchField,
  SectionCard,
  SelectField,
  StatusBadge,
  formatMaybeValue,
  formatRelativeTime,
} from "./operations-ui";

export function AgentsPage() {
  const { locale } = useI18n();
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const deferredSearch = useDeferredValue(search.trim().toLowerCase());

  const agentsQuery = useApiQuery({
    queryFn: (signal) => listAgents({ signal }),
    deps: [],
  });

  const statuses = Array.from(
    new Set((agentsQuery.data?.items ?? []).map((agent) => agent.status))
  );

  const filteredItems =
    agentsQuery.data?.items.filter((agent) => {
      const matchesSearch =
        !deferredSearch ||
        [agent.id, agent.host, agent.policyId]
          .filter(Boolean)
          .some((value) => value?.toLowerCase().includes(deferredSearch));
      const matchesStatus = !statusFilter || agent.status === statusFilter;
      return matchesSearch && matchesStatus;
    }) ?? [];

  return (
    <PageStack>
      <PageHeader
        title="Agents"
        description="Inspect agent connectivity, labels, policy assignment, and last-seen data exposed by the registry endpoint."
      />

      <SectionCard title="Registry" description="`GET /api/v1/agents`">
        <div className="space-y-4">
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            <SearchField
              label="Search"
              value={search}
              onChange={setSearch}
              placeholder="Search by host, agent id, or policy id"
            />
            <SelectField
              label="Status"
              value={statusFilter}
              onChange={setStatusFilter}
              options={[
                { value: "", label: "All statuses" },
                ...statuses.map((status) => ({
                  value: status,
                  label: status,
                })),
              ]}
            />
          </div>

          {agentsQuery.isLoading && !agentsQuery.data ? (
            <LoadingState label="Loading agents..." />
          ) : agentsQuery.error && !agentsQuery.data ? (
            <ErrorState error={agentsQuery.error} retry={() => void agentsQuery.refetch()} />
          ) : (
            <div className="space-y-4">
              <DataTable
                columns={[
                  "Host",
                  "Agent ID",
                  "Status",
                  "Policy ID",
                  "Last seen",
                  "Labels",
                  "Actions",
                ]}
                rows={filteredItems.map((agent) => (
                  <TableRow key={agent.id}>
                    <TableCell className="font-medium text-[color:var(--foreground)]">
                      {agent.host}
                    </TableCell>
                    <TableCell className="font-mono text-xs text-[color:var(--muted-foreground)]">
                      {agent.id}
                    </TableCell>
                    <TableCell>
                      <StatusBadge value={agent.status} />
                    </TableCell>
                    <TableCell>{formatMaybeValue(agent.policyId)}</TableCell>
                    <TableCell>{formatRelativeTime(agent.lastSeenAt)}</TableCell>
                    <TableCell>
                      <LabelMap labels={agent.labels} />
                    </TableCell>
                    <TableCell>
                      <Link href={withLocalePath(locale, `/agents/${agent.id}`)}>
                        <Button variant="outline" size="sm" className="h-9 px-3">
                          Details
                        </Button>
                      </Link>
                    </TableCell>
                  </TableRow>
                ))}
                emptyTitle={
                  agentsQuery.data?.items.length
                    ? "No agents match the current filters."
                    : "No agents were returned."
                }
                isEmpty={filteredItems.length === 0}
                emptyDescription={
                  agentsQuery.data?.items.length
                    ? "Try removing the search query or status filter."
                    : "The agents endpoint returned an empty list."
                }
              />

              <RequestMetaLine meta={agentsQuery.meta} />
            </div>
          )}
        </div>
      </SectionCard>
    </PageStack>
  );
}
