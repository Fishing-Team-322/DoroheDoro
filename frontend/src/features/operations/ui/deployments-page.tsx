"use client";

import { useState } from "react";
import Link from "next/link";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
import { Button, TableCell, TableRow } from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { listDeployments } from "../api";
import { useApiQuery } from "../model";
import {
  CursorPagination,
  DataTable,
  ErrorState,
  LoadingState,
  PageStack,
  RequestMetaLine,
  SectionCard,
  StatusBadge,
  formatDateTime,
  formatNumber,
  formatParamsSummary,
} from "./operations-ui";

export function DeploymentsPage() {
  const { locale } = useI18n();
  const [cursor, setCursor] = useState<string>();
  const [cursorHistory, setCursorHistory] = useState<string[]>([]);

  const deploymentsQuery = useApiQuery({
    queryFn: (signal) => listDeployments({ signal, cursor }),
    deps: [cursor],
  });

  const items = deploymentsQuery.data?.items ?? [];

  return (
    <PageStack>
      <PageHeader
        title="Deployments"
        description="Track deployment jobs, inspect their current status, and drill down into per-job details."
        action={
          <Link href={withLocalePath(locale, "/deployments/new")}>
            <Button size="sm" className="h-10 px-4">
              New Deployment
            </Button>
          </Link>
        }
      />

      <SectionCard
        title="Jobs"
        description="Public source: `GET /api/v1/deployments`"
      >
        {deploymentsQuery.isLoading && !deploymentsQuery.data ? (
          <LoadingState label="Loading deployments..." />
        ) : deploymentsQuery.error && !deploymentsQuery.data ? (
          <ErrorState error={deploymentsQuery.error} retry={() => void deploymentsQuery.refetch()} />
        ) : (
          <div className="space-y-4">
            <DataTable
              columns={[
                "Deployment ID",
                "Policy ID",
                "Status",
                "Created",
                "Targets",
                "Params",
                "Actions",
              ]}
              isEmpty={items.length === 0}
              rows={items.map((deployment) => (
                <TableRow key={deployment.id}>
                  <TableCell className="font-mono text-xs text-[color:var(--foreground)]">
                    {deployment.id}
                  </TableCell>
                  <TableCell>{deployment.policyId ?? "n/a"}</TableCell>
                  <TableCell>
                    <StatusBadge value={deployment.status} />
                  </TableCell>
                  <TableCell>{formatDateTime(deployment.createdAt)}</TableCell>
                  <TableCell>
                    {formatNumber(
                      deployment.totalTargets ?? deployment.agentIds.length
                    )}
                  </TableCell>
                  <TableCell className="max-w-md text-[color:var(--muted-foreground)]">
                    {formatParamsSummary(deployment.params)}
                  </TableCell>
                  <TableCell>
                    <Link
                      href={withLocalePath(
                        locale,
                        `/deployments/${deployment.id}`
                      )}
                    >
                      <Button variant="outline" size="sm" className="h-9 px-3">
                        Details
                      </Button>
                    </Link>
                  </TableCell>
                </TableRow>
              ))}
              emptyTitle="No deployments were returned."
              emptyDescription="The deployments endpoint returned an empty collection."
            />

            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <RequestMetaLine meta={deploymentsQuery.meta} />
              <CursorPagination
                hasPrevious={cursorHistory.length > 0}
                hasNext={Boolean(deploymentsQuery.data?.nextCursor)}
                onPrevious={() => {
                  setCursorHistory((current) => {
                    if (current.length === 0) {
                      setCursor(undefined);
                      return current;
                    }

                    const nextHistory = current.slice(0, -1);
                    setCursor(nextHistory[nextHistory.length - 1] || undefined);
                    return nextHistory;
                  });
                }}
                onNext={() => {
                  const nextCursor = deploymentsQuery.data?.nextCursor;
                  if (!nextCursor) {
                    return;
                  }

                  setCursorHistory((current) => [...current, cursor ?? ""]);
                  setCursor(nextCursor);
                }}
              />
            </div>
          </div>
        )}
      </SectionCard>
    </PageStack>
  );
}
