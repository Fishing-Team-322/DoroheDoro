import { UnavailablePage } from "@/src/features/operations";
import { getLocaleFromParams } from "@/src/shared/lib/i18n";

export default async function DashboardHostsRoute({
  params,
}: {
  params: Promise<{ locale: string }>;
}) {
  const locale = getLocaleFromParams(await params);
  const title = locale === "ru" ? "Хосты" : "Hosts";
  const description =
    locale === "ru"
      ? "Управление хостами пока недоступно через текущий публичный HTTP API."
      : "Hosts management is not available through the current public HTTP API yet.";

  return (
    <UnavailablePage
      title={title}
      description={description}
    />
  );
}
