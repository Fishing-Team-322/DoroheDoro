"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import type { Locale } from "@/src/shared/config";
import { buildLoginPath } from "@/src/shared/lib/auth";
import { useI18n } from "@/src/shared/lib/i18n";
import { Button, type ButtonProps } from "@/src/shared/ui";
import { useAuth } from "../model/use-auth";

type LogoutButtonProps = Omit<ButtonProps, "onClick" | "loading"> & {
  locale: Locale;
};

export function LogoutButton({
  locale,
  children,
  ...props
}: LogoutButtonProps) {
  const router = useRouter();
  const { dictionary } = useI18n();
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
      {children ?? dictionary.auth.logout.full}
    </Button>
  );
}
