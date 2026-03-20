import { HomePage } from "@/src/page-modules/home";
import { getLocaleFromParams } from "@/src/shared/lib/i18n";

type HomePageRouteProps = {
  params: Promise<{ locale?: string }>;
};

export default async function LocaleHomePage({ params }: HomePageRouteProps) {
  const locale = getLocaleFromParams(await params);

  return <HomePage locale={locale} />;
}
