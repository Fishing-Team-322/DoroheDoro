import { LocaleEntryRedirect } from "@/src/features/auth/ui/locale-entry-redirect";
import { getLocaleFromParams } from "@/src/shared/lib/i18n";

type HomePageRouteProps = {
  params: Promise<{ locale?: string }>;
};

export default async function LocaleHomePage({ params }: HomePageRouteProps) {
  const locale = getLocaleFromParams(await params);

  return <LocaleEntryRedirect locale={locale} />;
}
