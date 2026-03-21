"use client";

import { PageHeader } from "@/src/widgets/dashboard-layout";
import { PageStack, SectionCard, UnavailableState } from "./operations-ui";

export function UnavailablePage({
  title,
  description,
}: {
  title: string;
  description: string;
}) {
  return (
    <PageStack>
      <PageHeader title={title} description={description} />
      <SectionCard title="Unavailable" description="This page is ready for a future public HTTP integration.">
        <UnavailableState title="Unavailable" description={description} />
      </SectionCard>
    </PageStack>
  );
}
