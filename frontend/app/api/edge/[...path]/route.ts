import type { NextRequest } from "next/server";

export const dynamic = "force-dynamic";
export const runtime = "nodejs";

const DEFAULT_EDGE_API_INTERNAL_URL = "http://edge-api:8080";
const DEFAULT_EDGE_API_LOCAL_URL = "http://localhost:8080";
const HOP_BY_HOP_HEADERS = new Set([
  "connection",
  "content-length",
  "host",
  "keep-alive",
  "proxy-authenticate",
  "proxy-authorization",
  "te",
  "trailer",
  "transfer-encoding",
  "upgrade",
]);

type UpstreamAttempt = {
  baseUrl: string;
  targetUrl: string;
  error: unknown;
};

function normalizeBaseUrl(rawUrl: string): string {
  return rawUrl.endsWith("/") ? rawUrl.slice(0, -1) : rawUrl;
}

function isDevelopmentServer(): boolean {
  return process.env.NODE_ENV === "development";
}

function shouldUseLocalDevFallback(configuredUrl?: string): boolean {
  return (
    isDevelopmentServer() &&
    (!configuredUrl ||
      normalizeBaseUrl(configuredUrl) === DEFAULT_EDGE_API_INTERNAL_URL)
  );
}

function getEdgeApiBaseUrls(): string[] {
  const configuredUrl = process.env.EDGE_API_INTERNAL_URL?.trim();

  if (!configuredUrl) {
    return isDevelopmentServer()
      ? [DEFAULT_EDGE_API_LOCAL_URL, DEFAULT_EDGE_API_INTERNAL_URL]
      : [DEFAULT_EDGE_API_INTERNAL_URL];
  }

  const candidates = [normalizeBaseUrl(configuredUrl)];
  if (shouldUseLocalDevFallback(configuredUrl)) {
    candidates.push(DEFAULT_EDGE_API_LOCAL_URL);
  }

  return [...new Set(candidates)];
}

function buildTargetUrl(
  baseUrl: string,
  request: NextRequest,
  path: string[]
): string {
  const normalizedPath = path.join("/");
  const targetUrl = new URL(`${baseUrl}/${normalizedPath}`);
  request.nextUrl.searchParams.forEach((value, key) => {
    targetUrl.searchParams.append(key, value);
  });
  return targetUrl.toString();
}

function buildForwardHeaders(request: NextRequest): Headers {
  const headers = new Headers();

  request.headers.forEach((value, key) => {
    if (HOP_BY_HOP_HEADERS.has(key.toLowerCase())) {
      return;
    }
    headers.set(key, value);
  });

  headers.set("x-forwarded-host", request.headers.get("host") ?? "");
  headers.set(
    "x-forwarded-proto",
    request.nextUrl.protocol.replace(":", "") || "http"
  );
  headers.set(
    "x-forwarded-for",
    request.headers.get("x-forwarded-for") ?? "127.0.0.1"
  );

  return headers;
}

function copyResponseHeaders(upstreamHeaders: Headers): Headers {
  const responseHeaders = new Headers();

  upstreamHeaders.forEach((value, key) => {
    if (
      HOP_BY_HOP_HEADERS.has(key.toLowerCase()) ||
      key.toLowerCase() === "set-cookie"
    ) {
      return;
    }
    responseHeaders.append(key, value);
  });

  const getSetCookie = (
    upstreamHeaders as Headers & { getSetCookie?: () => string[] }
  ).getSetCookie;
  if (typeof getSetCookie === "function") {
    for (const cookie of getSetCookie.call(upstreamHeaders)) {
      responseHeaders.append("set-cookie", cookie);
    }
  } else {
    const fallbackCookie = upstreamHeaders.get("set-cookie");
    if (fallbackCookie) {
      responseHeaders.append("set-cookie", fallbackCookie);
    }
  }

  return responseHeaders;
}

function serializeError(error: unknown): Record<string, unknown> {
  if (error instanceof Error) {
    return {
      name: error.name,
      message: error.message,
      cause:
        error.cause instanceof Error
          ? {
              name: error.cause.name,
              message: error.cause.message,
            }
          : error.cause,
    };
  }

  return { value: error };
}

function logUpstreamFailure(
  request: NextRequest,
  attempts: UpstreamAttempt[]
): void {
  console.error(
    `[edge-proxy] upstream request failed ${JSON.stringify({
      method: request.method,
      path: request.nextUrl.pathname,
      search: request.nextUrl.search,
      attempts: attempts.map((attempt) => ({
        baseUrl: attempt.baseUrl,
        targetUrl: attempt.targetUrl,
        error: serializeError(attempt.error),
      })),
    })}`
  );
}

function buildProxyErrorResponse(): Response {
  return Response.json(
    {
      code: "proxy_upstream_unavailable",
      message:
        "Edge API is unavailable. Check the frontend proxy configuration and backend availability.",
    },
    {
      status: 502,
      headers: {
        "cache-control": "no-store",
      },
    }
  );
}

async function proxyRequest(
  request: NextRequest,
  context: { params: Promise<{ path: string[] }> }
): Promise<Response> {
  const { path } = await context.params;
  const shouldForwardBody = request.method !== "GET" && request.method !== "HEAD";
  const requestBody = shouldForwardBody ? await request.arrayBuffer() : undefined;
  const attempts: UpstreamAttempt[] = [];

  for (const baseUrl of getEdgeApiBaseUrls()) {
    const targetUrl = buildTargetUrl(baseUrl, request, path);

    try {
      const upstreamResponse = await fetch(targetUrl, {
        method: request.method,
        headers: buildForwardHeaders(request),
        body: requestBody,
        redirect: "manual",
        cache: "no-store",
      });

      if (attempts.length > 0) {
        console.warn("[edge-proxy] upstream fallback succeeded", {
          method: request.method,
          path: request.nextUrl.pathname,
          upstream: baseUrl,
        });
      }

      return new Response(upstreamResponse.body, {
        status: upstreamResponse.status,
        statusText: upstreamResponse.statusText,
        headers: copyResponseHeaders(upstreamResponse.headers),
      });
    } catch (error) {
      attempts.push({ baseUrl, targetUrl, error });
    }
  }

  logUpstreamFailure(request, attempts);
  return buildProxyErrorResponse();
}

export const GET = proxyRequest;
export const POST = proxyRequest;
export const PUT = proxyRequest;
export const PATCH = proxyRequest;
export const DELETE = proxyRequest;
export const OPTIONS = proxyRequest;
export const HEAD = proxyRequest;
