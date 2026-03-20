"use client";

import type { ReactNode } from "react";
import { useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  Badge,
  Button,
  Card,
  Checkbox,
  ConfirmDialog,
  Dialog,
  EmptyState,
  ErrorState,
  Input,
  Pagination,
  Radio,
  SearchInput,
  Select,
  Skeleton,
  Spinner,
  Tabs,
  Textarea,
} from "@/src/shared/ui";

function Section({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <Card className="space-y-4">
      <h2 className="text-xl font-semibold">{title}</h2>
      {children}
    </Card>
  );
}

const demoSelectOptions = [
  { value: "one", label: "Первый" },
  { value: "two", label: "Второй" },
];

export function UiKitPage() {
  const pathname = usePathname();
  const locale = pathname.split("/")[1] || "ru";
  const [dialogOpen, setDialogOpen] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [isConfirmLoading, setIsConfirmLoading] = useState(false);
  const [demoSelect, setDemoSelect] = useState("");

  const onConfirm = async () => {
    setIsConfirmLoading(true);
    await new Promise((resolve) => setTimeout(resolve, 800));
    setIsConfirmLoading(false);
    setConfirmOpen(false);
  };

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-5xl flex-col gap-6 bg-zinc-50 p-6">
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-3xl font-bold">Демо набора интерфейса</h1>
          <p className="text-sm text-zinc-600">
            Витрина переиспользуемых визуальных компонентов.
          </p>
        </div>
        <Link href={`/${locale}`} className="text-sm underline">
          Назад на главную
        </Link>
      </header>

      <Section title="Поля ввода и элементы управления">
        <div className="grid gap-3 md:grid-cols-2">
          <Input placeholder="Поле ввода" />
          <Input placeholder="Отключено" disabled />
          <Textarea placeholder="Многострочное поле" />
          <Select
            value={demoSelect}
            onChange={(event) => setDemoSelect(event.target.value)}
            options={demoSelectOptions}
            placeholder="Выберите вариант"
            selectSize="md"
          />
          <div className="flex flex-col gap-2">
            <Checkbox label="Запомнить меня" />
            <Checkbox label="Отключенный чекбокс" disabled />
          </div>
          <div className="flex flex-col gap-2">
            <Radio name="plan" label="Бесплатный" defaultChecked />
            <Radio name="plan" label="Профессиональный" />
          </div>
        </div>
      </Section>

      <Section title="Кнопки, бейджи и загрузка">
        <div className="flex flex-wrap items-center gap-3">
          <Button>Основная</Button>
          <Button variant="secondary">Вторичная</Button>
          <Button variant="outline">Контурная</Button>
          <Button variant="danger">Опасная</Button>
          <Button loading>Загрузка</Button>
          <Badge>Обычный</Badge>
          <Badge variant="success">Успех</Badge>
          <Badge variant="warning">Предупреждение</Badge>
          <Badge variant="danger">Опасно</Badge>
        </div>
        <div className="flex items-center gap-3">
          <Spinner size="sm" />
          <Spinner />
          <Spinner size="lg" />
        </div>
      </Section>

      <Section title="Вкладки, поиск и пагинация">
        <Tabs
          tabs={[
            {
              key: "overview",
              label: "Обзор",
              content: <p className="text-sm text-zinc-700">Содержимое обзора</p>,
            },
            {
              key: "details",
              label: "Детали",
              content: <p className="text-sm text-zinc-700">Содержимое деталей</p>,
            },
            {
              key: "disabled",
              label: "Отключено",
              disabled: true,
              content: null,
            },
          ]}
        />
        <SearchInput placeholder="Поиск элементов..." />
        <Pagination page={2} totalPages={5} onPageChange={() => {}} />
      </Section>

      <Section title="Состояния интерфейса">
        <div className="grid gap-3 md:grid-cols-2">
          <EmptyState
            title="Нет данных"
            description="Создайте первую запись, чтобы начать."
            action={<Button size="sm">Создать</Button>}
          />
          <ErrorState
            title="Запрос не выполнен"
            description="Попробуйте еще раз немного позже."
            action={
              <Button size="sm" variant="outline">
                Повторить
              </Button>
            }
          />
          <Card className="space-y-2">
            <Skeleton className="h-4 w-2/3" />
            <Skeleton className="h-4 w-full" />
            <Skeleton className="h-4 w-4/5" />
          </Card>
        </div>
      </Section>

      <Section title="Диалог и подтверждение">
        <div className="flex flex-wrap gap-3">
          <Button onClick={() => setDialogOpen(true)}>Открыть диалог</Button>
          <Button variant="danger" onClick={() => setConfirmOpen(true)}>
            Открыть подтверждение
          </Button>
        </div>
      </Section>

      <Dialog
        open={dialogOpen}
        onClose={() => setDialogOpen(false)}
        title="Заголовок диалога"
        description="Простой модальный диалог для повторного использования."
      >
        <p className="text-sm text-zinc-600">Здесь находится содержимое диалога.</p>
      </Dialog>

      <ConfirmDialog
        open={confirmOpen}
        onCancel={() => setConfirmOpen(false)}
        onConfirm={onConfirm}
        loading={isConfirmLoading}
        title="Удалить запись"
        description="Это действие нельзя отменить."
      />
    </main>
  );
}
