"use client";

import { useMemo, useState } from "react";
import {
  Badge,
  Button,
  Card,
  ErrorState,
  Spinner,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/src/shared/ui";
import { ApiError, createApiClient } from "@/src/shared/lib/api";

type DemoTodo = {
  id: number;
  title: string;
  completed: boolean;
};

type DemoState = {
  loading: boolean;
  data: DemoTodo[] | null;
  error: ApiError | null;
};

const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const resolveInputUrl = (input: RequestInfo | URL): string => {
  if (typeof input === "string") return input;
  if (input instanceof URL) return input.toString();
  return input.url;
};

const mockFetch: typeof fetch = async (input) => {
  await wait(900);

  const url = resolveInputUrl(input);

  if (url.includes("/todos-error")) {
    return new Response(
      JSON.stringify({
        message: "Тестовая ошибка: не удалось загрузить список задач",
      }),
      {
        status: 500,
        headers: { "Content-Type": "application/json" },
      }
    );
  }

  return new Response(
    JSON.stringify([
      { id: 1, title: "Прочитать документацию", completed: true },
      { id: 2, title: "Сделать демо API-клиента", completed: false },
      { id: 3, title: "Проверить обработку ошибок", completed: false },
    ] satisfies DemoTodo[]),
    {
      status: 200,
      headers: { "Content-Type": "application/json" },
    }
  );
};

export default function ApiDemoPage() {
  const [state, setState] = useState<DemoState>({
    loading: false,
    data: null,
    error: null,
  });

  const apiClient = useMemo(
    () =>
      createApiClient({
        baseUrl: "/mock-api",
        fetcher: mockFetch,
      }),
    []
  );

  const runRequest = async (mode: "success" | "error") => {
    setState({ loading: true, data: null, error: null });

    try {
      const endpoint = mode === "success" ? "/todos" : "/todos-error";
      const result = await apiClient.get<DemoTodo[]>(endpoint);
      setState({ loading: false, data: result, error: null });
    } catch (error) {
      setState({ loading: false, data: null, error: error as ApiError });
    }
  };

  return (
    <main className="min-h-screen bg-zinc-50 px-6 py-10 text-zinc-900">
      <Card className="mx-auto flex w-full max-w-3xl flex-col gap-6">
        <header className="space-y-2">
          <h1 className="text-2xl font-semibold">Демо API-клиента</h1>
          <p className="text-sm text-zinc-600">
            Демонстрация тестовых запросов с искусственной задержкой и единым типом ошибки API.
          </p>
        </header>

        <div className="flex flex-wrap gap-3">
          <Button onClick={() => runRequest("success")} disabled={state.loading}>
            Успешный запрос
          </Button>
          <Button
            variant="danger"
            onClick={() => runRequest("error")}
            disabled={state.loading}
          >
            Запрос с ошибкой
          </Button>
        </div>

        {state.loading ? (
          <div className="inline-flex items-center gap-2 text-sm text-zinc-600">
            <Spinner size="sm" /> Загрузка данных...
          </div>
        ) : null}

        {state.data ? (
          <section className="space-y-3">
            <h2 className="text-lg font-medium">Успех</h2>
            <div className="overflow-hidden rounded-lg border border-zinc-200">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>ID</TableHead>
                    <TableHead>Название</TableHead>
                    <TableHead>Статус</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {state.data.map((todo) => (
                    <TableRow key={todo.id}>
                      <TableCell>{todo.id}</TableCell>
                      <TableCell>{todo.title}</TableCell>
                      <TableCell>
                        <Badge variant={todo.completed ? "success" : "warning"}>
                          {todo.completed ? "выполнено" : "в процессе"}
                        </Badge>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          </section>
        ) : null}

        {state.error ? (
          <ErrorState
            title="Ошибка"
            description={`сообщение: ${state.error.message}; код: ${state.error.code}; статус: ${state.error.status ?? "н/д"}`}
          />
        ) : null}
      </Card>
    </main>
  );
}
