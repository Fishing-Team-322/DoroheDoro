"use client";

import Link from "next/link";
import { useEffect } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import type { Locale } from "@/src/shared/config";
import {
  getDefaultAuthenticatedPath,
  normalizeRedirectPath,
} from "@/src/shared/lib/auth";
import { withLocalePath } from "@/src/shared/lib/i18n";
import { Card, Spinner } from "@/src/shared/ui";
import { LoginForm, useAuth } from "@/src/features/auth";

type LoginPageProps = {
  locale: Locale;
};

export function LoginPage({ locale }: LoginPageProps) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const { status } = useAuth();

  const redirectTo =
    normalizeRedirectPath(searchParams.get("next"), locale) ??
    getDefaultAuthenticatedPath(locale);

  useEffect(() => {
    if (status === "authenticated") {
      router.replace(redirectTo);
    }
  }, [redirectTo, router, status]);

  return (
    <main className="min-h-screen bg-[radial-gradient(circle_at_top,_rgba(59,130,246,0.16),_transparent_42%),linear-gradient(180deg,_#f7f8fb_0%,_#eef2ff_100%)] px-6 py-12 text-zinc-900">
      <div className="mx-auto flex min-h-[calc(100vh-6rem)] w-full max-w-5xl items-center justify-center">
        <Card className="grid w-full max-w-4xl overflow-hidden border-white/70 bg-white/90 p-0 shadow-2xl shadow-slate-900/10 backdrop-blur">
          <div className="grid lg:grid-cols-[1.1fr_0.9fr]">
            <section className="space-y-6 p-8 sm:p-10">
              <div className="space-y-3">
                <p className="text-xs font-semibold uppercase tracking-[0.24em] text-sky-700">
                  Session auth
                </p>
                <h1 className="text-3xl font-semibold tracking-tight text-slate-950 sm:text-4xl">
                  Sign in with your account
                </h1>
                <p className="max-w-lg text-sm leading-6 text-slate-600">
                  The frontend uses cookie-based sessions and automatically sends
                  the CSRF token for every state-changing request.
                </p>
              </div>

              {status === "loading" ? (
                <div className="inline-flex items-center gap-3 rounded-full border border-slate-200 bg-slate-50 px-4 py-2 text-sm text-slate-600">
                  <Spinner size="sm" />
                  Checking the current session...
                </div>
              ) : (
                <LoginForm />
              )}

              <div className="flex flex-wrap gap-3 text-sm text-slate-500">
                <Link
                  href={withLocalePath(locale, "/")}
                  className="transition-colors hover:text-slate-950"
                >
                  Back to home
                </Link>
                <Link
                  href={getDefaultAuthenticatedPath(locale)}
                  className="transition-colors hover:text-slate-950"
                >
                  Open dashboard
                </Link>
              </div>
            </section>

            <aside className="flex flex-col justify-between gap-6 border-t border-slate-200 bg-slate-950 px-8 py-10 text-slate-100 lg:border-l lg:border-t-0">
              <div className="space-y-4">
                <p className="text-xs font-semibold uppercase tracking-[0.24em] text-sky-300">
                  Frontend responsibilities
                </p>
                <ul className="space-y-3 text-sm leading-6 text-slate-300">
                  <li>
                    <code>credentials: &quot;include&quot;</code> is enabled for every
                    request.
                  </li>
                  <li>
                    <code>X-CSRF-Token</code> is injected automatically for{" "}
                    <code>POST</code>, <code>PUT</code>, <code>PATCH</code>, and{" "}
                    <code>DELETE</code>.
                  </li>
                  <li>
                    <code>401</code> responses reset auth state and protected routes
                    redirect back to <code>/login</code>.
                  </li>
                </ul>
              </div>

              <div className="rounded-3xl border border-white/10 bg-white/5 p-5">
                <p className="text-sm font-medium text-white">Expected API</p>
                <p className="mt-2 text-sm leading-6 text-slate-300">
                  <code>POST /auth/login</code>, <code>POST /auth/logout</code>,{" "}
                  <code>GET /auth/me</code>, and a protected{" "}
                  <code>PATCH /profile</code> endpoint for profile updates.
                </p>
              </div>
            </aside>
          </div>
        </Card>
      </div>
    </main>
  );
}
