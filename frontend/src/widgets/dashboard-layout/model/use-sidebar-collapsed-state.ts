"use client";

import { useEffect, useState } from "react";

const SIDEBAR_COLLAPSED_STORAGE_KEY = "dashboard-sidebar-collapsed";

function readStoredSidebarCollapsedState() {
  if (typeof window === "undefined") {
    return false;
  }

  try {
    return window.localStorage.getItem(SIDEBAR_COLLAPSED_STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

export function useSidebarCollapsedState() {
  const [collapsed, setCollapsed] = useState<boolean>(() =>
    readStoredSidebarCollapsedState()
  );

  useEffect(() => {
    try {
      window.localStorage.setItem(
        SIDEBAR_COLLAPSED_STORAGE_KEY,
        String(collapsed)
      );
    } catch {
      // Ignore storage errors so the sidebar still works in restricted browsers.
    }
  }, [collapsed]);

  useEffect(() => {
    const handleStorage = (event: StorageEvent) => {
      if (event.key !== SIDEBAR_COLLAPSED_STORAGE_KEY) {
        return;
      }

      setCollapsed(event.newValue === "true");
    };

    window.addEventListener("storage", handleStorage);

    return () => {
      window.removeEventListener("storage", handleStorage);
    };
  }, []);

  return [collapsed, setCollapsed] as const;
}
