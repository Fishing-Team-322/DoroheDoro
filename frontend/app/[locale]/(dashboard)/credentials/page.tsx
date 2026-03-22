import { redirect } from "next/navigation";
import { getLocaleFromParams, withLocalePath } from "@/src/shared/lib/i18n";

export default async function CredentialsPage({
  params,
}: {
  params: Promise<{ locale: string }>;
}) {
  const locale = getLocaleFromParams(await params);
  redirect(withLocalePath(locale, "/infrastructure?tab=access"));
}
