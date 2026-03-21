"use client";

import Link from "next/link";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
import { Button } from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { getPolicy } from "../api";
import { useApiQuery } from "../model";
import {
  DetailGrid,
  ErrorState,
  JsonPreview,
  LoadingState,
  PageStack,
  RequestMetaLine,
  SectionCard,
  TokenList,
  formatMaybeValue,
} from "./operations-ui";

export function PolicyDetailsPage({ id }: { id: string }) {
  const { locale } = useI18n();
  const policyQuery = useApiQuery({
    queryFn: (signal) => getPolicy(id, signal),
    deps: [id],
  });

  return (
    <PageStack>
      <PageHeader
        title="Policy Details"
        description="Inspect the policy payload exposed by the public HTTP API before starting a deployment."
        breadcrumbs={[
          { label: "Policies", href: withLocalePath(locale, "/policies") },
          { label: id },
        ]}
        action={
          <Link
            href={withLocalePath(
              locale,
              `/deployments/new?policy_id=${encodeURIComponent(id)}`
            )}
          >
            <Button size="sm" className="h-10 px-4">
              Create Deployment
            </Button>
          </Link>
        }
      />

      {policyQuery.isLoading && !policyQuery.data ? (
        <LoadingState label="Loading policy details..." />
      ) : policyQuery.error && !policyQuery.data ? (
        <ErrorState error={policyQuery.error} retry={() => void policyQuery.refetch()} />
      ) : (
        <PageStack>
          <SectionCard title="Summary" description="`GET /api/v1/policies/{id}`">
            <div className="space-y-4">
              <DetailGrid
                items={[
                  { label: "ID", value: formatMaybeValue(policyQuery.data?.id) },
                  { label: "Name", value: formatMaybeValue(policyQuery.data?.name) },
                  {
                    label: "Revision",
                    value: formatMaybeValue(policyQuery.data?.revision),
                  },
                  {
                    label: "Description",
                    value: formatMaybeValue(policyQuery.data?.description),
                  },
                  {
                    label: "Targets",
                    value: (
                      <TokenList
                        items={policyQuery.data?.targets ?? []}
                        emptyLabel="No targets"
                      />
                    ),
                  },
                ]}
              />
              <RequestMetaLine meta={policyQuery.meta} />
            </div>
          </SectionCard>

          <SectionCard title="Params" description="Structured params exposed by the public response.">
            <JsonPreview
              value={policyQuery.data?.params}
              emptyLabel="The current public response does not expose policy params."
            />
          </SectionCard>

          <SectionCard title="Raw Policy Body" description="Useful when the policy body is delivered as JSON text.">
            <JsonPreview
              value={policyQuery.data?.body ?? policyQuery.data?.raw}
              emptyLabel="No raw policy body was returned."
            />
          </SectionCard>
        </PageStack>
      )}
    </PageStack>
  );
}
