"use client";

import {
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactNode,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
} from "react";
import { cn } from "@/src/shared/lib/cn";
import { getSiteCopy, useOptionalI18n } from "@/src/shared/lib/i18n";
import { Button } from "@/src/shared/ui/button";

type DateTimePickerProps = {
  label: string;
  value: string;
  onChange: (value: string) => void;
  helperText?: string;
  error?: string;
  disabled?: boolean;
  containerClassName?: string;
};

function CalendarIcon() {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 24 24"
      className="h-5 w-5"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M8 2v4" />
      <path d="M16 2v4" />
      <rect x="3" y="4" width="18" height="18" rx="2" />
      <path d="M3 10h18" />
    </svg>
  );
}

function ClockIcon() {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 24 24"
      className="h-4 w-4"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <circle cx="12" cy="12" r="9" />
      <path d="M12 7v5l3 3" />
    </svg>
  );
}

function ChevronLeftIcon() {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 24 24"
      className="h-4 w-4"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="m15 18-6-6 6-6" />
    </svg>
  );
}

function ChevronRightIcon() {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 24 24"
      className="h-4 w-4"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="m9 18 6-6-6-6" />
    </svg>
  );
}

function parseLocalDateTime(value: string) {
  if (!value) return null;

  const [datePart, timePart = "00:00"] = value.split("T");
  if (!datePart) return null;

  const [year, month, day] = datePart.split("-").map(Number);
  const [hours = 0, minutes = 0] = timePart.split(":").map(Number);

  if (
    !Number.isFinite(year) ||
    !Number.isFinite(month) ||
    !Number.isFinite(day) ||
    !Number.isFinite(hours) ||
    !Number.isFinite(minutes)
  ) {
    return null;
  }

  return new Date(year, month - 1, day, hours, minutes, 0, 0);
}

function formatToLocalValue(date: Date) {
  const year = date.getFullYear();
  const month = `${date.getMonth() + 1}`.padStart(2, "0");
  const day = `${date.getDate()}`.padStart(2, "0");
  const hours = `${date.getHours()}`.padStart(2, "0");
  const minutes = `${date.getMinutes()}`.padStart(2, "0");

  return `${year}-${month}-${day}T${hours}:${minutes}`;
}

function formatDisplayValue(value: string, locale: string) {
  const date = parseLocalDateTime(value);
  if (!date) return "";

  return new Intl.DateTimeFormat(locale, {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function isSameDay(a: Date, b: Date) {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

function getMonthMatrix(viewDate: Date) {
  const year = viewDate.getFullYear();
  const month = viewDate.getMonth();

  const firstDayOfMonth = new Date(year, month, 1);
  const firstWeekday = (firstDayOfMonth.getDay() + 6) % 7;
  const startDate = new Date(year, month, 1 - firstWeekday);

  return Array.from({ length: 42 }, (_, index) => {
    const date = new Date(startDate);
    date.setDate(startDate.getDate() + index);
    return date;
  });
}

function clampNumber(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

function normalizeTwoDigits(value: string, max: number) {
  const digits = value.replace(/\D/g, "");
  if (!digits) return "";
  const normalized = clampNumber(Number(digits), 0, max);
  return `${normalized}`.padStart(2, "0");
}

function increment(value: string, max: number, step = 1) {
  const current = Number(value || 0);
  const next = current + step > max ? 0 : current + step;
  return `${next}`.padStart(2, "0");
}

function decrement(value: string, max: number, step = 1) {
  const current = Number(value || 0);
  const next = current - step < 0 ? max : current - step;
  return `${next}`.padStart(2, "0");
}

export function DateTimePicker({
  label,
  value,
  onChange,
  helperText,
  error,
  disabled,
  containerClassName,
}: DateTimePickerProps) {
  const i18n = useOptionalI18n();
  const locale = i18n?.locale ?? "en";
  const copy = getSiteCopy(locale).datePicker;
  const id = useId();
  const rootRef = useRef<HTMLDivElement | null>(null);

  const initialDate = useMemo(
    () => parseLocalDateTime(value) ?? new Date(),
    [value]
  );

  const [open, setOpen] = useState(false);
  const [selectedDate, setSelectedDate] = useState<Date | null>(
    parseLocalDateTime(value)
  );
  const [viewDate, setViewDate] = useState<Date>(initialDate);
  const [hour, setHour] = useState(
    selectedDate ? `${selectedDate.getHours()}`.padStart(2, "0") : "00"
  );
  const [minute, setMinute] = useState(
    selectedDate ? `${selectedDate.getMinutes()}`.padStart(2, "0") : "00"
  );

  useEffect(() => {
    const parsed = parseLocalDateTime(value);
    setSelectedDate(parsed);
    setViewDate(parsed ?? new Date());
    setHour(parsed ? `${parsed.getHours()}`.padStart(2, "0") : "00");
    setMinute(parsed ? `${parsed.getMinutes()}`.padStart(2, "0") : "00");
  }, [value]);

  useEffect(() => {
    if (!open) return;

    const handlePointerDown = (event: MouseEvent) => {
      if (!rootRef.current) return;
      if (rootRef.current.contains(event.target as Node)) return;
      setOpen(false);
    };

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
      }
    };

    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleEscape);

    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [open]);

  const describedBy = error
    ? `${id}-error`
    : helperText
      ? `${id}-helper`
      : undefined;

  const days = useMemo(() => getMonthMatrix(viewDate), [viewDate]);
  const displayValue = useMemo(
    () => formatDisplayValue(value, copy.locale),
    [copy.locale, value]
  );

  const applyValue = () => {
    if (!selectedDate) {
      onChange("");
      setOpen(false);
      return;
    }

    const nextDate = new Date(selectedDate);
    nextDate.setHours(Number(hour || "0"), Number(minute || "0"), 0, 0);

    onChange(formatToLocalValue(nextDate));
    setOpen(false);
  };

  const clearValue = () => {
    setSelectedDate(null);
    setHour("00");
    setMinute("00");
    onChange("");
    setOpen(false);
  };

  const handleSelectDay = (day: Date) => {
    const nextDate = selectedDate ? new Date(selectedDate) : new Date(day);
    nextDate.setFullYear(day.getFullYear(), day.getMonth(), day.getDate());
    setSelectedDate(nextDate);
    setViewDate(new Date(day.getFullYear(), day.getMonth(), 1));
  };

  const handleKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
      applyValue();
    }

    if (event.key === "Escape") {
      event.preventDefault();
      setOpen(false);
    }
  };

  return (
    <div className={cn("w-full", containerClassName)} ref={rootRef}>
      <div className="relative">
        <button
          type="button"
          disabled={disabled}
          aria-haspopup="dialog"
          aria-expanded={open}
          aria-describedby={describedBy}
          data-invalid={Boolean(error) || undefined}
          onClick={() => setOpen((current) => !current)}
          className={cn(
            "peer relative flex h-14 w-full items-end rounded-xl border px-4 pb-2 pt-6 text-left outline-none transition-all duration-200",
            "bg-[var(--input-background)] text-[var(--foreground)]",
            "disabled:cursor-not-allowed disabled:opacity-50",
            error
              ? "border-[var(--status-danger-border)]"
              : [
                  "border-[var(--input-border)]",
                  "hover:border-[var(--input-border-hover)]",
                  "hover:bg-[var(--input-background-hover)]",
                  "focus-visible:border-[var(--ring)]",
                  "focus-visible:bg-[var(--input-background-focus)]",
                  "focus-visible:shadow-[0_0_0_1px_var(--ring),0_0_0_2px_rgba(113,113,122,0.08)]",
                ].join(" ")
          )}
        >
          <span
            className={cn(
              "pointer-events-none absolute left-4 top-4 z-10 origin-[0] bg-transparent px-0 text-sm text-[#8a8a8f] transition-all duration-150",
              value ? "-translate-y-3 scale-90" : "translate-y-0 scale-100",
              open && "-translate-y-3 scale-90",
              error && open && "text-[var(--status-danger-fg)]"
            )}
          >
            {label}
          </span>

          <span
            className={cn(
              "block min-h-[20px] w-full truncate pr-9 text-sm",
              value ? "text-[var(--foreground)]" : "text-transparent"
            )}
          >
            {displayValue || " "}
          </span>

          <span className="pointer-events-none absolute right-4 top-1/2 -translate-y-1/2 text-[var(--muted-foreground)]">
            <CalendarIcon />
          </span>
        </button>

        {open ? (
          <div
            role="dialog"
            aria-label={label}
            onKeyDown={handleKeyDown}
            className="absolute left-0 top-[calc(100%+12px)] z-50 w-[360px] max-w-[calc(100vw-32px)] rounded-[24px] border border-[color:var(--border)] bg-[color:var(--surface)] p-4 shadow-[0_24px_80px_rgba(0,0,0,0.45)]"
          >
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <button
                  type="button"
                  onClick={() =>
                    setViewDate(
                      new Date(viewDate.getFullYear(), viewDate.getMonth() - 1, 1)
                    )
                  }
                  className="flex h-9 w-9 items-center justify-center rounded-xl border border-[var(--input-border)] bg-[var(--input-background)] text-[var(--foreground)] transition-all hover:border-[var(--input-border-hover)] hover:bg-[var(--input-background-hover)]"
                >
                  <ChevronLeftIcon />
                </button>

                <div className="text-sm font-semibold tracking-wide text-[var(--foreground)]">
                  {copy.months[viewDate.getMonth()]} {viewDate.getFullYear()}
                </div>

                <button
                  type="button"
                  onClick={() =>
                    setViewDate(
                      new Date(viewDate.getFullYear(), viewDate.getMonth() + 1, 1)
                    )
                  }
                  className="flex h-9 w-9 items-center justify-center rounded-xl border border-[var(--input-border)] bg-[var(--input-background)] text-[var(--foreground)] transition-all hover:border-[var(--input-border-hover)] hover:bg-[var(--input-background-hover)]"
                >
                  <ChevronRightIcon />
                </button>
              </div>

              <div className="grid grid-cols-7 gap-1.5">
                {copy.weekDays.map((day) => (
                  <div
                    key={day}
                    className="flex h-7 items-center justify-center text-[11px] font-semibold uppercase tracking-wide text-[var(--muted-foreground)]"
                  >
                    {day}
                  </div>
                ))}

                {days.map((day) => {
                  const isCurrentMonth = day.getMonth() === viewDate.getMonth();
                  const isSelected = selectedDate ? isSameDay(day, selectedDate) : false;
                  const isTodayValue = isSameDay(day, new Date());

                  return (
                    <button
                      key={`${day.getFullYear()}-${day.getMonth()}-${day.getDate()}`}
                      type="button"
                      onClick={() => handleSelectDay(day)}
                      className={cn(
                        "flex h-11 items-center justify-center rounded-xl text-sm font-medium transition-all",
                        isSelected
                          ? "bg-white text-black"
                          : "bg-[var(--input-background)] text-[var(--foreground)] hover:bg-[var(--input-background-hover)]",
                        !isCurrentMonth && !isSelected && "opacity-30",
                        isTodayValue &&
                          !isSelected &&
                          "ring-1 ring-[var(--ring)]"
                      )}
                    >
                      {day.getDate()}
                    </button>
                  );
                })}
              </div>

              <div className="rounded-[20px] border border-[color:var(--border)] bg-[var(--input-background)] p-3">
                <div className="mb-2 flex items-center gap-2 text-[11px] font-semibold uppercase tracking-wide text-[var(--muted-foreground)]">
                  <ClockIcon />
                  {copy.time}
                </div>

                <div className="grid grid-cols-[1fr_auto_1fr] items-end gap-2">
                  <CompactTimeField
                    label={copy.hours}
                    value={hour}
                    onChange={(next) => setHour(normalizeTwoDigits(next, 23) || "00")}
                    onIncrement={() => setHour((current) => increment(current, 23))}
                    onDecrement={() => setHour((current) => decrement(current, 23))}
                  />

                  <div className="pb-3 text-center text-xl font-semibold text-[var(--muted-foreground)]">
                    :
                  </div>

                  <CompactTimeField
                    label={copy.minutes}
                    value={minute}
                    onChange={(next) =>
                      setMinute(normalizeTwoDigits(next, 59) || "00")
                    }
                    onIncrement={() => setMinute((current) => increment(current, 59))}
                    onDecrement={() => setMinute((current) => decrement(current, 59))}
                  />
                </div>
              </div>

              <div className="border-t border-[color:var(--border)] pt-3">
                <div className="mb-3 text-sm text-[var(--muted-foreground)]">
                  {selectedDate
                    ? `${copy.selectedPrefix} ${new Intl.DateTimeFormat(copy.locale, {
                        day: "2-digit",
                        month: "2-digit",
                        year: "numeric",
                      }).format(selectedDate)} ${hour}:${minute}`
                    : copy.noDateSelected}
                </div>

                <div className="flex gap-3">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="h-11 flex-1"
                    onClick={clearValue}
                  >
                    {copy.clear}
                  </Button>

                  <Button
                    type="button"
                    size="sm"
                    className="h-11 flex-1"
                    onClick={applyValue}
                    disabled={!selectedDate}
                  >
                    {copy.apply}
                  </Button>
                </div>
              </div>
            </div>
          </div>
        ) : null}
      </div>

      {error ? (
        <p
          id={`${id}-error`}
          className="pt-2 text-sm text-[var(--status-danger-fg)]"
        >
          {error}
        </p>
      ) : helperText ? (
        <p
          id={`${id}-helper`}
          className="pt-2 text-sm text-[var(--muted-foreground)]"
        >
          {helperText}
        </p>
      ) : null}
    </div>
  );
}

function CompactTimeField({
  label,
  value,
  onChange,
  onIncrement,
  onDecrement,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  onIncrement: () => void;
  onDecrement: () => void;
}) {
  return (
    <div className="space-y-1.5">
      <div className="text-[11px] font-semibold uppercase tracking-wide text-[var(--muted-foreground)]">
        {label}
      </div>

      <div className="flex items-center gap-1.5">
        <MiniStepperButton onClick={onDecrement}>-</MiniStepperButton>

        <input
          value={value}
          inputMode="numeric"
          maxLength={2}
          onChange={(event) => onChange(event.target.value)}
          className={cn(
            "h-12 w-full rounded-xl border border-[var(--input-border)] bg-[var(--surface)] px-3 text-center text-lg font-semibold tracking-[0.08em] text-[var(--foreground)] outline-none transition-all",
            "hover:border-[var(--input-border-hover)] hover:bg-[var(--input-background-hover)]",
            "focus:border-[var(--ring)] focus:bg-[var(--input-background-focus)] focus:shadow-[0_0_0_1px_var(--ring),0_0_0_2px_rgba(113,113,122,0.08)]"
          )}
        />

        <MiniStepperButton onClick={onIncrement}>+</MiniStepperButton>
      </div>
    </div>
  );
}

function MiniStepperButton({
  children,
  onClick,
}: {
  children: ReactNode;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex h-12 w-10 items-center justify-center rounded-xl border border-[var(--input-border)] bg-[var(--surface)] text-base font-semibold text-[var(--foreground)] transition-all",
        "hover:border-[var(--input-border-hover)] hover:bg-[var(--input-background-hover)]",
        "focus-visible:outline-none focus-visible:border-[var(--ring)] focus-visible:shadow-[0_0_0_1px_var(--ring),0_0_0_2px_rgba(113,113,122,0.08)]"
      )}
    >
      {children}
    </button>
  );
}
