export type SortDirection = "asc" | "desc";

type SearchParamsLike = Pick<URLSearchParams, "get" | "toString">;

export type QueryParamsState = {
  search: string;
  page: number;
  sort: string;
  filter: string;
};

export const QUERY_DEFAULTS: QueryParamsState = {
  search: "",
  page: 1,
  sort: "name:asc",
  filter: "all",
};

const numericPage = (value: string | null): number => {
  if (!value) {
    return QUERY_DEFAULTS.page;
  }

  const parsed = Number.parseInt(value, 10);

  if (Number.isNaN(parsed) || parsed < 1) {
    return QUERY_DEFAULTS.page;
  }

  return parsed;
};

const cleanValue = (value: string | null, fallback: string): string => {
  if (!value) {
    return fallback;
  }

  const trimmed = value.trim();
  return trimmed.length === 0 ? fallback : trimmed;
};

const copyParams = (source: SearchParamsLike): URLSearchParams =>
  new URLSearchParams(source.toString());

export function readQueryParams(source: SearchParamsLike): QueryParamsState {
  return {
    search: cleanValue(source.get("search"), QUERY_DEFAULTS.search),
    page: numericPage(source.get("page")),
    sort: cleanValue(source.get("sort"), QUERY_DEFAULTS.sort),
    filter: cleanValue(source.get("filter"), QUERY_DEFAULTS.filter),
  };
}

export function updateQueryParams(
  source: SearchParamsLike,
  updates: Partial<
    Record<keyof QueryParamsState, string | number | null | undefined>
  >
): URLSearchParams {
  const next = copyParams(source);

  for (const [key, rawValue] of Object.entries(updates)) {
    if (rawValue === null || rawValue === undefined || rawValue === "") {
      next.delete(key);
      continue;
    }

    next.set(key, String(rawValue));
  }

  return next;
}

export function removeQueryParams(
  source: SearchParamsLike,
  keys: (keyof QueryParamsState)[]
): URLSearchParams {
  const next = copyParams(source);

  for (const key of keys) {
    next.delete(key);
  }

  return next;
}

export function buildQueryString(
  source: SearchParamsLike,
  updates?: Partial<
    Record<keyof QueryParamsState, string | number | null | undefined>
  >
): string {
  const next = updates
    ? updateQueryParams(source, updates)
    : copyParams(source);
  return next.toString();
}
