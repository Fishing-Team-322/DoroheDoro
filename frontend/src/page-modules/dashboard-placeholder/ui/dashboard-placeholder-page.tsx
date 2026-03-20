import { Button, ConsolePage, SectionHeader } from "@/src/shared/ui";

export function DashboardPlaceholderPage({
  title,
  description,
}: {
  title: string;
  description: string;
}) {
  return (
    <ConsolePage>
      <div className="min-w-0">
        <header className="space-y-4">
          <nav aria-label="Breadcrumb">
            <ol className="flex flex-wrap items-center gap-2 text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
              <li className="flex items-center gap-2">
                <span className="text-[color:var(--foreground)]">Панель</span>
                <span className="text-[color:var(--border-strong)]">/</span>
              </li>
              <li className="text-[color:var(--foreground)]">{title}</li>
            </ol>
          </nav>

          <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
            <div className="max-w-4xl space-y-2">
              <h1 className="text-[28px] font-semibold tracking-tight text-[color:var(--foreground)]">
                {title}
              </h1>
              <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
                {description}
              </p>
            </div>

            <div className="flex flex-wrap items-center gap-2 lg:justify-end">
              <Button type="button" variant="outline">
                Спланировать модуль
              </Button>
            </div>
          </div>
        </header>

        <section className="min-w-0 border-t border-[color:var(--border)]">
          <div className="grid gap-0 xl:grid-cols-[minmax(0,1.4fr)_360px]">
            <div className="space-y-5 py-5 pr-0 xl:pr-6">
              <SectionHeader
                title="Запланированный модуль"
                description="Контентная область уже встроена в shell и готова к наполнению рабочими таблицами, политиками применения и разбором отклонений."
              />

              <div className="space-y-0">
                <div className="border-b border-[color:var(--border)] py-4 first:border-t">
                  <p className="text-sm font-medium text-[color:var(--foreground)]">Статус</p>
                  <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                    Модуль зарезервирован и ожидает предметную модель правил и состояний
                    применения.
                  </p>
                </div>
                <div className="border-b border-[color:var(--border)] py-4">
                  <p className="text-sm font-medium text-[color:var(--foreground)]">
                    Что появится здесь
                  </p>
                  <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                    Реестр политик, статусы rollout, цели применения, нарушения и окно
                    разбора по каждому правилу.
                  </p>
                </div>
                <div className="border-b border-[color:var(--border)] py-4">
                  <p className="text-sm font-medium text-[color:var(--foreground)]">
                    Следующий шаг
                  </p>
                  <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                    Подключить policy inventory, compliance summary и детальный inspector
                    для выбранного правила.
                  </p>
                </div>
              </div>
            </div>

            <div className="border-t border-[color:var(--border)] py-5 xl:border-l xl:border-t-0 xl:pl-6">
              <SectionHeader
                title="Скелет области"
                description="Рабочие зоны, которые уже можно занимать данными."
              />
              <div className="mt-4 w-full border-t border-[color:var(--border)]" />
              <div className="space-y-0 pt-4">
                <div className="border-b border-[color:var(--border)] py-3 first:border-t">
                  <p className="text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
                    Row 01
                  </p>
                  <p className="mt-1 text-sm text-[color:var(--foreground)]">
                    Таблица политик и статусов применения
                  </p>
                </div>
                <div className="border-b border-[color:var(--border)] py-3">
                  <p className="text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
                    Row 02
                  </p>
                  <p className="mt-1 text-sm text-[color:var(--foreground)]">
                    Фильтры по сервису, окружению и типу нарушения
                  </p>
                </div>
                <div className="border-b border-[color:var(--border)] py-3">
                  <p className="text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
                    Row 03
                  </p>
                  <p className="mt-1 text-sm text-[color:var(--foreground)]">
                    Inspector выбранной политики и истории изменений
                  </p>
                </div>
              </div>
            </div>
          </div>
        </section>
      </div>
    </ConsolePage>
  );
}
