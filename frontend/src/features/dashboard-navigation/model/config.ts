export type DashboardNavItemKey =
  | "overview"
  | "infrastructure"
  | "security"
  | "operations"
  | "integrations"
  | "audit"
  | "profile";

export type DashboardNavItem = {
  key: DashboardNavItemKey;
  href: string;
  fallbackLabel: string;
  matchPaths?: string[];
};

export const dashboardNavigation: DashboardNavItem[] = [
  {
    key: "overview",
    href: "/overview",
    fallbackLabel: "Overview",
  },
  {
    key: "infrastructure",
    href: "/infrastructure",
    fallbackLabel: "Infrastructure",
    matchPaths: ["/infrastructure", "/system", "/inventory", "/credentials", "/agents"],
  },
  {
    key: "security",
    href: "/security",
    fallbackLabel: "Security",
    matchPaths: ["/security", "/policies", "/alerts", "/anomalies"],
  },
  {
    key: "operations",
    href: "/operations",
    fallbackLabel: "Operations",
    matchPaths: ["/operations", "/deployments", "/logs"],
  },
  {
    key: "integrations",
    href: "/integrations",
    fallbackLabel: "Integrations",
  },
  {
    key: "audit",
    href: "/audit",
    fallbackLabel: "Audit",
  },
  {
    key: "profile",
    href: "/profile",
    fallbackLabel: "Profile",
  },
] as const;
