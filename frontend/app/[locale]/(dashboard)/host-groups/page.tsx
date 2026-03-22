import { UnavailablePage } from "@/src/features/operations";
import { getLocaleFromParams } from "@/src/shared/lib/i18n";

export default async function DashboardHostGroupsRoute({
  params,
}: {
  params: Promise<{ locale: string }>;
}) {
  const locale = getLocaleFromParams(await params);
  const title = locale === "ru" ? "Группы хостов" : "Host Groups";
  const description =
    locale === "ru"
      ? "Управление группами хостов пока недоступно через текущий публичный HTTP API."
      : "Host group management is not available through the current public HTTP API yet.";

  return (
    <UnavailablePage
      title={title}
      description={description}
    />
  );
}
