import { redirect } from "next/navigation";
import { getLocaleFromParams, withLocalePath } from "@/src/shared/lib/i18n";

export default async function DashboardAgentDetailsRoute({
  params,
}: {
  params: Promise<{ locale: string; id: string }>;
}) {
  const resolvedParams = await params;
  const locale = getLocaleFromParams(resolvedParams);

  redirect(withLocalePath(locale, "/infrastructure?tab=agents"));
}
