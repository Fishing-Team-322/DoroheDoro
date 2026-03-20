import { DemoPage } from "@/src/page-modules/demo";
import { getLocaleFromParams } from "@/src/shared/lib/i18n";

type DemoPageRouteProps = {
  params: Promise<{ locale?: string }>;
};

export default async function DemoRoutePage({ params }: DemoPageRouteProps) {
  const locale = getLocaleFromParams(await params);

  return <DemoPage locale={locale} />;
}
