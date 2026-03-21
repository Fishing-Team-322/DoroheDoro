export interface ApiError extends Error {
  name: "ApiError";
  status: number | null;
  code: string;
  details?: unknown;
  response?: Response;
  cause?: unknown;
}

export interface RequestOptions extends Omit<RequestInit, "body"> {
  body?: unknown;
}

export interface ApiClientConfig {
  baseUrl?: string;
  defaultHeaders?: HeadersInit;
  fetcher?: typeof fetch;
  credentials?: RequestCredentials;
  getCsrfToken?: () => string | null | undefined;
  csrfHeaderName?: string;
  onUnauthorized?: (error: ApiError) => void | Promise<void>;
}

const JSON_CONTENT_TYPE = "application/json";
const DEFAULT_CSRF_HEADER_NAME = "X-CSRF-Token";
const MUTATING_METHODS = new Set(["POST", "PUT", "PATCH", "DELETE"]);

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

const buildUrl = (baseUrl: string | undefined, path: string): string => {
  if (/^https?:\/\//i.test(path)) {
    return path;
  }

  if (!baseUrl) {
    return path;
  }

  const normalizedBase = baseUrl.endsWith("/") ? baseUrl.slice(0, -1) : baseUrl;
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  return `${normalizedBase}${normalizedPath}`;
};

const createApiError = (params: {
  message: string;
  status: number | null;
  code: string;
  details?: unknown;
  response?: Response;
  cause?: unknown;
}): ApiError => {
  const error = new Error(params.message) as ApiError;
  error.name = "ApiError";
  error.status = params.status;
  error.code = params.code;
  error.details = params.details;
  error.response = params.response;
  error.cause = params.cause;
  return error;
};

const normalizeError = async (
  error: unknown,
  response?: Response
): Promise<ApiError> => {
  if (response) {
    let details: unknown;
    let message = `Request failed with status ${response.status}`;
    let code = "API_RESPONSE_ERROR";

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

    if (isRecord(details) && typeof details.message === "string") {
      message = details.message;
    }

    if (isRecord(details) && typeof details.code === "string" && details.code) {
      code = details.code;
    }

    return createApiError({
      message,
      status: response.status,
      code,
      details,
      response,
      cause: error,
    });
  }

  if (isApiError(error)) {
    return error as ApiError;
  }

  if (error instanceof TypeError) {
    return createApiError({
      message: error.message || "Network error",
      status: null,
      code: "NETWORK_ERROR",
      cause: error,
    });
  }

  return createApiError({
    message: "Unexpected error",
    status: null,
    code: "UNKNOWN_ERROR",
    cause: error,
  });
};

export class ApiClient {
  private readonly baseUrl?: string;
  private readonly defaultHeaders?: HeadersInit;
  private readonly fetcher: typeof fetch;
  private readonly credentials?: RequestCredentials;
  private readonly getCsrfToken?: () => string | null | undefined;
  private readonly csrfHeaderName: string;
  private readonly onUnauthorized?: (error: ApiError) => void | Promise<void>;

  constructor(config: ApiClientConfig = {}) {
    this.baseUrl = config.baseUrl;
    this.defaultHeaders = config.defaultHeaders;
    this.fetcher = resolveFetch(config.fetcher);
    this.credentials = config.credentials;
    this.getCsrfToken = config.getCsrfToken;
    this.csrfHeaderName = config.csrfHeaderName ?? DEFAULT_CSRF_HEADER_NAME;
    this.onUnauthorized = config.onUnauthorized;
  }

  async request<T>(path: string, options: RequestOptions = {}): Promise<T> {
    const { body, headers, ...init } = options;
    const url = buildUrl(this.baseUrl, path);
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
      });

      if (!response.ok) {
        throw await normalizeError(undefined, response);
      }

      if (response.status === 204) {
        return undefined as T;
      }

      const contentType = response.headers.get("content-type") ?? "";
      if (contentType.includes(JSON_CONTENT_TYPE)) {
        return (await response.json()) as T;
      }

      return (await response.text()) as T;
    } catch (error) {
      const normalizedError = await normalizeError(error);
      if (normalizedError.status === 401) {
        await this.onUnauthorized?.(normalizedError);
      }
      throw normalizedError;
    }
  }

  get<T>(
    path: string,
    options?: Omit<RequestOptions, "method" | "body">
  ): Promise<T> {
    return this.request<T>(path, { ...options, method: "GET" });
  }

  post<T>(
    path: string,
    body?: unknown,
    options?: Omit<RequestOptions, "method" | "body">
  ): Promise<T> {
    return this.request<T>(path, { ...options, method: "POST", body });
  }

  put<T>(
    path: string,
    body?: unknown,
    options?: Omit<RequestOptions, "method" | "body">
  ): Promise<T> {
    return this.request<T>(path, { ...options, method: "PUT", body });
  }

  patch<T>(
    path: string,
    body?: unknown,
    options?: Omit<RequestOptions, "method" | "body">
  ): Promise<T> {
    return this.request<T>(path, { ...options, method: "PATCH", body });
  }

  delete<T>(
    path: string,
    options?: Omit<RequestOptions, "method" | "body">
  ): Promise<T> {
    return this.request<T>(path, { ...options, method: "DELETE" });
  }
}

export const createApiClient = (config?: ApiClientConfig): ApiClient => {
  return new ApiClient(config);
};
