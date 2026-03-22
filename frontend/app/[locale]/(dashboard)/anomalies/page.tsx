import { redirect } from "next/navigation";
import { getLocaleFromParams, withLocalePath } from "@/src/shared/lib/i18n";

export default async function DashboardAnomaliesRoute({
  params,
}: {
  params: Promise<{ locale: string }>;
}) {
  const locale = getLocaleFromParams(await params);
  redirect(withLocalePath(locale, "/security?tab=anomalies"));
}
