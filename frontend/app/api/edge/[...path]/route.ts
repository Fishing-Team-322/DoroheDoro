import type { NextRequest } from "next/server";

export const dynamic = "force-dynamic";
export const runtime = "nodejs";

const DEFAULT_EDGE_API_INTERNAL_URL = "http://edge-api:8080";
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

function getEdgeApiBaseUrl(): string {
  const rawUrl =
    process.env.EDGE_API_INTERNAL_URL ?? DEFAULT_EDGE_API_INTERNAL_URL;

  return rawUrl.endsWith("/") ? rawUrl.slice(0, -1) : rawUrl;
}

function buildTargetUrl(request: NextRequest, path: string[]): string {
  const normalizedPath = path.join("/");
  const targetUrl = new URL(`${getEdgeApiBaseUrl()}/${normalizedPath}`);
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

async function proxyRequest(
  request: NextRequest,
  context: { params: Promise<{ path: string[] }> }
): Promise<Response> {
  const { path } = await context.params;
  const targetUrl = buildTargetUrl(request, path);
  const shouldForwardBody = request.method !== "GET" && request.method !== "HEAD";

  const upstreamResponse = await fetch(targetUrl, {
    method: request.method,
    headers: buildForwardHeaders(request),
    body: shouldForwardBody ? await request.arrayBuffer() : undefined,
    redirect: "manual",
    cache: "no-store",
  });

  return new Response(upstreamResponse.body, {
    status: upstreamResponse.status,
    statusText: upstreamResponse.statusText,
    headers: copyResponseHeaders(upstreamResponse.headers),
  });
}

export const GET = proxyRequest;
export const POST = proxyRequest;
export const PUT = proxyRequest;
export const PATCH = proxyRequest;
export const DELETE = proxyRequest;
export const OPTIONS = proxyRequest;
export const HEAD = proxyRequest;
