"use client";

import { useMemo, useState } from "react";
import { formatRelativeLabel } from "@/src/shared/lib/dashboard";
import {
  Button,
  CodeBlock,
  KeyValueList,
  SearchInput,
  SectionHeader,
  SeverityBadge,
  TableToolbar,
  TimeRangePicker,
} from "@/src/shared/ui";
import { DetailsDrawer } from "@/src/shared/ui/dashboard";
import type { LogRecord } from "@/src/shared/types/dashboard";

export function QueryBar({
  value,
  onChange,
  timeRange,
  onTimeRangeChange,
}: {
  value: string;
  onChange: (value: string) => void;
  timeRange: string;
  onTimeRangeChange: (value: string) => void;
}) {
  return (
    <TableToolbar
      title="Консоль запросов"
      description="Базовый блок для будущего проводника журналов и потокового режима."
      actions={
        <>
          <TimeRangePicker value={timeRange} onChange={onTimeRangeChange} />
          <Button type="button" variant="outline">
            Сохранить вид
          </Button>
        </>
      }
    >
      <SearchInput
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder='service:payments-api severity:error "timeout"'
      />
    </TableToolbar>
  );
}

export function StructuredFieldsPreview({
  fields,
}: {
  fields: LogRecord["fields"];
}) {
  return (
    <div className="space-y-4 rounded-lg border border-[color:var(--border)] bg-[color:var(--surface-subtle)] px-4 py-4">
      <SectionHeader title="Структурированные поля" />
      <KeyValueList
        items={Object.entries(fields).map(([label, value]) => ({
          label,
          value: String(value),
        }))}
      />
    </div>
  );
}

export function LogRow({
  record,
  selected,
  onSelect,
}: {
  record: LogRecord;
  selected?: boolean;
  onSelect?: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={
        selected
          ? "w-full border-b border-[color:var(--border)] bg-[color:rgba(56,189,248,0.08)] px-4 py-4 text-left transition-colors"
          : "w-full border-b border-[color:var(--border)] px-4 py-4 text-left transition-colors hover:bg-[color:var(--surface-subtle)]"
      }
    >
      <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
        <div className="space-y-2">
          <div className="flex flex-wrap items-center gap-2">
            <SeverityBadge severity={record.severity} />
            <span className="text-xs uppercase tracking-wide text-zinc-500">
              {record.service}
            </span>
            <span className="text-xs text-zinc-400">{record.host}</span>
          </div>
          <p className="text-sm font-medium text-zinc-100">{record.message}</p>
          <p className="text-xs text-zinc-500">trace_id={record.traceId}</p>
        </div>
        <p className="text-xs text-zinc-400">{formatRelativeLabel(record.timestamp)}</p>
      </div>
    </button>
  );
}

export function LogList({
  records,
  selectedId,
  onSelect,
}: {
  records: LogRecord[];
  selectedId?: string;
  onSelect: (record: LogRecord) => void;
}) {
  return (
    <div className="border-t border-[color:var(--border)]">
      {records.map((record) => (
        <LogRow
          key={record.id}
          record={record}
          selected={record.id === selectedId}
          onSelect={() => onSelect(record)}
        />
      ))}
    </div>
  );
}

export function LogDetailsPanel({
  record,
  open,
  onClose,
}: {
  record?: LogRecord;
  open: boolean;
  onClose?: () => void;
}) {
  if (!record) {
    return (
      <DetailsDrawer
        title="Детали записи"
        description="Выберите запись, чтобы посмотреть структурированный контекст."
        open={open}
        onClose={onClose}
      >
        <p className="text-sm text-zinc-500">
          Событие еще не выбрано. Выберите строку в списке журналов, чтобы
          посмотреть поля полезной нагрузки.
        </p>
      </DetailsDrawer>
    );
  }

  return (
    <DetailsDrawer
      title="Детали записи"
      description={`${record.service} на ${record.host}`}
      open={open}
      onClose={onClose}
    >
      <KeyValueList
        items={[
          { label: "Время", value: formatRelativeLabel(record.timestamp) },
          { label: "Важность", value: <SeverityBadge severity={record.severity} /> },
          { label: "Идентификатор трассы", value: record.traceId },
        ]}
      />
      <StructuredFieldsPreview fields={record.fields} />
      <CodeBlock code={JSON.stringify(record, null, 2)} />
    </DetailsDrawer>
  );
}

export function LogExplorer({ records }: { records: LogRecord[] }) {
  const [query, setQuery] = useState("");
  const [timeRange, setTimeRange] = useState("1h");
  const [selectedRecord, setSelectedRecord] = useState<LogRecord | undefined>(
    records[0]
  );

  const filteredRecords = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    if (!normalizedQuery) {
      return records;
    }

    return records.filter((record) =>
      [record.service, record.host, record.message, record.traceId]
        .join(" ")
        .toLowerCase()
        .includes(normalizedQuery)
    );
  }, [query, records]);

  return (
    <div className="space-y-4">
      <QueryBar
        value={query}
        onChange={setQuery}
        timeRange={timeRange}
        onTimeRangeChange={setTimeRange}
      />

      <div className="grid min-w-0 items-stretch gap-0 xl:grid-cols-[minmax(0,1fr)_420px]">
        <section className="min-w-0 py-5">
          <SectionHeader
            title="Поток событий"
            description="Компактный список журналов с достаточной структурой для поиска и детализации."
          />
          <LogList
            records={filteredRecords}
            selectedId={selectedRecord?.id}
            onSelect={setSelectedRecord}
          />
        </section>

        <aside className="min-w-0 border-t border-[color:var(--border)] py-4 xl:border-l xl:border-t-0 xl:py-0">
          <LogDetailsPanel record={selectedRecord} open={true} />
        </aside>
      </div>
    </div>
  );
}
