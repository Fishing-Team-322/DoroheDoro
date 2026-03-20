"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import type { Locale } from "@/src/shared/config";
import { buildLoginPath } from "@/src/shared/lib/auth";
import { Button, type ButtonProps } from "@/src/shared/ui";
import { useAuth } from "../model/use-auth";

type LogoutButtonProps = Omit<ButtonProps, "onClick" | "loading"> & {
  locale: Locale;
};

export function LogoutButton({
  locale,
  children = "Sign out",
  ...props
}: LogoutButtonProps) {
  const router = useRouter();
  const { logout } = useAuth();
  const [isPending, setIsPending] = useState(false);

  const handleLogout = async () => {
    setIsPending(true);

    try {
      await logout();
      router.replace(buildLoginPath(locale));
      router.refresh();
    } finally {
      setIsPending(false);
    }
  };

  return (
    <Button {...props} onClick={handleLogout} loading={isPending}>
      {children}
    </Button>
  );
}
