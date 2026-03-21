"use client";

import { useDeferredValue, useState } from "react";
import Link from "next/link";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
import { Button, TableCell, TableRow } from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { listPolicies } from "../api";
import { useApiQuery } from "../model";
import {
  DataTable,
  ErrorState,
  LoadingState,
  PageStack,
  RequestMetaLine,
  SearchField,
  SectionCard,
  TokenList,
  formatMaybeValue,
} from "./operations-ui";

export function PoliciesPage() {
  const { locale } = useI18n();
  const [search, setSearch] = useState("");
  const deferredSearch = useDeferredValue(search.trim().toLowerCase());

  const policiesQuery = useApiQuery({
    queryFn: (signal) => listPolicies({ signal }),
    deps: [],
  });

  const filteredItems =
    policiesQuery.data?.items.filter((policy) => {
      if (!deferredSearch) {
        return true;
      }

      return [policy.name, policy.id]
        .filter(Boolean)
        .some((value) => value?.toLowerCase().includes(deferredSearch));
    }) ?? [];

  return (
    <PageStack>
      <PageHeader
        title="Policies"
        description="Browse the policy catalog, search by id or name, and open a policy to prepare a deployment from it."
        action={
          <Link href={withLocalePath(locale, "/deployments/new")}>
            <Button size="sm" className="h-10 px-4">
              Create Deployment
            </Button>
          </Link>
        }
      />

      <SectionCard
        title="Catalog"
        description="Public source: `GET /api/v1/policies`"
      >
        <div className="space-y-4">
          <SearchField
            label="Search"
            value={search}
            onChange={setSearch}
            placeholder="Search by policy name or id"
          />

          {policiesQuery.isLoading && !policiesQuery.data ? (
            <LoadingState label="Loading policies..." />
          ) : policiesQuery.error && !policiesQuery.data ? (
            <ErrorState error={policiesQuery.error} retry={() => void policiesQuery.refetch()} />
          ) : (
            <div className="space-y-4">
              <DataTable
                columns={[
                  "Name",
                  "ID",
                  "Revision",
                  "Targets",
                  "Description",
                  "Actions",
                ]}
                rows={filteredItems.map((policy) => (
                  <TableRow key={policy.id}>
                    <TableCell className="font-medium text-[color:var(--foreground)]">
                      {policy.name}
                    </TableCell>
                    <TableCell className="font-mono text-xs text-[color:var(--muted-foreground)]">
                      {policy.id}
                    </TableCell>
                    <TableCell>{formatMaybeValue(policy.revision)}</TableCell>
                    <TableCell>
                      <TokenList items={policy.targets} emptyLabel="No targets" />
                    </TableCell>
                    <TableCell className="max-w-md text-[color:var(--muted-foreground)]">
                      {formatMaybeValue(policy.description)}
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-wrap gap-2">
                        <Link
                          href={withLocalePath(locale, `/policies/${policy.id}`)}
                        >
                          <Button variant="outline" size="sm" className="h-9 px-3">
                            Details
                          </Button>
                        </Link>
                        <Link
                          href={withLocalePath(
                            locale,
                            `/deployments/new?policy_id=${encodeURIComponent(policy.id)}`
                          )}
                        >
                          <Button variant="ghost" size="sm" className="h-9 px-3">
                            Create Deployment
                          </Button>
                        </Link>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
                emptyTitle={
                  policiesQuery.data?.items.length
                    ? "No policies match the current search."
                    : "No policies were returned."
                }
                isEmpty={filteredItems.length === 0}
                emptyDescription={
                  policiesQuery.data?.items.length
                    ? "Try a different policy name or identifier."
                    : "The public policies endpoint returned an empty collection."
                }
              />

              <RequestMetaLine meta={policiesQuery.meta} />
            </div>
          )}
        </div>
      </SectionCard>
    </PageStack>
  );
}
