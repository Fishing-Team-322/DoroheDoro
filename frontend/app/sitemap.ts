import type { MetadataRoute } from "next";
import { locales } from "@/src/shared/config";

const siteUrl = (
  process.env.NEXT_PUBLIC_SITE_URL ?? "https://example.com"
).replace(/\/$/, "");
const appRoutes = [
  "",
  "/login",
  "/overview",
  "/system",
  "/policies",
  "/deployments",
  "/deployments/new",
  "/agents",
  "/logs",
  "/logs/live",
  "/hosts",
  "/host-groups",
  "/credentials",
  "/profile",
];

export default function sitemap(): MetadataRoute.Sitemap {
  const now = new Date();

  return [
    {
      url: `${siteUrl}/`,
      lastModified: now,
      changeFrequency: "weekly",
      priority: 1,
    },
    ...locales.flatMap((locale) =>
      appRoutes.map((route) => ({
        url: `${siteUrl}/${locale}${route}`,
        lastModified: now,
        changeFrequency: "weekly" as const,
        priority: route === "" ? 0.9 : 0.7,
      }))
    ),
  ];
}

