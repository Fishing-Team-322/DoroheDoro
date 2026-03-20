"use client";

import { useMemo, useState } from "react";
import { Button } from "@/src/shared/ui/button";
import { Input } from "@/src/shared/ui/input";
import { Select } from "@/src/shared/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/src/shared/ui/table";
import type { DataTableColumn } from "@/src/shared/ui/data-table/model/types";

type SelectFilterOption = {
  label: string;
  value: string;
};

type SelectColumnFilter<TData> = {
  columnId: keyof TData & string;
  label: string;
  options: SelectFilterOption[];
  allOptionLabel?: string;
};

type DataTableProps<TData extends { id: string }> = {
  columns: DataTableColumn<TData>[];
  data: TData[];
  isLoading?: boolean;
  emptyMessage?: string;
  searchPlaceholder?: string;
  pageSizeOptions?: number[];
  selectColumnFilters?: SelectColumnFilter<TData>[];
};

export function DataTable<TData extends { id: string }>({
  columns,
  data,
  isLoading = false,
  emptyMessage = "Нет данных для отображения",
  searchPlaceholder = "Поиск по таблице...",
  pageSizeOptions = [5, 10, 20, 50],
  selectColumnFilters = [],
}: DataTableProps<TData>) {
  const [sortBy, setSortBy] = useState<(keyof TData & string) | null>(null);
  const [sortDirection, setSortDirection] = useState<"asc" | "desc">("asc");
  const [globalFilter, setGlobalFilter] = useState("");
  const [columnFilters, setColumnFilters] = useState<Record<string, string>>({});
  const [pageSize, setPageSize] = useState(pageSizeOptions[0] ?? 10);
  const [pageIndex, setPageIndex] = useState(0);

  const filteredRowModel = useMemo(() => {
    const search = globalFilter.trim().toLowerCase();

    return data.filter((row) => {
      const matchesGlobal =
        search.length === 0 ||
        columns.some((column) =>
          String(row[column.accessorKey] ?? "")
            .toLowerCase()
            .includes(search)
        );

      const matchesColumnFilters = selectColumnFilters.every((filter) => {
        const selected = columnFilters[filter.columnId];
        if (!selected) return true;
        return String(row[filter.columnId] ?? "") === selected;
      });

      return matchesGlobal && matchesColumnFilters;
    });
  }, [data, globalFilter, columns, selectColumnFilters, columnFilters]);

  const sortedRowModel = useMemo(() => {
    if (!sortBy) return filteredRowModel;

    return [...filteredRowModel].sort((a, b) => {
      const result = String(a[sortBy] ?? "").localeCompare(
        String(b[sortBy] ?? ""),
        "en",
        { numeric: true }
      );
      return sortDirection === "asc" ? result : -result;
    });
  }, [filteredRowModel, sortBy, sortDirection]);

  const totalPages = Math.ceil(sortedRowModel.length / pageSize);

  const paginationRowModel = useMemo(() => {
    const from = pageIndex * pageSize;
    const to = from + pageSize;
    return sortedRowModel.slice(from, to);
  }, [sortedRowModel, pageIndex, pageSize]);

  const setSort = (columnId: keyof TData & string) => {
    if (sortBy !== columnId) {
      setSortBy(columnId);
      setSortDirection("asc");
      return;
    }

    setSortDirection((current) => (current === "asc" ? "desc" : "asc"));
  };

  const onFilterChange = (columnId: string, value: string) => {
    setPageIndex(0);
    setColumnFilters((current) => ({ ...current, [columnId]: value }));
  };

  return (
    <div className="w-full rounded-xl border border-zinc-800 bg-zinc-950 p-4 shadow-sm shadow-black/20">
      <div className="mb-4 flex flex-col gap-3">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <Input
            value={globalFilter}
            onChange={(event) => {
              setPageIndex(0);
              setGlobalFilter(event.target.value);
            }}
            placeholder={searchPlaceholder}
          />

          <label className="flex items-center gap-2 text-sm text-zinc-600">
            Строк на странице
            <Select
              value={String(pageSize)}
              onChange={(event) => {
                setPageIndex(0);
                setPageSize(Number(event.target.value));
              }}
              className="w-auto"
              options={pageSizeOptions.map((size) => ({
                value: String(size),
                label: String(size),
              }))}
              placeholder="Размер страницы"
              selectSize="md"
            />
          </label>
        </div>

        {selectColumnFilters.length > 0 ? (
          <div className="flex flex-wrap gap-2">
            {selectColumnFilters.map((filter) => (
              <label
                key={filter.columnId}
                className="flex items-center gap-2 rounded-md border border-zinc-200 px-2 py-1.5 text-sm text-zinc-700"
              >
                {filter.label}
                <Select
                  value={columnFilters[filter.columnId] ?? ""}
                  onChange={(event) =>
                    onFilterChange(filter.columnId, event.target.value)
                  }
                  className="h-8"
                  options={[
                    { value: "", label: filter.allOptionLabel ?? "Все" },
                    ...filter.options,
                  ]}
                  placeholder={filter.allOptionLabel ?? "Все"}
                  selectSize="sm"
                  searchable={filter.options.length > 8}
                  searchPlaceholder={`Найти: ${filter.label.toLowerCase()}...`}
                />
              </label>
            ))}
          </div>
        ) : null}
      </div>

      <div className="overflow-x-auto rounded-lg border border-zinc-200">
        <Table>
          <TableHeader>
            <TableRow>
              {columns.map((column) => {
                const isActiveSort = sortBy === column.accessorKey;
                return (
                  <TableHead key={column.accessorKey}>
                    <button
                      type="button"
                      className={column.enableSorting ? "inline-flex items-center gap-1" : ""}
                      onClick={() =>
                        column.enableSorting && setSort(column.accessorKey)
                      }
                      disabled={!column.enableSorting}
                    >
                      {column.header}
                      {column.enableSorting ? (
                        <span aria-hidden>
                          {!isActiveSort ? "<>" : sortDirection === "asc" ? "^" : "v"}
                        </span>
                      ) : null}
                    </button>
                  </TableHead>
                );
              })}
            </TableRow>
          </TableHeader>

          <TableBody>
            {isLoading ? (
              <TableRow>
                <TableCell
                  colSpan={columns.length}
                  className="px-3 py-8 text-center text-zinc-500"
                >
                  Загрузка данных...
                </TableCell>
              </TableRow>
            ) : paginationRowModel.length === 0 ? (
              <TableRow>
                <TableCell
                  colSpan={columns.length}
                  className="px-3 py-8 text-center text-zinc-500"
                >
                  {emptyMessage}
                </TableCell>
              </TableRow>
            ) : (
              paginationRowModel.map((row) => (
                <TableRow key={row.id}>
                  {columns.map((column) => (
                    <TableCell
                      key={`${row.id}-${column.accessorKey}`}
                      className="whitespace-nowrap text-zinc-700"
                    >
                      {column.cell
                        ? column.cell(row)
                        : String(row[column.accessorKey] ?? "")}
                    </TableCell>
                  ))}
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      <div className="mt-4 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <p className="text-sm text-zinc-600">
          Страница {totalPages === 0 ? 0 : pageIndex + 1} из {totalPages}
        </p>

        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setPageIndex((current) => Math.max(0, current - 1))}
            disabled={pageIndex === 0}
          >
            Назад
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() =>
              setPageIndex((current) => Math.min(totalPages - 1, current + 1))
            }
            disabled={totalPages === 0 || pageIndex >= totalPages - 1}
          >
            Вперед
          </Button>
        </div>
      </div>
    </div>
  );
}
