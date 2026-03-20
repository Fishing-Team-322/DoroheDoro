export type DashboardNavItem = {
  label: string;
  href: string;
  description: string;
};

export const dashboardNavigation: DashboardNavItem[] = [
  {
    label: "Overview",
    href: "/overview",
    description: "High-level system summary",
  },
  {
    label: "Inventory",
    href: "/inventory",
    description: "Hosts and infrastructure resources",
  },
  {
    label: "Policies",
    href: "/policies",
    description: "Rules, controls, and policy checks",
  },
  {
    label: "Profile",
    href: "/profile",
    description: "Current account and session settings",
  },
] as const;
