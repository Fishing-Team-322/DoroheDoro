"use client";

import { useMemo } from "react";
import { useQueryParams } from "@/src/shared/hooks";
import {
  Button,
  Card,
  Pagination,
  SearchInput,
  Select,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/src/shared/ui";

type Product = {
  id: number;
  name: string;
  category: "Книги" | "Электроника" | "Одежда";
  price: number;
  rating: number;
};

const PRODUCTS: Product[] = [
  { id: 1, name: "Наушники с шумоподавлением", category: "Электроника", price: 220, rating: 4.8 },
  { id: 2, name: "Механическая клавиатура", category: "Электроника", price: 130, rating: 4.5 },
  { id: 3, name: "Паттерны JavaScript", category: "Книги", price: 38, rating: 4.7 },
  { id: 4, name: "Минималистичная футболка", category: "Одежда", price: 22, rating: 4.3 },
  { id: 5, name: "Беговые кроссовки", category: "Одежда", price: 110, rating: 4.6 },
  { id: 6, name: "Системы, работающие с данными", category: "Книги", price: 42, rating: 4.9 },
  { id: 7, name: "Хаб USB-C", category: "Электроника", price: 45, rating: 4.2 },
  { id: 8, name: "Худи", category: "Одежда", price: 55, rating: 4.4 },
  { id: 9, name: "Чистая архитектура", category: "Книги", price: 35, rating: 4.8 },
  { id: 10, name: "Веб-камера", category: "Электроника", price: 90, rating: 4.1 },
  { id: 11, name: "Брюки чинос", category: "Одежда", price: 60, rating: 4.5 },
  { id: 12, name: "Программист-прагматик", category: "Книги", price: 40, rating: 4.9 },
];

const PAGE_SIZE = 4;

const filterOptions = [
  { value: "all", label: "Все категории" },
  { value: "books", label: "Книги" },
  { value: "electronics", label: "Электроника" },
  { value: "clothing", label: "Одежда" },
];

const sortOptions = [
  { value: "name:asc", label: "Название (А-Я)" },
  { value: "name:desc", label: "Название (Я-А)" },
  { value: "price:asc", label: "Цена (по возрастанию)" },
  { value: "price:desc", label: "Цена (по убыванию)" },
  { value: "rating:desc", label: "Рейтинг (по убыванию)" },
];

export default function QueryDemoPage() {
  const { query, setParam, setParams, removeParams } = useQueryParams();

  const filteredAndSorted = useMemo(() => {
    const normalizedSearch = query.search.toLowerCase();
    const [sortField, sortDirection] = query.sort.split(":") as [
      "name" | "price" | "rating",
      "asc" | "desc",
    ];

    return PRODUCTS.filter((item) => {
      const matchesSearch = item.name.toLowerCase().includes(normalizedSearch);
      const matchesFilter =
        query.filter === "all" ||
        item.category.toLowerCase() === query.filter.toLowerCase();
      return matchesSearch && matchesFilter;
    }).sort((a, b) => {
      const direction = sortDirection === "asc" ? 1 : -1;
      if (sortField === "name") {
        return a.name.localeCompare(b.name) * direction;
      }
      return (a[sortField] - b[sortField]) * direction;
    });
  }, [query.filter, query.search, query.sort]);

  const totalPages = Math.max(1, Math.ceil(filteredAndSorted.length / PAGE_SIZE));
  const activePage = Math.min(query.page, totalPages);

  const paginated = useMemo(() => {
    const start = (activePage - 1) * PAGE_SIZE;
    return filteredAndSorted.slice(start, start + PAGE_SIZE);
  }, [activePage, filteredAndSorted]);

  const onResetQuery = () => removeParams(["search", "filter", "sort", "page"]);

  return (
    <main className="min-h-screen bg-zinc-50 p-6 text-zinc-900">
      <Card className="mx-auto max-w-5xl space-y-6">
        <header className="space-y-2">
          <h1 className="text-2xl font-bold">Демо параметров запроса</h1>
          <p className="text-sm text-zinc-600">
            Поиск, сортировка, фильтрация и пагинация полностью синхронизированы с
            параметрами URL.
          </p>
        </header>

        <section className="grid gap-3 md:grid-cols-4">
          <SearchInput
            value={query.search}
            onChange={(event) => setParams({ search: event.target.value, page: 1 })}
            placeholder="Поиск товара"
          />

          <Select
            value={query.filter}
            onChange={(event) => setParams({ filter: event.target.value, page: 1 })}
            options={filterOptions}
            placeholder="Выберите категорию"
            selectSize="md"
          />

          <Select
            value={query.sort}
            onChange={(event) => setParam("sort", event.target.value)}
            options={sortOptions}
            placeholder="Выберите сортировку"
            selectSize="md"
          />

          <Button variant="outline" onClick={onResetQuery}>
            Сбросить запрос
          </Button>
        </section>

        <div className="overflow-hidden rounded-lg border border-zinc-800 bg-zinc-950">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Товар</TableHead>
                <TableHead>Категория</TableHead>
                <TableHead>Цена</TableHead>
                <TableHead>Рейтинг</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {paginated.map((product) => (
                <TableRow key={product.id}>
                  <TableCell>{product.name}</TableCell>
                  <TableCell>{product.category}</TableCell>
                  <TableCell>${product.price}</TableCell>
                  <TableCell>{product.rating}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>

        <footer className="flex flex-wrap items-center justify-between gap-3">
          <p className="text-sm text-zinc-600">
            Найдено: {filteredAndSorted.length}, страница {activePage} / {totalPages}
          </p>
          <Pagination
            page={activePage}
            totalPages={totalPages}
            onPageChange={(page) => setParam("page", page)}
          />
        </footer>
      </Card>
    </main>
  );
}
