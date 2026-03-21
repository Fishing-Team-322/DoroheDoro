"use client";

import {
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import {
  buildLiveLogsStreamUrl,
  parseLiveLogEvent,
  type LiveLogsFilters,
  type LogEntry,
} from "../api";

export type LiveLogsConnectionState =
  | "paused"
  | "connecting"
  | "connected"
  | "disconnected"
  | "reconnecting";

type UseLiveLogsOptions = {
  enabled: boolean;
  filters: LiveLogsFilters;
  maxItems?: number;
};

export function useLiveLogs({
  enabled,
  filters,
  maxItems = 200,
}: UseLiveLogsOptions) {
  const [items, setItems] = useState<LogEntry[]>([]);
  const [connectionState, setConnectionState] =
    useState<LiveLogsConnectionState>(enabled ? "connecting" : "paused");
  const [lastEventAt, setLastEventAt] = useState<number>();
  const [lastError, setLastError] = useState<string>();
  const [reconnectDelayMs, setReconnectDelayMs] = useState<number>();

  const sourceRef = useRef<EventSource | null>(null);
  const reconnectTimeoutRef = useRef<number | null>(null);
  const reconnectAttemptRef = useRef(0);
  const connectRef = useRef<() => void>(() => {});

  const cleanupConnection = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      window.clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    sourceRef.current?.close();
    sourceRef.current = null;
  }, []);

  const scheduleReconnect = useCallback(() => {
    if (!enabled) {
      return;
    }

    reconnectAttemptRef.current += 1;
    const delay = Math.min(30_000, 1_000 * 2 ** (reconnectAttemptRef.current - 1));
    setReconnectDelayMs(delay);
    setConnectionState("disconnected");

    reconnectTimeoutRef.current = window.setTimeout(() => {
      setConnectionState("reconnecting");
      connectRef.current();
    }, delay);
  }, [enabled]);

  const connect = useCallback(() => {
    cleanupConnection();

    if (!enabled) {
      return;
    }

    setConnectionState(
      reconnectAttemptRef.current > 0 ? "reconnecting" : "connecting"
    );

    const source = new EventSource(buildLiveLogsStreamUrl(filters), {
      withCredentials: true,
    });

    sourceRef.current = source;

    const handleMessage = (event: MessageEvent<string>) => {
      try {
        const parsed = JSON.parse(event.data);
        const entry = parseLiveLogEvent(parsed);

        if (!entry) {
          return;
        }

        setItems((current) => [...current, entry].slice(-maxItems));
        setLastEventAt(Date.now());
      } catch {
        setLastError("Received an invalid live log event.");
      }
    };

    source.onopen = () => {
      reconnectAttemptRef.current = 0;
      setReconnectDelayMs(undefined);
      setLastError(undefined);
      setConnectionState("connected");
    };

    source.onmessage = handleMessage;
    source.addEventListener("log", handleMessage as EventListener);

    source.onerror = () => {
      source.close();
      sourceRef.current = null;
      setLastError("Live log stream disconnected.");
      scheduleReconnect();
    };
  }, [
    cleanupConnection,
    enabled,
    filters,
    maxItems,
    scheduleReconnect,
  ]);

  useEffect(() => {
    connectRef.current = connect;
  }, [connect]);

  useEffect(() => {
    if (!enabled) {
      cleanupConnection();
      return;
    }

    const timeout = window.setTimeout(() => {
      connect();
    }, 0);

    return () => {
      window.clearTimeout(timeout);
      cleanupConnection();
    };
  }, [cleanupConnection, connect, enabled]);

  return {
    items,
    connectionState: enabled ? connectionState : "paused",
    lastEventAt,
    lastError,
    reconnectDelayMs: enabled ? reconnectDelayMs : undefined,
    clear: () => setItems([]),
  };
}
