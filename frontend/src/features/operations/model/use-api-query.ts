"use client";

import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type DependencyList,
} from "react";
import { isApiError, type ApiError, type ApiResponseMeta } from "@/src/shared/lib/api";

type QueryResult<T> = {
  data: T;
  meta: ApiResponseMeta;
};

type UseApiQueryOptions<T> = {
  enabled?: boolean;
  deps?: DependencyList;
  keepPreviousData?: boolean;
  pollIntervalMs?: number;
  initialData?: T;
  queryFn: (signal: AbortSignal) => Promise<QueryResult<T>>;
};

export type ApiQueryState<T> = {
  data?: T;
  meta?: ApiResponseMeta;
  error?: ApiError;
  isLoading: boolean;
  isRefreshing: boolean;
  lastUpdatedAt?: number;
  refetch: (options?: { silent?: boolean }) => Promise<void>;
};

export function useApiQuery<T>({
  enabled = true,
  deps = [],
  keepPreviousData = true,
  pollIntervalMs,
  initialData,
  queryFn,
}: UseApiQueryOptions<T>): ApiQueryState<T> {
  const [data, setData] = useState<T | undefined>(initialData);
  const [meta, setMeta] = useState<ApiResponseMeta>();
  const [error, setError] = useState<ApiError>();
  const [isLoading, setIsLoading] = useState(!initialData && enabled);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [lastUpdatedAt, setLastUpdatedAt] = useState<number>();
  const abortRef = useRef<AbortController | null>(null);
  const queryFnRef = useRef(queryFn);

  const dependencyKey = JSON.stringify(deps);

  useEffect(() => {
    queryFnRef.current = queryFn;
  }, [queryFn]);

  const runQuery = useCallback(
    async (silent = false) => {
      if (!enabled) {
        return;
      }

      abortRef.current?.abort();
      const controller = new AbortController();
      abortRef.current = controller;

      if (silent) {
        setIsRefreshing(true);
      } else {
        setIsLoading(true);
        setIsRefreshing(false);
        setError(undefined);
        if (!keepPreviousData) {
          setData(undefined);
          setMeta(undefined);
        }
      }

      try {
        const result = await queryFnRef.current(controller.signal);

        if (controller.signal.aborted) {
          return;
        }

        setData(result.data);
        setMeta(result.meta);
        setError(undefined);
        setLastUpdatedAt(Date.now());
      } catch (caughtError) {
        if (controller.signal.aborted) {
          return;
        }

        if (
          isApiError(caughtError) &&
          caughtError.code === "REQUEST_ABORTED"
        ) {
          return;
        }

        setError(
          isApiError(caughtError)
            ? caughtError
            : ({
                name: "ApiError",
                message:
                  caughtError instanceof Error
                    ? caughtError.message
                    : "Unexpected error",
                status: null,
                code: "UNKNOWN_ERROR",
                cause: caughtError,
              } as ApiError)
        );
      } finally {
        if (!controller.signal.aborted) {
          setIsLoading(false);
          setIsRefreshing(false);
        }
      }
    },
    [enabled, keepPreviousData]
  );

  useEffect(() => {
    if (!enabled) {
      setIsLoading(false);
      setIsRefreshing(false);
      abortRef.current?.abort();
      return;
    }

    void runQuery(false);

    return () => {
      abortRef.current?.abort();
    };
  }, [dependencyKey, enabled, keepPreviousData, runQuery]);

  useEffect(() => {
    if (!enabled || !pollIntervalMs) {
      return;
    }

    const interval = window.setInterval(() => {
      void runQuery(true);
    }, pollIntervalMs);

    return () => {
      window.clearInterval(interval);
    };
  }, [dependencyKey, enabled, pollIntervalMs, runQuery]);

  return {
    data,
    meta,
    error,
    isLoading,
    isRefreshing,
    lastUpdatedAt,
    refetch: async (options) => {
      await runQuery(options?.silent ?? false);
    },
  };
}
