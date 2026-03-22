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
import { AnimatePresence, motion } from "motion/react";
import { useOptionalI18n } from "@/src/shared/lib/i18n";
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
  triggerClassName?: string;
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

const triggerSizeClasses: Record<
  NonNullable<SelectProps["selectSize"]>,
  string
> = {
  sm: "h-14 px-2 text-base",
  md: "h-14 px-3 text-base",
  lg: "h-14 px-4 text-base",
};

export function Select({
  value,
  onChange,
  options,
  placeholder,
  className,
  triggerClassName,
  disabled = false,
  selectSize = "md",
  searchable = false,
  searchPlaceholder,
  emptyTitle,
  emptyDescription,
  emptyActionLabel,
  onEmptyAction,
  name,
  id,
  renderOption,
  ...props
}: SelectProps) {
  const i18n = useOptionalI18n();
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [activeIndex, setActiveIndex] = useState<number | null>(null);
  const [openDirection, setOpenDirection] = useState<"up" | "down">("down");

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

  const resolvedPlaceholder =
    placeholder ?? i18n?.dictionary.select.placeholder ?? "Select a value";
  const resolvedSearchPlaceholder =
    searchPlaceholder ??
    i18n?.dictionary.select.searchPlaceholder ??
    "Search...";
  const resolvedEmptyTitle =
    emptyTitle ?? i18n?.dictionary.select.emptyTitle ?? "Nothing found";
  const resolvedEmptyDescription =
    emptyDescription ??
    i18n?.dictionary.select.emptyDescription ??
    "Try changing the query or choosing another option.";
  const resolvedEmptyActionLabel =
    emptyActionLabel ?? i18n?.dictionary.select.emptyAction ?? "Clear search";

  const selectedOption =
    options.find((option) => option.value === value) ?? null;

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
      listRef.current?.getBoundingClientRect().height ??
      FALLBACK_DROPDOWN_HEIGHT;

    const spaceBelow = viewportHeight - triggerRect.bottom - GAP;
    const spaceAbove = triggerRect.top - GAP;

    const shouldOpenUpward =
      spaceBelow < dropdownHeight && spaceAbove > spaceBelow;

    setOpenDirection(shouldOpenUpward ? "up" : "down");

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

    const frame = requestAnimationFrame(() => {
      updatePosition();
    });

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
      window.cancelAnimationFrame(frame);
      scrollParents.forEach((parent) => {
        parent.removeEventListener("scroll", handleScroll as EventListener);
      });
      window.removeEventListener("resize", handleResize);
      window.clearInterval(visibilityInterval);
    };
  }, [open]);

  useLayoutEffect(() => {
    if (!open) return;

    const frame = requestAnimationFrame(() => {
      updatePosition();
    });

    return () => {
      window.cancelAnimationFrame(frame);
    };
  }, [open, search, searchable, options.length]);

  const dropdownAnimation =
    openDirection === "up"
      ? {
          initial: { opacity: 0, scale: 0.98, y: 8 },
          animate: { opacity: 1, scale: 1, y: 0 },
          exit: { opacity: 0, scale: 0.98, y: 8 },
          origin: "bottom center",
        }
      : {
          initial: { opacity: 0, scale: 0.98, y: -8 },
          animate: { opacity: 1, scale: 1, y: 0 },
          exit: { opacity: 0, scale: 0.98, y: -8 },
          origin: "top center",
        };

  const dropdown =
    typeof document !== "undefined"
      ? createPortal(
          <AnimatePresence>
            {open ? (
              <motion.div
                ref={listRef}
                role="listbox"
                aria-label={props["aria-label"] ?? resolvedPlaceholder}
                initial={dropdownAnimation.initial}
                animate={dropdownAnimation.animate}
                exit={dropdownAnimation.exit}
                transition={{
                  duration: 0.18,
                  ease: [0.22, 1, 0.36, 1],
                }}
                style={{
                  position: "absolute",
                  top: position.top,
                  left: position.left,
                  width: position.width,
                  zIndex: 10000,
                  transformOrigin: dropdownAnimation.origin,
                }}
                className={cn(
                  "overflow-hidden rounded-md border border-[color:var(--border)]",
                  "bg-[color:var(--surface)] shadow-[0_18px_48px_rgba(0,0,0,0.45)]"
                )}
              >
                {searchable ? (
                  <div className="border-b border-[color:var(--border)] bg-black p-2">
                    <div className="flex items-center gap-2 rounded-lg border border-[color:var(--input-border)] bg-[color:var(--input-background)] px-3">
                      <SearchIcon className="h-4 w-4 shrink-0 text-[color:var(--muted-foreground)]" />
                      <input
                        ref={searchInputRef}
                        value={search}
                        onChange={(event) => setSearch(event.target.value)}
                        placeholder={resolvedSearchPlaceholder}
                        className={cn(
                          "h-9 w-full border-0 bg-transparent text-base text-white outline-none",
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
                        <div className="text-base font-medium text-white">
                          {resolvedEmptyTitle}
                        </div>

                        <div className="max-w-[26rem] text-base text-[color:var(--muted-foreground)]">
                          {resolvedEmptyDescription}
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
                            "px-3 text-base font-medium text-[color:var(--button-secondary-fg)] transition-colors",
                            "hover:bg-[color:var(--button-secondary-bg-hover)]"
                          )}
                        >
                          {resolvedEmptyActionLabel}
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
                            "flex h-10 w-full items-center px-4 text-left text-base transition-colors focus:outline-none",
                            isDisabled
                              ? "cursor-not-allowed opacity-50 text-[rgba(255,255,255,0.45)]"
                              : "cursor-pointer text-[rgba(255,255,255,0.78)] hover:text-white",
                            isSelected && "bg-white !text-black",
                            !isSelected && "focus-visible:text-white"
                          )}
                        >
                          <span className="truncate">
                            {renderOption
                              ? renderOption(option, isSelected)
                              : option.label}
                          </span>
                        </button>
                      );
                    })
                  )}
                </div>
              </motion.div>
            ) : null}
          </AnimatePresence>,
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
        aria-label={
          props["aria-label"] ??
          (selectedOption ? selectedOption.label : resolvedPlaceholder)
        }
        onClick={handleToggle}
        onKeyDown={handleTriggerKeyDown}
        className={cn(
          "flex w-full items-center justify-between rounded-md border-0",
          "bg-[color:var(--input-background)]",
          "text-[color:var(--foreground)] transition-colors",
          "hover:bg-[rgba(255,255,255,0.06)]",
          "focus-visible:outline-none",
          "disabled:cursor-not-allowed disabled:opacity-50",
          triggerSizeClasses[selectSize],
          open && "bg-[rgba(255,255,255,0.06)]",
          triggerClassName
        )}
      >
        <span
          className={cn(
            "truncate text-left",
            selectedOption
              ? "text-[color:var(--foreground)]"
              : "text-[color:var(--muted-foreground)]"
          )}
        >
          {selectedOption ? selectedOption.label : resolvedPlaceholder}
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