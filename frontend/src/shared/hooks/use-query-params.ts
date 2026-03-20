"use client";

import { useMemo } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import {
  QueryParamsState,
  buildQueryString,
  readQueryParams,
  removeQueryParams,
  updateQueryParams,
} from "@/src/shared/lib/query-params";

type QueryUpdateInput = Partial<
  Record<keyof QueryParamsState, string | number | null | undefined>
>;

export function useQueryParams() {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();

  const query = useMemo(() => readQueryParams(searchParams), [searchParams]);

  const pushQuery = (nextParams: URLSearchParams) => {
    const queryString = nextParams.toString();
    const nextUrl = queryString ? `${pathname}?${queryString}` : pathname;
    router.replace(nextUrl, { scroll: false });
  };

  const setParam = (
    key: keyof QueryParamsState,
    value: string | number | null
  ) => {
    pushQuery(updateQueryParams(searchParams, { [key]: value }));
  };

  const setParams = (updates: QueryUpdateInput) => {
    pushQuery(updateQueryParams(searchParams, updates));
  };

  const removeParams = (keys: (keyof QueryParamsState)[]) => {
    pushQuery(removeQueryParams(searchParams, keys));
  };

  const toQueryString = (updates?: QueryUpdateInput) =>
    buildQueryString(searchParams, updates);

  return {
    query,
    setParam,
    setParams,
    removeParams,
    toQueryString,
  };
}
