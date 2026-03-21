"use client";

import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { cn } from "@/src/shared/lib/cn";

type ToastVariant = "default" | "success" | "danger";

type ToastInput = {
  title: string;
  description?: string;
  variant?: ToastVariant;
  durationMs?: number;
};

type ToastRecord = ToastInput & {
  id: number;
};

type ToastContextValue = {
  showToast: (input: ToastInput) => void;
};

const ToastContext = createContext<ToastContextValue | null>(null);

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastRecord[]>([]);

  const contextValue = useMemo<ToastContextValue>(() => {
    return {
      showToast: (input) => {
        const id = Date.now() + Math.floor(Math.random() * 1000);
        setToasts((current) => [...current, { id, ...input }]);
      },
    };
  }, []);

  return (
    <ToastContext.Provider value={contextValue}>
      {children}
      <div className="pointer-events-none fixed right-4 top-4 z-[120] flex w-full max-w-sm flex-col gap-3">
        {toasts.map((toast) => (
          <ToastItem
            key={toast.id}
            toast={toast}
            onClose={() => {
              setToasts((current) =>
                current.filter((item) => item.id !== toast.id)
              );
            }}
          />
        ))}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast() {
  const context = useContext(ToastContext);

  if (!context) {
    throw new Error("useToast must be used within a ToastProvider");
  }

  return context;
}

function ToastItem({
  toast,
  onClose,
}: {
  toast: ToastRecord;
  onClose: () => void;
}) {
  useEffect(() => {
    const timeout = window.setTimeout(onClose, toast.durationMs ?? 4_000);
    return () => window.clearTimeout(timeout);
  }, [onClose, toast.durationMs]);

  return (
    <div
      className={cn(
        "pointer-events-auto rounded-xl border px-4 py-3 shadow-[0_16px_40px_rgba(0,0,0,0.35)] backdrop-blur",
        toast.variant === "success" &&
          "border-[color:var(--status-positive-border)] bg-[color:var(--status-positive-bg)] text-[color:var(--status-positive-fg)]",
        toast.variant === "danger" &&
          "border-[color:var(--status-danger-border)] bg-[color:var(--status-danger-bg)] text-[color:var(--status-danger-fg)]",
        (!toast.variant || toast.variant === "default") &&
          "border-[color:var(--border)] bg-[color:var(--surface-elevated)] text-[color:var(--foreground)]"
      )}
      role="status"
      aria-live="polite"
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <p className="text-sm font-semibold">{toast.title}</p>
          {toast.description ? (
            <p className="mt-1 text-xs text-inherit/80">{toast.description}</p>
          ) : null}
        </div>

        <button
          type="button"
          onClick={onClose}
          className="inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-full text-inherit/70 transition-colors hover:bg-white/10 hover:text-inherit"
          aria-label="Dismiss notification"
        >
          <svg
            viewBox="0 0 20 20"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.8"
            className="h-4 w-4"
            aria-hidden="true"
          >
            <path d="m6 6 8 8M14 6l-8 8" strokeLinecap="round" />
          </svg>
        </button>
      </div>
    </div>
  );
}
