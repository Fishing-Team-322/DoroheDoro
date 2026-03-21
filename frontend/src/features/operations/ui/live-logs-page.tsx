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
import { useLiveLogs } from "../model";
import { PageStack, formatDateTime } from "./operations-ui";

const SEVERITY_OPTIONS = [
  { value: "", label: "Любая severity" },
  { value: "debug", label: "debug" },
  { value: "info", label: "info" },
  { value: "warn", label: "warn" },
  { value: "error", label: "error" },
  { value: "fatal", label: "fatal" },
];

type SortKey = "timestamp" | "severity" | "host" | "service" | "message";
type SortDirection = "asc" | "desc" | null;

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

function compareStrings(a?: string | null, b?: string | null) {
  return (a ?? "").localeCompare(b ?? "", "ru", { sensitivity: "base" });
}

export function LiveLogsPage() {
  const [host, setHost] = useState("");
  const [service, setService] = useState("");
  const [severity, setSeverity] = useState("");
  const [paused, setPaused] = useState(false);
  const [autoScroll, setAutoScroll] = useState(true);
  const [sortKey, setSortKey] = useState<SortKey | null>(null);
  const [sortDirection, setSortDirection] = useState<SortDirection>(null);

  const scrollContainerRef = useRef<HTMLDivElement | null>(null);

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
          result = compareStrings(a.host, b.host);
          break;
        case "service":
          result = compareStrings(a.service, b.service);
          break;
        case "message":
          result = compareStrings(a.message, b.message);
          break;
      }

      return sortDirection === "asc" ? result : -result;
    });

    return items;
  }, [liveLogs.items, sortKey, sortDirection]);

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
              поток логов
            </h2>

            <p className="max-w-3xl text-base leading-7 text-[color:var(--muted-foreground)] md:text-lg">
              Здесь можно смотреть новые события из live stream,
              приостанавливать поток и быстро фильтровать его по host, service и
              severity.
            </p>
          </div>

          <div className="border-t border-[color:var(--border)] pt-8">
            <div className="grid grid-cols-1 gap-4 xl:grid-cols-3">
              <Input
                label="Host"
                value={host}
                onChange={(event) => setHost(event.target.value)}
                inputSize="md"
              />

              <Input
                label="Service"
                value={service}
                onChange={(event) => setService(event.target.value)}
                inputSize="md"
              />

              <div className="w-full">
                <Select
                  value={severity}
                  onChange={(event) => setSeverity(event.target.value)}
                  options={SEVERITY_OPTIONS}
                  placeholder="Любая severity"
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
                {paused ? "Возобновить" : "Пауза"}
              </Button>

              <Button
                size="sm"
                className="h-11 px-5"
                onClick={() => liveLogs.clear()}
              >
                Очистить
              </Button>

              <Button
                variant="outline"
                size="sm"
                className="h-11 px-5"
                onClick={() => setAutoScroll((current) => !current)}
              >
                {autoScroll ? "Автоскролл: вкл" : "Автоскролл: выкл"}
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
                        direction={sortDirection}
                        onClick={() => handleSort("timestamp")}
                      >
                        время
                      </TableSortButton>
                    </TableHead>

                    <TableHead>
                      <TableSortButton
                        active={sortKey === "severity"}
                        direction={sortDirection}
                        onClick={() => handleSort("severity")}
                      >
                        severity
                      </TableSortButton>
                    </TableHead>

                    <TableHead>
                      <TableSortButton
                        active={sortKey === "host"}
                        direction={sortDirection}
                        onClick={() => handleSort("host")}
                      >
                        host
                      </TableSortButton>
                    </TableHead>

                    <TableHead>
                      <TableSortButton
                        active={sortKey === "service"}
                        direction={sortDirection}
                        onClick={() => handleSort("service")}
                      >
                        service
                      </TableSortButton>
                    </TableHead>

                    <TableHead>
                      <TableSortButton
                        active={sortKey === "message"}
                        direction={sortDirection}
                        onClick={() => handleSort("message")}
                      >
                        сообщение
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
                          {formatDateTime(item.timestamp)}
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
                            {(item.severity ?? "n/a").toUpperCase()}
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
                          title="Пока нет событий"
                          description="Подождите новые сообщения или ослабьте фильтры."
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
