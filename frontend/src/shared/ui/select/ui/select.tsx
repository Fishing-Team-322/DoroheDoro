"use client";

import {
  type KeyboardEvent,
  type ReactNode,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { cn } from "@/src/shared/lib/cn";

export interface SelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface SelectProps {
  value: string;
  onChange: (e: { target: { value: string } }) => void;
  options: ReadonlyArray<SelectOption>;
  placeholder?: string;
  className?: string;
  disabled?: boolean;
  selectSize?: "sm" | "md" | "lg";

  searchable?: boolean;
  searchPlaceholder?: string;

  emptyTitle?: string;
  emptyDescription?: string;
  emptyActionLabel?: string;
  onEmptyAction?: () => void;

  name?: string;
  id?: string;
  "aria-label"?: string;
  renderOption?: (option: SelectOption, selected: boolean) => ReactNode;
}

function getInitialActiveIndex(
  options: ReadonlyArray<SelectOption>,
  value: string
): number | null {
  const selectedIndex = options.findIndex(
    (option) => option.value === value && !option.disabled
  );

  if (selectedIndex >= 0) {
    return selectedIndex;
  }

  const firstEnabledIndex = options.findIndex((option) => !option.disabled);
  return firstEnabledIndex >= 0 ? firstEnabledIndex : null;
}

const triggerSizeClasses: Record<NonNullable<SelectProps["selectSize"]>, string> = {
  sm: "h-8 px-2 text-sm",
  md: "h-10 px-3 text-sm",
  lg: "h-11 px-4 text-base",
};

export function Select({
  value,
  onChange,
  options,
  placeholder = "Выберите значение",
  className,
  disabled = false,
  selectSize = "md",
  searchable = false,
  searchPlaceholder = "Поиск...",
  emptyTitle = "Ничего не найдено",
  emptyDescription = "Попробуйте изменить запрос или выбрать другой вариант.",
  emptyActionLabel = "Очистить поиск",
  onEmptyAction,
  name,
  id,
  renderOption,
  ...props
}: SelectProps) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [activeIndex, setActiveIndex] = useState<number | null>(null);

  const rootRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const optionRefs = useRef<Array<HTMLButtonElement | null>>([]);

  const GAP = 6;
  const FALLBACK_DROPDOWN_HEIGHT = 240;

  const [position, setPosition] = useState({
    top: 0,
    left: 0,
    width: 0,
  });

  const selectedOption = options.find((option) => option.value === value) ?? null;

  const filteredOptions =
    searchable && search.trim()
      ? options.filter((option) => {
          const query = search.trim().toLowerCase();
          return (
            option.label.toLowerCase().includes(query) ||
            option.value.toLowerCase().includes(query)
          );
        })
      : options;
  const initialActiveIndex = getInitialActiveIndex(filteredOptions, value);

  function getScrollParents(node: Element | null): Array<Element | Window> {
    const parents: Array<Element | Window> = [];
    let current: Element | null = node;

    while (current && current !== document.body) {
      const styles = getComputedStyle(current);
      const overflow = `${styles.overflow}${styles.overflowY}${styles.overflowX}`;
      if (/(auto|scroll|overlay)/.test(overflow)) {
        parents.push(current);
      }
      current = current.parentElement;
    }

    parents.push(window);
    return parents;
  }

  const updatePosition = () => {
    const trigger = buttonRef.current;
    if (!trigger) return;

    const triggerRect = trigger.getBoundingClientRect();
    const viewportHeight = window.innerHeight;
    const scrollY = window.scrollY;
    const scrollX = window.scrollX;

    const dropdownHeight =
      listRef.current?.getBoundingClientRect().height ?? FALLBACK_DROPDOWN_HEIGHT;

    const spaceBelow = viewportHeight - triggerRect.bottom - GAP;
    const spaceAbove = triggerRect.top - GAP;

    const shouldOpenUpward =
      spaceBelow < dropdownHeight && spaceAbove > spaceBelow;

    let top = 0;

    if (shouldOpenUpward) {
      top = triggerRect.top + scrollY - dropdownHeight - GAP;
      const minTop = scrollY + GAP;
      if (top < minTop) top = minTop;
    } else {
      top = triggerRect.bottom + scrollY + GAP;
      const maxBottom = scrollY + viewportHeight - GAP;
      const dropdownBottom = top + dropdownHeight;
      if (dropdownBottom > maxBottom) {
        top -= dropdownBottom - maxBottom;
      }
    }

    setPosition({
      top: Math.round(top),
      left: Math.round(triggerRect.left + scrollX),
      width: Math.round(triggerRect.width),
    });
  };

  const closeAndFocusTrigger = () => {
    setActiveIndex(null);
    setOpen(false);
    requestAnimationFrame(() => {
      buttonRef.current?.focus();
    });
  };

  const handleSelect = (nextValue: string) => {
    onChange({ target: { value: nextValue } });
    closeAndFocusTrigger();
  };

  const handleToggle = () => {
    if (disabled) return;

    setOpen((prev) => {
      const next = !prev;
      if (next && searchable) {
        setSearch("");
      }
      setActiveIndex(next ? initialActiveIndex : null);
      return next;
    });
  };

  const handleTriggerKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (disabled) return;

    if (
      event.key === "Enter" ||
      event.key === " " ||
      event.key === "ArrowDown" ||
      event.key === "ArrowUp"
    ) {
      event.preventDefault();
      setActiveIndex(initialActiveIndex);
      setOpen(true);
    }
  };

  const handleOptionKeyDown =
    (index: number, option: SelectOption) =>
    (event: KeyboardEvent<HTMLButtonElement>) => {
      if (event.key === "ArrowDown") {
        event.preventDefault();
        setActiveIndex((prev) => {
          if (prev == null) return 0;
          return Math.min(filteredOptions.length - 1, prev + 1);
        });
      }

      if (event.key === "ArrowUp") {
        event.preventDefault();
        setActiveIndex((prev) => {
          if (prev == null) return 0;
          return Math.max(0, prev - 1);
        });
      }

      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        if (!option.disabled) {
          handleSelect(option.value);
        }
      }

      if (event.key === "Escape") {
        event.preventDefault();
        closeAndFocusTrigger();
      }
    };

  useEffect(() => {
    const handlePointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (!rootRef.current) return;

      const clickedInsideTrigger = rootRef.current.contains(target);
      const clickedInsideDropdown = listRef.current?.contains(target);

      if (!clickedInsideTrigger && !clickedInsideDropdown) {
        setActiveIndex(null);
        setOpen(false);
      }
    };

    document.addEventListener("mousedown", handlePointerDown);
    return () => document.removeEventListener("mousedown", handlePointerDown);
  }, []);

  useEffect(() => {
    if (!open) return;

    if (searchable) {
      requestAnimationFrame(() => {
        searchInputRef.current?.focus();
      });
      return;
    }

    if (activeIndex == null) return;

    const activeElement = optionRefs.current[activeIndex];
    if (activeElement) {
      activeElement.focus();
    }
  }, [open, searchable, activeIndex, filteredOptions.length]);

  useLayoutEffect(() => {
    if (!open) return;

    updatePosition();

    const scrollParents = getScrollParents(buttonRef.current);
    const handleScroll = () => updatePosition();
    const handleResize = () => updatePosition();

    scrollParents.forEach((parent) => {
      parent.addEventListener("scroll", handleScroll as EventListener, {
        passive: true,
      });
    });

    window.addEventListener("resize", handleResize);

    const visibilityInterval = window.setInterval(() => {
      const rect = buttonRef.current?.getBoundingClientRect();
      if (!rect) return;

      const isOutOfViewport =
        rect.bottom < 0 ||
        rect.top > window.innerHeight ||
        rect.right < 0 ||
        rect.left > window.innerWidth;

      if (isOutOfViewport) {
        setActiveIndex(null);
        setOpen(false);
      }
    }, 150);

    return () => {
      scrollParents.forEach((parent) => {
        parent.removeEventListener("scroll", handleScroll as EventListener);
      });
      window.removeEventListener("resize", handleResize);
      window.clearInterval(visibilityInterval);
    };
  }, [open]);

  useLayoutEffect(() => {
    if (!open) return;
    updatePosition();
  }, [open, search, searchable, options.length]);

  const dropdown = typeof document !== "undefined" && open
    ? createPortal(
        <div
          ref={listRef}
          role="listbox"
          aria-label={props["aria-label"] ?? placeholder}
          style={{
            position: "absolute",
            top: position.top,
            left: position.left,
            width: position.width,
            zIndex: 10000,
          }}
          className={cn(
            "overflow-hidden rounded-xl border border-[color:var(--border)]",
            "bg-[color:var(--surface)] shadow-[0_18px_48px_rgba(0,0,0,0.45)]"
          )}
        >
          {searchable ? (
            <div className="border-b border-[color:var(--border)] bg-[color:var(--surface)] p-2">
              <div className="flex items-center gap-2 rounded-lg border border-[color:var(--input-border)] bg-[color:var(--input-background)] px-3">
                <SearchIcon className="h-4 w-4 shrink-0 text-[color:var(--muted-foreground)]" />
                <input
                  ref={searchInputRef}
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                  placeholder={searchPlaceholder}
                  className={cn(
                    "h-9 w-full border-0 bg-transparent text-sm text-[color:var(--foreground)] outline-none",
                    "placeholder:text-[color:var(--muted-foreground)]"
                  )}
                />
              </div>
            </div>
          ) : null}

          <div className="max-h-60 overflow-auto">
            {filteredOptions.length === 0 ? (
              <div className="px-4 py-6">
                <div className="flex min-h-[136px] flex-col items-center justify-center gap-2 text-center">
                  <div className="text-sm font-medium text-[color:var(--foreground)]">
                    {emptyTitle}
                  </div>

                  <div className="max-w-[26rem] text-sm text-[color:var(--muted-foreground)]">
                    {emptyDescription}
                  </div>

                  <button
                    type="button"
                    onClick={() => {
                      setSearch("");
                      requestAnimationFrame(() => {
                        searchInputRef.current?.focus();
                      });
                      onEmptyAction?.();
                    }}
                    className={cn(
                      "mt-2 inline-flex h-9 items-center justify-center rounded-lg border",
                      "border-[color:var(--button-secondary-border)] bg-[color:var(--button-secondary-bg)]",
                      "px-3 text-sm font-medium text-[color:var(--button-secondary-fg)] transition-colors",
                      "hover:bg-[color:var(--button-secondary-bg-hover)]"
                    )}
                  >
                    {emptyActionLabel}
                  </button>
                </div>
              </div>
            ) : (
              filteredOptions.map((option, index) => {
                const isSelected = option.value === value;
                const isDisabled = !!option.disabled;

                return (
                  <button
                    key={option.value}
                    ref={(element) => {
                      optionRefs.current[index] = element;
                    }}
                    type="button"
                    role="option"
                    aria-selected={isSelected}
                    disabled={isDisabled}
                    onClick={() => {
                      if (!isDisabled) {
                        handleSelect(option.value);
                      }
                    }}
                    onKeyDown={handleOptionKeyDown(index, option)}
                    className={cn(
                      "flex w-full items-center justify-between gap-3 border-t border-[color:var(--border)] px-3 py-2.5 text-left text-sm transition-colors",
                      index === 0 && "border-t-0",
                      isDisabled
                        ? "cursor-not-allowed opacity-50"
                        : "cursor-pointer hover:bg-[color:var(--surface-elevated)]",
                      isSelected &&
                        "bg-[color:var(--surface-elevated)] text-[color:var(--foreground)]",
                      "focus:outline-none focus-visible:bg-[color:var(--surface-elevated)]"
                    )}
                  >
                    <span className="truncate">
                      {renderOption ? renderOption(option, isSelected) : option.label}
                    </span>

                    {isSelected ? (
                      <CheckIcon className="h-4 w-4 shrink-0 text-[color:var(--foreground)]" />
                    ) : null}
                  </button>
                );
              })
            )}
          </div>
        </div>,
        document.body
      )
    : null;

  return (
    <div ref={rootRef} className={cn("relative w-full", className)}>
      {name ? <input type="hidden" name={name} value={value} /> : null}

      <button
        ref={buttonRef}
        id={id}
        type="button"
        disabled={disabled}
        aria-expanded={open}
        aria-haspopup="listbox"
        aria-label={props["aria-label"] ?? (selectedOption
          ? `Выбрано: ${selectedOption.label}`
          : placeholder)}
        onClick={handleToggle}
        onKeyDown={handleTriggerKeyDown}
        className={cn(
          "flex w-full items-center justify-between rounded-md border",
          "border-[color:var(--input-border)] bg-[color:var(--input-background)]",
          "text-[color:var(--foreground)] transition-[border-color,box-shadow,background-color]",
          "hover:border-[color:var(--input-border-hover)]",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--ring)]",
          "disabled:cursor-not-allowed disabled:opacity-50",
          triggerSizeClasses[selectSize],
          open && "border-[color:var(--input-border-hover)] ring-2 ring-[color:var(--ring)]"
        )}
      >
        <span
          className={cn(
            "truncate text-left",
            selectedOption ? "text-[color:var(--foreground)]" : "text-[color:var(--muted-foreground)]"
          )}
        >
          {selectedOption ? selectedOption.label : placeholder}
        </span>

        <ChevronDownIcon
          className={cn(
            "ml-2 h-4 w-4 shrink-0 text-[color:var(--muted-foreground)] transition-transform duration-200",
            open && "rotate-180"
          )}
        />
      </button>

      {dropdown}
    </div>
  );
}

function SearchIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      className={className}
      aria-hidden="true"
    >
      <circle cx="9" cy="9" r="4.5" />
      <path d="M12.5 12.5L16 16" strokeLinecap="round" />
    </svg>
  );
}

function CheckIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
    >
      <path d="M4 10.5L8 14.5L16 6.5" />
    </svg>
  );
}

function ChevronDownIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      className={className}
      aria-hidden="true"
    >
      <path d="M6 8l4 4 4-4" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}
