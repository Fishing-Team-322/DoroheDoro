"use client";

import { DataTable } from "@/src/shared/ui";
import { customerColumns, customers } from "../model/customer-table";

export function TablePage() {
  return (
    <main className="min-h-screen bg-zinc-50 p-6 md:p-10">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-4">
        <header>
          <h1 className="text-2xl font-bold text-zinc-900">
            Демо таблицы клиентов
          </h1>
          <p className="mt-1 text-sm text-zinc-600">
            Демонстрационная таблица с сортировкой, пагинацией, фильтрацией и пользовательским рендерингом ячеек.
          </p>
        </header>

        <DataTable
          columns={customerColumns}
          data={customers}
          searchPlaceholder="Поиск по компании, менеджеру или email..."
          emptyMessage="По текущим фильтрам клиентов не найдено"
          selectColumnFilters={[
            {
              columnId: "status",
              label: "Статус",
              allOptionLabel: "Все статусы",
              options: [
                { label: "Активен", value: "Active" },
                { label: "Пробный", value: "Trial" },
                { label: "Приостановлен", value: "Suspended" },
              ],
            },
            {
              columnId: "plan",
              label: "Тариф",
              allOptionLabel: "Все тарифы",
              options: [
                { label: "Бесплатный", value: "Free" },
                { label: "Профессиональный", value: "Pro" },
                { label: "Корпоративный", value: "Enterprise" },
              ],
            },
          ]}
        />
      </div>
    </main>
  );
}
