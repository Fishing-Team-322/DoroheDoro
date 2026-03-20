"use client";

import { cn } from "@/src/shared/lib/cn";
import { Button } from "@/src/shared/ui/button";

interface PaginationProps {
  page: number;
  totalPages: number;
  disabled?: boolean;
  onPageChange: (page: number) => void;
  className?: string;
}

export function Pagination({
  page,
  totalPages,
  disabled,
  onPageChange,
  className,
}: PaginationProps) {
  const canPrev = page > 1;
  const canNext = page < totalPages;

  return (
    <div className={cn("flex items-center gap-3", className)}>
      <Button
        variant="outline"
        size="sm"
        disabled={disabled || !canPrev}
        onClick={() => onPageChange(page - 1)}
      >
        Назад
      </Button>
      <span className="text-sm text-gray-700">
        {page} / {totalPages}
      </span>
      <Button
        variant="outline"
        size="sm"
        disabled={disabled || !canNext}
        onClick={() => onPageChange(page + 1)}
      >
        Вперёд
      </Button>
    </div>
  );
}
