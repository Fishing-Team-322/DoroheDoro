export interface ApiError extends Error {
  name: "ApiError";
  status: number | null;
  code: string;
  details?: unknown;
  response?: Response;
  cause?: unknown;
  requestId?: string;
  natsSubject?: string;
  url?: string;
}

export type QueryValue =
  | string
  | number
  | boolean
  | null
  | undefined
  | Array<string | number | boolean | null | undefined>;

export type QueryParameters = Record<string, QueryValue>;

export interface RequestOptions extends Omit<RequestInit, "body"> {
  body?: unknown;
  query?: QueryParameters;
  timeoutMs?: number;
}

export interface ApiResponseMeta {
  status: number;
  statusText: string;
  requestId?: string;
  natsSubject?: string;
  headers: Headers;
  url: string;
}

export interface ApiResult<T> {
  data: T;
  meta: ApiResponseMeta;
}

export interface ApiClientConfig {
  baseUrl?: string;
  defaultHeaders?: HeadersInit;
  fetcher?: typeof fetch;
  credentials?: RequestCredentials;
  timeoutMs?: number;
  getCsrfToken?: () => string | null | undefined;
  csrfHeaderName?: string;
  onUnauthorized?: (error: ApiError) => void | Promise<void>;
}

const JSON_CONTENT_TYPE = "application/json";
const DEFAULT_CSRF_HEADER_NAME = "X-CSRF-Token";
const MUTATING_METHODS = new Set(["POST", "PUT", "PATCH", "DELETE"]);
const DEFAULT_TIMEOUT_MS = 15_000;

const isRecord = (value: unknown): value is Record<string, unknown> => {
  return typeof value === "object" && value !== null;
};

const resolveFetch = (fetcher?: typeof fetch): typeof fetch => {
  if (fetcher) {
    if (typeof window !== "undefined" && fetcher === window.fetch) {
      return window.fetch.bind(window);
    }

    if (typeof globalThis.fetch === "function" && fetcher === globalThis.fetch) {
      return globalThis.fetch.bind(globalThis);
    }

    return ((input: RequestInfo | URL, init?: RequestInit) =>
      fetcher(input, init)) as typeof fetch;
  }

  if (typeof window !== "undefined" && typeof window.fetch === "function") {
    return window.fetch.bind(window);
  }

  if (typeof globalThis.fetch === "function") {
    return globalThis.fetch.bind(globalThis);
  }

  throw new Error("Fetch API is unavailable in this runtime");
};

export const isApiError = (error: unknown): error is ApiError => {
  return error instanceof Error && (error as ApiError).name === "ApiError";
};

const isAbsoluteUrl = (value: string): boolean => /^https?:\/\//i.test(value);

const appendQueryParams = (
  url: URL,
  query: QueryParameters | undefined
): void => {
  if (!query) {
    return;
  }

  for (const [key, rawValue] of Object.entries(query)) {
    if (rawValue == null || rawValue === "") {
      continue;
    }

    const values = Array.isArray(rawValue) ? rawValue : [rawValue];

    for (const value of values) {
      if (value == null || value === "") {
        continue;
      }

      url.searchParams.append(key, String(value));
    }
  }
};

const buildUrl = (
  baseUrl: string | undefined,
  path: string,
  query?: QueryParameters
): string => {
  const rawUrl = isAbsoluteUrl(path)
    ? path
    : baseUrl
      ? `${baseUrl.endsWith("/") ? baseUrl.slice(0, -1) : baseUrl}${path.startsWith("/") ? path : `/${path}`}`
      : path;

  if (!query) {
    return rawUrl;
  }

  const absolute = isAbsoluteUrl(rawUrl);
  const url = new URL(rawUrl, "http://api.local");
  appendQueryParams(url, query);

  if (absolute) {
    return url.toString();
  }

  return `${url.pathname}${url.search}`;
};

const createApiError = (params: {
  message: string;
  status: number | null;
  code: string;
  details?: unknown;
  response?: Response;
  cause?: unknown;
  requestId?: string;
  natsSubject?: string;
  url?: string;
}): ApiError => {
  const error = new Error(params.message) as ApiError;
  error.name = "ApiError";
  error.status = params.status;
  error.code = params.code;
  error.details = params.details;
  error.response = params.response;
  error.cause = params.cause;
  error.requestId = params.requestId;
  error.natsSubject = params.natsSubject;
  error.url = params.url;
  return error;
};

const getHeaderValue = (headers: Headers, name: string): string | undefined => {
  return headers.get(name) ?? headers.get(name.toLowerCase()) ?? undefined;
};

const resolveErrorPayload = (details: unknown): Record<string, unknown> | null => {
  if (isRecord(details) && isRecord(details.error)) {
    return details.error;
  }

  if (isRecord(details)) {
    return details;
  }

  return null;
};

const normalizeError = async (
  error: unknown,
  response?: Response,
  context?: {
    didTimeout?: boolean;
    url?: string;
  }
): Promise<ApiError> => {
  if (response) {
    let details: unknown;
    let message = `Request failed with status ${response.status}`;
    let code = "API_RESPONSE_ERROR";
    const requestId =
      getHeaderValue(response.headers, "x-request-id") ??
      getHeaderValue(response.headers, "X-Request-ID");
    const natsSubject =
      getHeaderValue(response.headers, "x-nats-subject") ??
      getHeaderValue(response.headers, "X-NATS-Subject");

    try {
      const contentType = response.headers.get("content-type") ?? "";
      if (contentType.includes(JSON_CONTENT_TYPE)) {
        details = await response.json();
      } else {
        details = await response.text();
      }
    } catch {
      details = undefined;
    }

    const payload = resolveErrorPayload(details);

    if (payload && typeof payload.message === "string") {
      message = payload.message;
    }

    if (payload && typeof payload.code === "string" && payload.code) {
      code = payload.code;
    }

    return createApiError({
      message,
      status: response.status,
      code,
      details,
      response,
      cause: error,
      requestId:
        (payload && typeof payload.request_id === "string"
          ? payload.request_id
          : undefined) ?? requestId,
      natsSubject,
      url: context?.url ?? response.url,
    });
  }

  if (isApiError(error)) {
    return error as ApiError;
  }

  if (context?.didTimeout) {
    return createApiError({
      message: "Request timed out",
      status: null,
      code: "TIMEOUT_ERROR",
      cause: error,
      url: context.url,
    });
  }

  if (
    error instanceof DOMException &&
    (error.name === "AbortError" || error.name === "TimeoutError")
  ) {
    return createApiError({
      message:
        error.name === "TimeoutError" ? "Request timed out" : "Request aborted",
      status: null,
      code:
        error.name === "TimeoutError" ? "TIMEOUT_ERROR" : "REQUEST_ABORTED",
      cause: error,
      url: context?.url,
    });
  }

  if (error instanceof TypeError) {
    return createApiError({
      message: error.message || "Network error",
      status: null,
      code: "NETWORK_ERROR",
      cause: error,
      url: context?.url,
    });
  }

  return createApiError({
    message: "Unexpected error",
    status: null,
    code: "UNKNOWN_ERROR",
    cause: error,
    url: context?.url,
  });
};

const createResponseMeta = (
  response: Response,
  url: string
): ApiResponseMeta => {
  return {
    status: response.status,
    statusText: response.statusText,
    requestId:
      getHeaderValue(response.headers, "x-request-id") ??
      getHeaderValue(response.headers, "X-Request-ID"),
    natsSubject:
      getHeaderValue(response.headers, "x-nats-subject") ??
      getHeaderValue(response.headers, "X-NATS-Subject"),
    headers: response.headers,
    url: response.url || url,
  };
};

const createRequestSignal = (
  inputSignal: AbortSignal | null | undefined,
  timeoutMs: number | undefined
): {
  signal?: AbortSignal;
  didTimeout: () => boolean;
  cleanup: () => void;
} => {
  const timeout = timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const hasTimeout = timeout > 0;
  const controller = new AbortController();
  let didTimeout = false;
  let timeoutId: ReturnType<typeof setTimeout> | null = null;

  const abortWithReason = (reason?: unknown) => {
    if (!controller.signal.aborted) {
      controller.abort(reason);
    }
  };

  const onInputAbort = () => {
    abortWithReason(inputSignal?.reason);
  };

  if (inputSignal) {
    if (inputSignal.aborted) {
      abortWithReason(inputSignal.reason);
    } else {
      inputSignal.addEventListener("abort", onInputAbort, { once: true });
    }
  }

  if (hasTimeout) {
    timeoutId = setTimeout(() => {
      didTimeout = true;
      abortWithReason(
        new DOMException("Request timed out", "TimeoutError")
      );
    }, timeout);
  }

  return {
    signal: controller.signal,
    didTimeout: () => didTimeout,
    cleanup: () => {
      if (timeoutId) {
        clearTimeout(timeoutId);
      }

      inputSignal?.removeEventListener("abort", onInputAbort);
    },
  };
};

export class ApiClient {
  private readonly baseUrl?: string;
  private readonly defaultHeaders?: HeadersInit;
  private readonly fetcher: typeof fetch;
  private readonly credentials?: RequestCredentials;
  private readonly timeoutMs?: number;
  private readonly getCsrfToken?: () => string | null | undefined;
  private readonly csrfHeaderName: string;
  private readonly onUnauthorized?: (error: ApiError) => void | Promise<void>;

  constructor(config: ApiClientConfig = {}) {
    this.baseUrl = config.baseUrl;
    this.defaultHeaders = config.defaultHeaders;
    this.fetcher = resolveFetch(config.fetcher);
    this.credentials = config.credentials;
    this.timeoutMs = config.timeoutMs;
    this.getCsrfToken = config.getCsrfToken;
    this.csrfHeaderName = config.csrfHeaderName ?? DEFAULT_CSRF_HEADER_NAME;
    this.onUnauthorized = config.onUnauthorized;
  }

  private async parseResponse<T>(response: Response): Promise<T> {
    if (response.status === 204) {
      return undefined as T;
    }

    const contentType = response.headers.get("content-type") ?? "";
    if (contentType.includes(JSON_CONTENT_TYPE)) {
      return (await response.json()) as T;
    }

    return (await response.text()) as T;
  }

  async requestWithMeta<T>(
    path: string,
    options: RequestOptions = {}
  ): Promise<ApiResult<T>> {
    const { body, headers, query, timeoutMs, ...init } = options;
    const url = buildUrl(this.baseUrl, path, query);
    const method = (init.method ?? "GET").toUpperCase();

    const requestHeaders = new Headers(this.defaultHeaders);
    if (headers) {
      new Headers(headers).forEach((value, key) => {
        requestHeaders.set(key, value);
      });
    }

    let serializedBody: BodyInit | undefined;
    if (body !== undefined && body !== null) {
      if (
        typeof body === "string" ||
        body instanceof FormData ||
        body instanceof URLSearchParams ||
        body instanceof Blob ||
        body instanceof ArrayBuffer
      ) {
        serializedBody = body;
      } else {
        if (!requestHeaders.has("Content-Type")) {
          requestHeaders.set("Content-Type", JSON_CONTENT_TYPE);
        }
        serializedBody = JSON.stringify(body);
      }
    }

    const signalState = createRequestSignal(init.signal, timeoutMs ?? this.timeoutMs);

    try {
      if (
        MUTATING_METHODS.has(method) &&
        !requestHeaders.has(this.csrfHeaderName)
      ) {
        const csrfToken = this.getCsrfToken?.();
        if (csrfToken) {
          requestHeaders.set(this.csrfHeaderName, csrfToken);
        }
      }

      const response = await this.fetcher(url, {
        ...init,
        method,
        credentials: init.credentials ?? this.credentials,
        headers: requestHeaders,
        body: serializedBody,
        signal: signalState.signal,
      });

      if (!response.ok) {
        throw await normalizeError(undefined, response, { url });
      }

      return {
        data: await this.parseResponse<T>(response),
        meta: createResponseMeta(response, url),
      };
    } catch (error) {
      const normalizedError = await normalizeError(error, undefined, {
        didTimeout: signalState.didTimeout(),
        url,
      });
      if (normalizedError.status === 401) {
        await this.onUnauthorized?.(normalizedError);
      }
      throw normalizedError;
    } finally {
      signalState.cleanup();
    }
  }

  async request<T>(path: string, options: RequestOptions = {}): Promise<T> {
    const response = await this.requestWithMeta<T>(path, options);
    return response.data;
  }

  get<T>(
    path: string,
    options?: Omit<RequestOptions, "method" | "body">
  ): Promise<T> {
    return this.request<T>(path, { ...options, method: "GET" });
  }

  getWithMeta<T>(
    path: string,
    options?: Omit<RequestOptions, "method" | "body">
  ): Promise<ApiResult<T>> {
    return this.requestWithMeta<T>(path, { ...options, method: "GET" });
  }

  post<T>(
    path: string,
    body?: unknown,
    options?: Omit<RequestOptions, "method" | "body">
  ): Promise<T> {
    return this.request<T>(path, { ...options, method: "POST", body });
  }

  postWithMeta<T>(
    path: string,
    body?: unknown,
    options?: Omit<RequestOptions, "method" | "body">
  ): Promise<ApiResult<T>> {
    return this.requestWithMeta<T>(path, { ...options, method: "POST", body });
  }

  put<T>(
    path: string,
    body?: unknown,
    options?: Omit<RequestOptions, "method" | "body">
  ): Promise<T> {
    return this.request<T>(path, { ...options, method: "PUT", body });
  }

  putWithMeta<T>(
    path: string,
    body?: unknown,
    options?: Omit<RequestOptions, "method" | "body">
  ): Promise<ApiResult<T>> {
    return this.requestWithMeta<T>(path, { ...options, method: "PUT", body });
  }

  patch<T>(
    path: string,
    body?: unknown,
    options?: Omit<RequestOptions, "method" | "body">
  ): Promise<T> {
    return this.request<T>(path, { ...options, method: "PATCH", body });
  }

  patchWithMeta<T>(
    path: string,
    body?: unknown,
    options?: Omit<RequestOptions, "method" | "body">
  ): Promise<ApiResult<T>> {
    return this.requestWithMeta<T>(path, { ...options, method: "PATCH", body });
  }

  delete<T>(
    path: string,
    options?: Omit<RequestOptions, "method" | "body">
  ): Promise<T> {
    return this.request<T>(path, { ...options, method: "DELETE" });
  }

  deleteWithMeta<T>(
    path: string,
    options?: Omit<RequestOptions, "method" | "body">
  ): Promise<ApiResult<T>> {
    return this.requestWithMeta<T>(path, { ...options, method: "DELETE" });
  }
}

export const createApiClient = (config?: ApiClientConfig): ApiClient => {
  return new ApiClient(config);
};
