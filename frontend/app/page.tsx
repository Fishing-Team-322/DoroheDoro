import { redirect } from "next/navigation";
import { defaultLocale } from "@/src/shared/config";

export default function RootPage() {
  redirect(`/${defaultLocale}`);
}

