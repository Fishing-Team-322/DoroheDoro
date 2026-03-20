import type { DataTableColumn } from "@/src/shared/ui";

export type CustomerRow = {
  id: string;
  company: string;
  manager: string;
  email: string;
  country: string;
  plan: "Free" | "Pro" | "Enterprise";
  status: "Active" | "Trial" | "Suspended";
  monthlyRevenue: number;
  lastInvoiceAt: string;
};

export const customers: CustomerRow[] = [
  {
    id: "C-1001",
    company: "Northwind Logistics",
    manager: "Elena Petrova",
    email: "elena@northwind.io",
    country: "Германия",
    plan: "Enterprise",
    status: "Active",
    monthlyRevenue: 18200,
    lastInvoiceAt: "2026-02-28",
  },
  {
    id: "C-1002",
    company: "Atlas Retail Group",
    manager: "Mikhail Smirnov",
    email: "m.smirnov@atlasretail.com",
    country: "Польша",
    plan: "Pro",
    status: "Trial",
    monthlyRevenue: 4200,
    lastInvoiceAt: "2026-02-22",
  },
  {
    id: "C-1003",
    company: "Sunrise Health",
    manager: "Anna Volkova",
    email: "anna.volkova@sunrise.health",
    country: "Чехия",
    plan: "Enterprise",
    status: "Active",
    monthlyRevenue: 23600,
    lastInvoiceAt: "2026-03-01",
  },
  {
    id: "C-1004",
    company: "Blue Peak Studio",
    manager: "Oleg Ivanov",
    email: "oleg@bluepeak.studio",
    country: "Сербия",
    plan: "Free",
    status: "Suspended",
    monthlyRevenue: 0,
    lastInvoiceAt: "2025-12-17",
  },
  {
    id: "C-1005",
    company: "Delta Mobility",
    manager: "Irina Sokolova",
    email: "irina@deltamobility.ai",
    country: "Нидерланды",
    plan: "Pro",
    status: "Active",
    monthlyRevenue: 7900,
    lastInvoiceAt: "2026-02-20",
  },
  {
    id: "C-1006",
    company: "Vector Cloud",
    manager: "Roman Belov",
    email: "roman@vectorcloud.dev",
    country: "Литва",
    plan: "Pro",
    status: "Trial",
    monthlyRevenue: 3600,
    lastInvoiceAt: "2026-02-15",
  },
  {
    id: "C-1007",
    company: "Orion Finance",
    manager: "Daria Kozlova",
    email: "daria.kozlova@orionfin.com",
    country: "Швеция",
    plan: "Enterprise",
    status: "Active",
    monthlyRevenue: 31400,
    lastInvoiceAt: "2026-03-02",
  },
  {
    id: "C-1008",
    company: "Astra Education",
    manager: "Vladimir Nosov",
    email: "v.nosov@astraedu.org",
    country: "Испания",
    plan: "Free",
    status: "Trial",
    monthlyRevenue: 0,
    lastInvoiceAt: "2026-01-30",
  },
];

const statusStyles: Record<CustomerRow["status"], string> = {
  Active: "bg-emerald-100 text-emerald-700",
  Trial: "bg-amber-100 text-amber-700",
  Suspended: "bg-rose-100 text-rose-700",
};

const statusLabels: Record<CustomerRow["status"], string> = {
  Active: "Активен",
  Trial: "Пробный",
  Suspended: "Приостановлен",
};

const planLabels: Record<CustomerRow["plan"], string> = {
  Free: "Бесплатный",
  Pro: "Профессиональный",
  Enterprise: "Корпоративный",
};

export const customerColumns: DataTableColumn<CustomerRow>[] = [
  {
    accessorKey: "company",
    header: "Компания",
    enableSorting: true,
    cell: (row) => (
      <div>
        <p className="font-medium text-zinc-900">{row.company}</p>
        <p className="text-xs text-zinc-500">{row.email}</p>
      </div>
    ),
  },
  {
    accessorKey: "manager",
    header: "Менеджер",
    enableSorting: true,
  },
  {
    accessorKey: "country",
    header: "Страна",
    enableSorting: true,
  },
  {
    accessorKey: "plan",
    header: "Тариф",
    enableSorting: true,
    cell: (row) => planLabels[row.plan],
  },
  {
    accessorKey: "status",
    header: "Статус",
    enableSorting: true,
    cell: (row) => (
      <span
        className={`rounded-full px-2 py-0.5 text-xs font-medium ${statusStyles[row.status]}`}
      >
        {statusLabels[row.status]}
      </span>
    ),
  },
  {
    accessorKey: "monthlyRevenue",
    header: "MRR",
    enableSorting: true,
    cell: (row) =>
      new Intl.NumberFormat("ru-RU", {
        style: "currency",
        currency: "USD",
        maximumFractionDigits: 0,
      }).format(row.monthlyRevenue),
  },
  {
    accessorKey: "lastInvoiceAt",
    header: "Последний счёт",
    enableSorting: true,
    cell: (row) => new Date(row.lastInvoiceAt).toLocaleDateString("ru-RU"),
  },
];
