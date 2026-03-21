"use client";

import { ButtonHTMLAttributes } from "react";
import { useOptionalI18n } from "@/src/shared/lib/i18n";
import { cn } from "@/src/shared/lib/cn";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "default" | "secondary" | "outline" | "danger" | "ghost";
  size?: "sm" | "md" | "lg";
  loading?: boolean;
}

const variantClasses: Record<NonNullable<ButtonProps["variant"]>, string> = {
  default:
    "bg-[color:var(--button-primary-bg)] text-[color:var(--button-primary-fg)] hover:bg-[color:var(--button-primary-bg-hover)] disabled:hover:bg-[color:var(--button-primary-bg)]",

  secondary:
    "border border-[color:var(--button-secondary-border)] bg-[color:var(--button-secondary-bg)] text-[color:var(--button-secondary-fg)] hover:bg-[color:var(--button-secondary-bg-hover)] disabled:hover:bg-[color:var(--button-secondary-bg)]",

  outline:
    "border border-[color:var(--button-secondary-border)] bg-[color:var(--input-background)] text-[color:var(--button-secondary-fg)] hover:bg-[color:var(--button-secondary-bg)] disabled:hover:bg-[color:var(--input-background)]",

  danger: "bg-red-600 text-white hover:bg-red-500 disabled:hover:bg-red-600",

  ghost:
    "border-0 bg-[#161616] text-[color:var(--button-ghost-fg)] hover:bg-[#202020] hover:text-[color:var(--foreground)] disabled:bg-[#161616] disabled:hover:bg-[#161616] disabled:text-[color:var(--button-ghost-fg)]",
};

const sizeClasses: Record<NonNullable<ButtonProps["size"]>, string> = {
  sm: "h-14 px-4 text-base",
  md: "h-14 px-4 text-base",
  lg: "h-14 px-5 text-base",
};

export function Button({
  className,
  variant = "default",
  size = "md",
  loading = false,
  disabled,
  children,
  type = "button",
  ...props
}: ButtonProps) {
  const i18n = useOptionalI18n();
  const isDisabled = disabled || loading;
  const loadingLabel = i18n?.dictionary.common.loadingButton ?? "Loading...";

  return (
    <button
      type={type}
      className={cn(
        "inline-flex cursor-pointer select-none items-center justify-center gap-2 rounded-md font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--ring)] disabled:cursor-not-allowed disabled:select-none disabled:opacity-50",
        variantClasses[variant],
        sizeClasses[size],
        className
      )}
      disabled={isDisabled}
      aria-disabled={isDisabled}
      {...props}
    >
      {loading ? loadingLabel : children}
    </button>
  );
}
