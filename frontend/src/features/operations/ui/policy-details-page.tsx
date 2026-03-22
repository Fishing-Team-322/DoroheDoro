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

const copyByLocale = {
  en: {
    page: {
      title: "Policy Details",
      description:
        "Inspect the policy payload exposed by the public HTTP API before starting a deployment.",
      breadcrumbs: "Policies",
      action: "Create Deployment",
    },
    loading: "Loading policy details...",
    summary: {
      title: "Summary",
      description: "`GET /api/v1/policies/{id}`",
      fields: {
        id: "ID",
        name: "Name",
        revision: "Revision",
        description: "Description",
        targets: "Targets",
      },
      noTargets: "No targets",
    },
    params: {
      title: "Params",
      description: "Structured params exposed by the public response.",
      empty:
        "The current public response does not expose policy params.",
    },
    raw: {
      title: "Raw Policy Body",
      description:
        "Useful when the policy body is delivered as JSON text.",
      empty: "No raw policy body was returned.",
    },
  },
  ru: {
    page: {
      title: "Детали политики",
      description:
        "Посмотрите payload политики, который отдает публичный HTTP API, перед запуском раскатки.",
      breadcrumbs: "Политики",
      action: "Создать раскатку",
    },
    loading: "Загрузка деталей политики...",
    summary: {
      title: "Сводка",
      description: "`GET /api/v1/policies/{id}`",
      fields: {
        id: "ID",
        name: "Имя",
        revision: "Ревизия",
        description: "Описание",
        targets: "Таргеты",
      },
      noTargets: "Нет таргетов",
    },
    params: {
      title: "Параметры",
      description: "Структурированные параметры из публичного ответа.",
      empty:
        "Текущий публичный ответ не содержит policy params.",
    },
    raw: {
      title: "Сырое тело политики",
      description:
        "Полезно, когда тело политики приходит как JSON-текст.",
      empty: "Сырое тело политики не было возвращено.",
    },
  },
} as const;

export function PolicyDetailsPage({ id }: { id: string }) {
  const { locale } = useI18n();
  const copy = copyByLocale[locale];
  const policyQuery = useApiQuery({
    queryFn: (signal) => getPolicy(id, signal),
    deps: [id],
  });

  return (
    <PageStack>
      <PageHeader
        title={copy.page.title}
        description={copy.page.description}
        breadcrumbs={[
          {
            label: copy.page.breadcrumbs,
            href: withLocalePath(locale, "/policies"),
          },
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
              {copy.page.action}
            </Button>
          </Link>
        }
      />

      {policyQuery.isLoading && !policyQuery.data ? (
        <LoadingState label={copy.loading} />
      ) : policyQuery.error && !policyQuery.data ? (
        <ErrorState error={policyQuery.error} retry={() => void policyQuery.refetch()} />
      ) : (
        <PageStack>
          <SectionCard
            title={copy.summary.title}
            description={copy.summary.description}
          >
            <div className="space-y-4">
              <DetailGrid
                items={[
                  {
                    label: copy.summary.fields.id,
                    value: formatMaybeValue(policyQuery.data?.id, locale),
                  },
                  {
                    label: copy.summary.fields.name,
                    value: formatMaybeValue(policyQuery.data?.name, locale),
                  },
                  {
                    label: copy.summary.fields.revision,
                    value: formatMaybeValue(policyQuery.data?.revision, locale),
                  },
                  {
                    label: copy.summary.fields.description,
                    value: formatMaybeValue(policyQuery.data?.description, locale),
                  },
                  {
                    label: copy.summary.fields.targets,
                    value: (
                      <TokenList
                        items={policyQuery.data?.targets ?? []}
                        emptyLabel={copy.summary.noTargets}
                      />
                    ),
                  },
                ]}
              />
              <RequestMetaLine meta={policyQuery.meta} />
            </div>
          </SectionCard>

          <SectionCard
            title={copy.params.title}
            description={copy.params.description}
          >
            <JsonPreview
              value={policyQuery.data?.params}
              emptyLabel={copy.params.empty}
            />
          </SectionCard>

          <SectionCard
            title={copy.raw.title}
            description={copy.raw.description}
          >
            <JsonPreview
              value={policyQuery.data?.body ?? policyQuery.data?.raw}
              emptyLabel={copy.raw.empty}
            />
          </SectionCard>
        </PageStack>
      )}
    </PageStack>
  );
}
