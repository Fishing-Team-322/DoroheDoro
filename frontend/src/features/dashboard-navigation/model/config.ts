export type DashboardNavItemKey =
  | "overview"
  | "inventory"
  | "policies"
  | "profile";

export type DashboardNavItem = {
  key: DashboardNavItemKey;
  href: string;
};

export const dashboardNavigation: DashboardNavItem[] = [
  { key: "overview", href: "/overview" },
  { key: "inventory", href: "/inventory" },
  { key: "policies", href: "/policies" },
  { key: "profile", href: "/profile" },
] as const;
