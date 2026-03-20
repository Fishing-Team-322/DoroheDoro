import { LoginPage } from "@/src/page-modules/login";
import { getLocaleFromParams } from "@/src/shared/lib/i18n";

type LoginRouteProps = {
  params: Promise<{ locale?: string }>;
};

export default async function LoginRoute({ params }: LoginRouteProps) {
  const locale = getLocaleFromParams(await params);

  return <LoginPage locale={locale} />;
}
