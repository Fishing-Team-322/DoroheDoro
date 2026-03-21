"use client";

import Link from "next/link";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
import { TableCell, TableRow } from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { getAgent, getAgentDiagnostics, searchLogs } from "../api";
import { useApiQuery } from "../model";
import {
  DataTable,
  DetailGrid,
  ErrorState,
  JsonPreview,
  LabelMap,
  LoadingState,
  PageStack,
  RequestMetaLine,
  SectionCard,
  StatusBadge,
  formatDateTime,
  formatMaybeValue,
  formatRelativeTime,
} from "./operations-ui";

export function AgentDetailsPage({ id }: { id: string }) {
  const { locale } = useI18n();
  const agentQuery = useApiQuery({
    queryFn: (signal) => getAgent(id, signal),
    deps: [id],
  });
  const diagnosticsQuery = useApiQuery({
    queryFn: (signal) => getAgentDiagnostics(id, signal),
    deps: [id],
  });
  const logsQuery = useApiQuery({
    queryFn: (signal) => searchLogs({ agentId: id, limit: 10 }, signal),
    deps: [id],
  });

  return (
    <PageStack>
      <PageHeader
        title="Agent Details"
        description="Review the latest summary, diagnostics snapshot, and optional related logs for a single agent."
        breadcrumbs={[
          { label: "Agents", href: withLocalePath(locale, "/agents") },
          { label: id },
        ]}
      />

      {agentQuery.isLoading && !agentQuery.data ? (
        <LoadingState label="Loading agent summary..." />
      ) : agentQuery.error && !agentQuery.data ? (
        <ErrorState error={agentQuery.error} retry={() => void agentQuery.refetch()} />
      ) : (
        <PageStack>
          <SectionCard title="Summary" description="`GET /api/v1/agents/{id}`">
            <div className="space-y-4">
              <DetailGrid
                items={[
                  { label: "Agent ID", value: formatMaybeValue(agentQuery.data?.id) },
                  { label: "Host", value: formatMaybeValue(agentQuery.data?.host) },
                  {
                    label: "Status",
                    value: <StatusBadge value={agentQuery.data?.status} />,
                  },
                  {
                    label: "Policy ID",
                    value: formatMaybeValue(agentQuery.data?.policyId),
                  },
                  {
                    label: "Last seen",
                    value: (
                      <span title={formatDateTime(agentQuery.data?.lastSeenAt)}>
                        {formatRelativeTime(agentQuery.data?.lastSeenAt)}
                      </span>
                    ),
                  },
                  {
                    label: "Labels",
                    value: <LabelMap labels={agentQuery.data?.labels} />,
                  },
                ]}
              />
              <RequestMetaLine meta={agentQuery.meta} />
            </div>
          </SectionCard>

          <SectionCard
            title="Diagnostics"
            description="`GET /api/v1/agents/{id}/diagnostics`"
          >
            {diagnosticsQuery.isLoading && !diagnosticsQuery.data ? (
              <LoadingState compact label="Loading diagnostics..." />
            ) : diagnosticsQuery.error && !diagnosticsQuery.data ? (
              <ErrorState
                title="Diagnostics request failed"
                error={diagnosticsQuery.error}
                retry={() => void diagnosticsQuery.refetch()}
              />
            ) : (
              <div className="space-y-4">
                <DetailGrid
                  items={[
                    {
                      label: "Diagnostics status",
                      value: <StatusBadge value={diagnosticsQuery.data?.status} />,
                    },
                    {
                      label: "Collected at",
                      value: formatDateTime(diagnosticsQuery.data?.collectedAt),
                    },
                    {
                      label: "Checks",
                      value: String(diagnosticsQuery.data?.checks.length ?? 0),
                    },
                  ]}
                />

                <DataTable
                  columns={["Check", "Status", "Message"]}
                  isEmpty={(diagnosticsQuery.data?.checks.length ?? 0) === 0}
                  rows={(diagnosticsQuery.data?.checks ?? []).map((check) => (
                    <TableRow key={`${check.name}-${check.status}`}>
                      <TableCell className="font-medium text-[color:var(--foreground)]">
                        {check.name}
                      </TableCell>
                      <TableCell>
                        <StatusBadge value={check.status} />
                      </TableCell>
                      <TableCell>{formatMaybeValue(check.message)}</TableCell>
                    </TableRow>
                  ))}
                  emptyTitle="No diagnostic checks were returned."
                />

                <JsonPreview
                  value={diagnosticsQuery.data?.payload}
                  emptyLabel="The diagnostics payload is empty."
                />
                <RequestMetaLine meta={diagnosticsQuery.meta} />
              </div>
            )}
          </SectionCard>

          <SectionCard
            title="Related Logs"
            description="Optional lookup through `POST /api/v1/logs/search` using `agent_id`."
          >
            {logsQuery.isLoading && !logsQuery.data ? (
              <LoadingState compact label="Loading related logs..." />
            ) : logsQuery.error && !logsQuery.data ? (
              <ErrorState
                title="Related logs request failed"
                error={logsQuery.error}
                retry={() => void logsQuery.refetch()}
              />
            ) : (
              <div className="space-y-4">
                <DataTable
                  columns={["Timestamp", "Severity", "Service", "Message", "Host"]}
                  isEmpty={(logsQuery.data?.items.length ?? 0) === 0}
                  rows={(logsQuery.data?.items ?? []).map((log) => (
                    <TableRow key={`${log.timestamp}-${log.message}`}>
                      <TableCell>{formatDateTime(log.timestamp)}</TableCell>
                      <TableCell>
                        <StatusBadge value={log.severity} />
                      </TableCell>
                      <TableCell>{formatMaybeValue(log.service)}</TableCell>
                      <TableCell className="max-w-2xl text-[color:var(--foreground)]">
                        {log.message}
                      </TableCell>
                      <TableCell>{formatMaybeValue(log.host)}</TableCell>
                    </TableRow>
                  ))}
                  emptyTitle="No related logs were returned for this agent."
                />
                <div className="flex justify-end">
                  <Link
                    href={withLocalePath(
                      locale,
                      `/logs?agent_id=${encodeURIComponent(id)}`
                    )}
                    className="text-sm text-[color:var(--muted-foreground)] transition-colors hover:text-[color:var(--foreground)]"
                  >
                    Open in Logs Explorer
                  </Link>
                </div>
                <RequestMetaLine meta={logsQuery.meta} />
              </div>
            )}
          </SectionCard>
        </PageStack>
      )}
    </PageStack>
  );
}
