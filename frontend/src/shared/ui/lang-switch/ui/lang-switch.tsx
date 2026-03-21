"use client";

import Image from "next/image";
import { usePathname, useRouter } from "next/navigation";
import { useTransition } from "react";
import * as Tooltip from "@radix-ui/react-tooltip";
import type { Locale } from "@/src/shared/config";
import { cn } from "@/src/shared/lib/cn";
import { replacePathLocale } from "@/src/shared/lib/i18n";

type DashboardSidebarLanguageSwitchProps = {
  locale: Locale;
  collapsed: boolean;
  onClick?: () => void;
};

const BUTTON_SIZE = 36;
const TRACK_GAP = 2;
const TRACK_PADDING = 2;
const TRACK_HEIGHT = BUTTON_SIZE + TRACK_PADDING * 2;
const TRACK_WIDTH = BUTTON_SIZE * 2 + TRACK_GAP + TRACK_PADDING * 2;
const FLAG_SIZE = 24;

export function DashboardSidebarLanguageSwitch({
  locale,
  collapsed,
  onClick,
}: DashboardSidebarLanguageSwitchProps) {
  const router = useRouter();
  const pathname = usePathname();
  const [isPending, startTransition] = useTransition();

  const normalizedLocale = String(locale).toLowerCase();
  const currentLocale = normalizedLocale === "en" ? "en" : "ru";
  const nextLocale = (currentLocale === "en" ? "ru" : "en") as Locale;

  const handleChangeLocale = (targetLocale: Locale) => {
    if (String(targetLocale).toLowerCase() === normalizedLocale) {
      return;
    }

    const nextPathname = replacePathLocale(pathname, targetLocale);

    startTransition(() => {
      router.push(nextPathname);
      onClick?.();
    });
  };

  const content = (
    <div className="h-14 overflow-visible">
      <div className="flex h-full items-center justify-start">
        <div
          className="relative"
          style={{
            width: `${TRACK_WIDTH}px`,
            height: `${TRACK_HEIGHT}px`,
          }}
        >
          <div
            className={cn(
              "absolute inset-0 transition-[opacity,transform] duration-250 ease-[cubic-bezier(0.22,1,0.36,1)]",
              collapsed
                ? "pointer-events-none scale-95 opacity-0"
                : "pointer-events-auto scale-100 opacity-100"
            )}
          >
            <LanguageFlagSwitch
              value={currentLocale}
              disabled={isPending}
              onChange={(value) => handleChangeLocale(value as Locale)}
            />
          </div>

          <div
            className={cn(
              "absolute left-0 top-1/2 -translate-y-1/2 transition-[opacity,transform] duration-250 ease-[cubic-bezier(0.22,1,0.36,1)]",
              collapsed
                ? "pointer-events-auto scale-100 opacity-100"
                : "pointer-events-none scale-95 opacity-0"
            )}
          >
            <CompactFlagButton
              locale={currentLocale}
              disabled={isPending}
              onClick={() => handleChangeLocale(nextLocale)}
            />
          </div>
        </div>
      </div>
    </div>
  );

  if (!collapsed) {
    return content;
  }

  return (
    <Tooltip.Provider delayDuration={100}>
      <Tooltip.Root>
        <Tooltip.Trigger asChild>{content}</Tooltip.Trigger>

        <Tooltip.Portal>
          <Tooltip.Content
            side="right"
            align="center"
            sideOffset={12}
            collisionPadding={8}
            className="z-[9999] whitespace-nowrap rounded-md bg-[color:var(--surface-elevated)] px-3 py-2 text-sm font-medium leading-none text-white shadow-[0_12px_30px_rgba(0,0,0,0.35)]"
          >
            {currentLocale === "ru" ? "Язык" : "Language"}
            <Tooltip.Arrow className="fill-[color:var(--surface-elevated)]" />
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
    </Tooltip.Provider>
  );
}

type LanguageFlagSwitchProps = {
  value: "ru" | "en";
  disabled?: boolean;
  onChange: (value: "ru" | "en") => void;
};

function LanguageFlagSwitch({
  value,
  disabled,
  onChange,
}: LanguageFlagSwitchProps) {
  return (
    <div
      role="tablist"
      aria-label="Language switch"
      className={cn(
        "inline-flex items-center rounded-[14px] border border-[color:var(--border)] bg-[rgba(255,255,255,0.04)] shadow-[inset_0_1px_0_rgba(255,255,255,0.05)]",
        disabled && "pointer-events-none opacity-60"
      )}
      style={{
        width: `${TRACK_WIDTH}px`,
        height: `${TRACK_HEIGHT}px`,
        padding: `${TRACK_PADDING}px`,
        gap: `${TRACK_GAP}px`,
      }}
    >
      <FlagSegmentButton
        active={value === "ru"}
        disabled={disabled}
        imageSrc="/img/ru.png"
        imageAlt="Russian"
        onClick={() => onChange("ru")}
      />

      <FlagSegmentButton
        active={value === "en"}
        disabled={disabled}
        imageSrc="/img/en.png"
        imageAlt="English"
        onClick={() => onChange("en")}
      />
    </div>
  );
}

type FlagSegmentButtonProps = {
  active: boolean;
  disabled?: boolean;
  imageSrc: string;
  imageAlt: string;
  onClick: () => void;
};

function FlagSegmentButton({
  active,
  disabled,
  imageSrc,
  imageAlt,
  onClick,
}: FlagSegmentButtonProps) {
  return (
    <button
      type="button"
      role="tab"
      aria-selected={active}
      disabled={disabled}
      onClick={onClick}
      className={cn(
        "inline-flex shrink-0 items-center justify-center rounded-[11px] border transition-all duration-200",
        active
          ? "border-white/10 bg-[rgba(255,255,255,0.18)] shadow-[0_6px_16px_rgba(0,0,0,0.22)] opacity-100"
          : "border-transparent bg-transparent opacity-65 hover:opacity-90"
      )}
      style={{
        width: `${BUTTON_SIZE}px`,
        height: `${BUTTON_SIZE}px`,
      }}
    >
      <Image
        src={imageSrc}
        alt={imageAlt}
        width={FLAG_SIZE}
        height={FLAG_SIZE}
        className="rounded-full object-cover"
        style={{
          width: `${FLAG_SIZE}px`,
          height: `${FLAG_SIZE}px`,
        }}
      />
    </button>
  );
}

function CompactFlagButton({
  locale,
  disabled,
  onClick,
}: {
  locale: "ru" | "en";
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      aria-label={locale === "ru" ? "Переключить язык" : "Switch language"}
      className={cn(
        "inline-flex items-center justify-center rounded-[11px] border border-white/10 bg-[rgba(255,255,255,0.18)] shadow-[0_6px_16px_rgba(0,0,0,0.22)] transition-colors",
        disabled && "pointer-events-none opacity-60"
      )}
      style={{
        width: `${BUTTON_SIZE}px`,
        height: `${BUTTON_SIZE}px`,
      }}
    >
      <Image
        src={locale === "ru" ? "/img/ru.png" : "/img/en.png"}
        alt={locale === "ru" ? "Russian" : "English"}
        width={FLAG_SIZE}
        height={FLAG_SIZE}
        className="rounded-full object-cover"
        style={{
          width: `${FLAG_SIZE}px`,
          height: `${FLAG_SIZE}px`,
        }}
      />
    </button>
  );
}
