"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import {
  Button,
  EmptyState,
  Input,
  Select,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  TableSortButton,
} from "@/src/shared/ui";
import { translateValueLabel, useI18n } from "@/src/shared/lib/i18n";
import { useLiveLogs } from "../model";
import { PageStack, formatDateTime } from "./operations-ui";

type SortKey = "timestamp" | "severity" | "host" | "service" | "message";
type SortDirection = "asc" | "desc" | null;

const copyByLocale = {
  en: {
    severityAny: "Any severity",
    title: "Live logs stream",
    description:
      "Watch new events from the live stream, pause the feed, and quickly filter by host, service, and severity.",
    filters: {
      host: "Host",
      service: "Service",
      pause: "Pause",
      resume: "Resume",
      clear: "Clear",
      autoOn: "Autoscroll: on",
      autoOff: "Autoscroll: off",
    },
    table: {
      timestamp: "Time",
      severity: "Severity",
      host: "Host",
      service: "Service",
      message: "Message",
      emptyTitle: "No events yet",
      emptyDescription:
        "Wait for new messages or relax the current filters.",
    },
  },
  ru: {
    severityAny: "Любая severity",
    title: "Поток логов",
    description:
      "Смотрите новые события из live stream, ставьте поток на паузу и быстро фильтруйте его по host, service и severity.",
    filters: {
      host: "Хост",
      service: "Сервис",
      pause: "Пауза",
      resume: "Продолжить",
      clear: "Очистить",
      autoOn: "Автоскролл: вкл",
      autoOff: "Автоскролл: выкл",
    },
    table: {
      timestamp: "Время",
      severity: "Severity",
      host: "Хост",
      service: "Сервис",
      message: "Сообщение",
      emptyTitle: "Событий пока нет",
      emptyDescription:
        "Подождите новые сообщения или ослабьте текущие фильтры.",
    },
  },
} as const;

function getSeverityTone(
  value?: string
): "success" | "warning" | "error" | "neutral" {
  const normalized = value?.toLowerCase();

  if (normalized === "debug" || normalized === "info") return "neutral";
  if (normalized === "warn") return "warning";
  if (normalized === "error" || normalized === "fatal") return "error";

  return "neutral";
}

function getSeverityRank(value?: string) {
  const normalized = value?.toLowerCase();

  if (normalized === "debug") return 0;
  if (normalized === "info") return 1;
  if (normalized === "warn") return 2;
  if (normalized === "error") return 3;
  if (normalized === "fatal") return 4;

  return -1;
}

function compareStrings(a?: string | null, b?: string | null, locale?: string) {
  return (a ?? "").localeCompare(b ?? "", locale, { sensitivity: "base" });
}

export function LiveLogsPage() {
  const { locale } = useI18n();
  const copy = copyByLocale[locale];
  const [host, setHost] = useState("");
  const [service, setService] = useState("");
  const [severity, setSeverity] = useState("");
  const [paused, setPaused] = useState(false);
  const [autoScroll, setAutoScroll] = useState(true);
  const [sortKey, setSortKey] = useState<SortKey | null>(null);
  const [sortDirection, setSortDirection] = useState<SortDirection>(null);

  const scrollContainerRef = useRef<HTMLDivElement | null>(null);
  const severityOptions = useMemo(
    () => [
      { value: "", label: copy.severityAny },
      { value: "debug", label: "debug" },
      { value: "info", label: "info" },
      { value: "warn", label: "warn" },
      { value: "error", label: "error" },
      { value: "fatal", label: "fatal" },
    ],
    [copy.severityAny]
  );

  const filters = useMemo(
    () => ({
      host: host || undefined,
      service: service || undefined,
      severity: severity || undefined,
    }),
    [host, service, severity]
  );

  const liveLogs = useLiveLogs({
    enabled: !paused,
    filters,
  });

  useEffect(() => {
    if (!autoScroll) return;

    const element = scrollContainerRef.current;
    if (!element) return;

    element.scrollTop = element.scrollHeight;
  }, [autoScroll, liveLogs.items.length]);

  const sortedItems = useMemo(() => {
    const items = [...liveLogs.items];

    items.sort((a, b) => {
      let result = 0;

      switch (sortKey) {
        case "timestamp":
          result =
            new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime();
          break;
        case "severity":
          result = getSeverityRank(a.severity) - getSeverityRank(b.severity);
          break;
        case "host":
          result = compareStrings(a.host, b.host, locale);
          break;
        case "service":
          result = compareStrings(a.service, b.service, locale);
          break;
        case "message":
          result = compareStrings(a.message, b.message, locale);
          break;
      }

      return sortDirection === "asc" ? result : -result;
    });

    return items;
  }, [liveLogs.items, locale, sortDirection, sortKey]);

  const currentSortDirection = sortDirection ?? undefined;

  const handleSort = (key: SortKey) => {
    if (sortKey !== key) {
      setSortKey(key);
      setSortDirection(key === "timestamp" ? "desc" : "asc");
      return;
    }

    if (sortDirection === "asc") {
      setSortDirection("desc");
      return;
    }

    if (sortDirection === "desc") {
      setSortKey(null);
      setSortDirection(null);
      return;
    }

    setSortDirection(key === "timestamp" ? "desc" : "asc");
  };

  return (
    <PageStack>
      <div className="rounded-[28px] border border-[color:var(--border)] bg-[color:var(--surface)] p-8 md:p-10">
        <div className="space-y-8">
          <div className="space-y-3">
            <h2 className="text-3xl font-semibold tracking-tight text-[color:var(--foreground)] md:text-5xl">
              {copy.title}
            </h2>

            <p className="max-w-3xl text-base leading-7 text-[color:var(--muted-foreground)] md:text-lg">
              {copy.description}
            </p>
          </div>

          <div className="border-t border-[color:var(--border)] pt-8">
            <div className="grid grid-cols-1 gap-4 xl:grid-cols-3">
              <Input
                label={copy.filters.host}
                value={host}
                onChange={(event) => setHost(event.target.value)}
                inputSize="md"
              />

              <Input
                label={copy.filters.service}
                value={service}
                onChange={(event) => setService(event.target.value)}
                inputSize="md"
              />

              <div className="w-full">
                <Select
                  value={severity}
                  onChange={(event) => setSeverity(event.target.value)}
                  options={severityOptions}
                  placeholder={copy.severityAny}
                  selectSize="md"
                  triggerClassName="h-14 px-4 text-sm border border-[var(--input-border)] hover:border-[var(--input-border-hover)] hover:bg-[var(--input-background-hover)] focus-visible:border-[var(--ring)] focus-visible:bg-[var(--input-background-focus)] focus-visible:shadow-[0_0_0_1px_var(--ring),0_0_0_2px_rgba(113,113,122,0.08)]"
                />
              </div>
            </div>

            <div className="mt-5 flex flex-wrap gap-3">
              <Button
                size="sm"
                className="h-11 px-5"
                onClick={() => setPaused((current) => !current)}
              >
                {paused ? copy.filters.resume : copy.filters.pause}
              </Button>

              <Button
                size="sm"
                className="h-11 px-5"
                onClick={() => liveLogs.clear()}
              >
                {copy.filters.clear}
              </Button>

              <Button
                variant="outline"
                size="sm"
                className="h-11 px-5"
                onClick={() => setAutoScroll((current) => !current)}
              >
                {autoScroll ? copy.filters.autoOn : copy.filters.autoOff}
              </Button>
            </div>
          </div>

          <div>
            <div ref={scrollContainerRef} className="overflow-auto rounded-2xl">
              <Table>
                <TableHeader>
                  <TableRow className="border-b-0 hover:bg-transparent">
                    <TableHead>
                      <TableSortButton
                        active={sortKey === "timestamp"}
                        direction={currentSortDirection}
                        onClick={() => handleSort("timestamp")}
                      >
                        {copy.table.timestamp}
                      </TableSortButton>
                    </TableHead>

                    <TableHead>
                      <TableSortButton
                        active={sortKey === "severity"}
                        direction={currentSortDirection}
                        onClick={() => handleSort("severity")}
                      >
                        {copy.table.severity}
                      </TableSortButton>
                    </TableHead>

                    <TableHead>
                      <TableSortButton
                        active={sortKey === "host"}
                        direction={currentSortDirection}
                        onClick={() => handleSort("host")}
                      >
                        {copy.table.host}
                      </TableSortButton>
                    </TableHead>

                    <TableHead>
                      <TableSortButton
                        active={sortKey === "service"}
                        direction={currentSortDirection}
                        onClick={() => handleSort("service")}
                      >
                        {copy.table.service}
                      </TableSortButton>
                    </TableHead>

                    <TableHead>
                      <TableSortButton
                        active={sortKey === "message"}
                        direction={currentSortDirection}
                        onClick={() => handleSort("message")}
                      >
                        {copy.table.message}
                      </TableSortButton>
                    </TableHead>
                  </TableRow>
                </TableHeader>

                <TableBody>
                  {sortedItems.map((item) => {
                    const severityTone = getSeverityTone(item.severity);

                    return (
                      <TableRow
                        key={`${item.timestamp}-${item.host ?? "n/a"}-${item.message}`}
                      >
                        <TableCell className="whitespace-nowrap">
                          {formatDateTime(item.timestamp, locale)}
                        </TableCell>

                        <TableCell>
                          <span
                            className={`font-semibold ${
                              severityTone === "error"
                                ? "text-red-400"
                                : severityTone === "warning"
                                  ? "text-amber-400"
                                  : "text-[color:var(--foreground)]"
                            }`}
                          >
                            {translateValueLabel(item.severity ?? "unknown", locale)}
                          </span>
                        </TableCell>

                        <TableCell>{item.host ?? "n/a"}</TableCell>
                        <TableCell>{item.service ?? "n/a"}</TableCell>
                        <TableCell className="max-w-3xl whitespace-normal break-words text-[color:var(--foreground)]">
                          {item.message}
                        </TableCell>
                      </TableRow>
                    );
                  })}

                  {sortedItems.length === 0 ? (
                    <TableRow>
                      <TableCell colSpan={5}>
                        <EmptyState
                          variant="flush"
                          title={copy.table.emptyTitle}
                          description={copy.table.emptyDescription}
                        />
                      </TableCell>
                    </TableRow>
                  ) : null}
                </TableBody>
              </Table>
            </div>
          </div>
        </div>
      </div>
    </PageStack>
  );
}
