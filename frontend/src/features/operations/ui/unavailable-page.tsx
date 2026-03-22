"use client";

import { useI18n } from "@/src/shared/lib/i18n";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { PageStack, SectionCard, UnavailableState } from "./operations-ui";

const copyByLocale = {
  en: {
    title: "Unavailable",
    description: "This page is ready for a future public HTTP integration.",
  },
  ru: {
    title: "Недоступно",
    description: "Эта страница готова для будущей интеграции через публичный HTTP.",
  },
} as const;

export function UnavailablePage({
  title,
  description,
}: {
  title: string;
  description: string;
}) {
  const { locale } = useI18n();
  const copy = copyByLocale[locale];

  return (
    <PageStack>
      <PageHeader title={title} description={description} />
      <SectionCard title={copy.title} description={copy.description}>
        <UnavailableState title={copy.title} description={description} />
      </SectionCard>
    </PageStack>
  );
}
