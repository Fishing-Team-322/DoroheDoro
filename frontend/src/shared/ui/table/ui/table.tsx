"use client";

import {
  HTMLAttributes,
  TableHTMLAttributes,
  TdHTMLAttributes,
  ThHTMLAttributes,
} from "react";
import { motion } from "motion/react";
import { cn } from "@/src/shared/lib/cn";

export function Table({
  className,
  ...props
}: TableHTMLAttributes<HTMLTableElement>) {
  return (
    <table
      className={cn("w-full caption-bottom border-collapse text-sm", className)}
      {...props}
    />
  );
}

export function TableHeader({
  className,
  ...props
}: HTMLAttributes<HTMLTableSectionElement>) {
  return (
    <thead
      className={cn("border-b border-[color:var(--border)]", className)}
      {...props}
    />
  );
}

export function TableBody({
  className,
  ...props
}: HTMLAttributes<HTMLTableSectionElement>) {
  return (
    <tbody className={cn("[&_tr:last-child]:border-0", className)} {...props} />
  );
}

export function TableRow({
  className,
  ...props
}: HTMLAttributes<HTMLTableRowElement>) {
  return (
    <tr
      className={cn(
        "border-b border-[color:var(--border)] transition-colors hover:bg-[color:var(--surface)]",
        className
      )}
      {...props}
    />
  );
}

export function TableHead({
  className,
  ...props
}: ThHTMLAttributes<HTMLTableCellElement>) {
  return (
    <th
      className={cn(
        "h-14 px-3 text-left align-middle text-sm font-semibold tracking-wide text-[color:var(--muted-foreground)] md:text-lg",
        "[&_button]:text-[color:var(--muted-foreground)] [&_button]:transition-colors",
        "[&_button:hover]:text-white",
        className
      )}
      {...props}
    />
  );
}

export function TableCell({
  className,
  ...props
}: TdHTMLAttributes<HTMLTableCellElement>) {
  return <td className={cn("p-3 align-middle", className)} {...props} />;
}

export function TableSortButton({
  children,
  active = false,
  direction = "desc",
  className,
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement> & {
  active?: boolean;
  direction?: "asc" | "desc";
}) {
  return (
    <button
      type="button"
      className={cn(
        "group inline-flex items-center gap-2 text-left text-inherit",
        active && "text-white",
        className
      )}
      {...props}
    >
      <span>{children}</span>
      <TableSortIcon active={active} direction={direction} />
    </button>
  );
}

export function TableSortIcon({
  active,
  direction,
  className,
}: {
  active: boolean;
  direction: "asc" | "desc";
  className?: string;
}) {
  if (!active) return null;

  return (
    <motion.svg
      width="16"
      height="16"
      viewBox="0 0 20 20"
      fill="none"
      initial={false}
      animate={{ rotate: direction === "asc" ? 180 : 0, opacity: 1 }}
      transition={{ duration: 0.18, ease: [0.22, 1, 0.36, 1] }}
      className={cn("shrink-0 text-white", className)}
      aria-hidden="true"
    >
      <path
        d="M6 8l4 4 4-4"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </motion.svg>
  );
}