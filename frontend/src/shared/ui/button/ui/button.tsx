"use client";

import { ButtonHTMLAttributes } from "react";
import { cn } from "@/src/shared/lib/cn";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "default" | "secondary" | "outline" | "danger" | "ghost";
  size?: "sm" | "md" | "lg";
  loading?: boolean;
}

const variantClasses: Record<NonNullable<ButtonProps["variant"]>, string> = {
  default:
    "bg-[color:var(--button-primary-bg)] text-[color:var(--button-primary-fg)] hover:bg-[color:var(--button-primary-bg-hover)]",
  secondary:
    "border border-[color:var(--button-secondary-border)] bg-[color:var(--button-secondary-bg)] text-[color:var(--button-secondary-fg)] hover:bg-[color:var(--button-secondary-bg-hover)]",
  outline:
    "border border-[color:var(--button-secondary-border)] bg-[color:var(--input-background)] text-[color:var(--button-secondary-fg)] hover:bg-[color:var(--button-secondary-bg)]",
  danger: "bg-red-600 text-white hover:bg-red-500",
  ghost:
    "text-[color:var(--button-ghost-fg)] hover:bg-[color:var(--button-ghost-bg-hover)] hover:text-[color:var(--foreground)]",
};

const sizeClasses: Record<NonNullable<ButtonProps["size"]>, string> = {
  sm: "h-8 px-3 text-sm",
  md: "h-10 px-4 text-sm",
  lg: "h-11 px-5 text-base",
};

export function Button({
  className,
  variant = "default",
  size = "md",
  loading = false,
  disabled,
  children,
  ...props
}: ButtonProps) {
  const isDisabled = disabled || loading;

  return (
    <button
      className={cn(
        "inline-flex items-center justify-center gap-2 rounded-md font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--ring)] disabled:pointer-events-none disabled:opacity-50",
        variantClasses[variant],
        sizeClasses[size],
        className
      )}
      disabled={isDisabled}
      {...props}
    >
      {loading ? "Loading..." : children}
    </button>
  );
}

