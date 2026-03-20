"use client";

type ErrorPageProps = {
  error: Error & { digest?: string };
  reset: () => void;
};

export default function ErrorPage({ error, reset }: ErrorPageProps) {
  return (
    <main className="flex min-h-screen items-center justify-center p-6">
      <div className="w-full max-w-md rounded-lg border border-black/10 bg-white p-8 text-center shadow-sm dark:border-white/15 dark:bg-neutral-950">
        <h1 className="text-2xl font-semibold">Что-то пошло не так</h1>
        <p className="mt-2 text-sm text-zinc-600 dark:text-zinc-400">
          {error.message || "Произошла непредвиденная ошибка."}
        </p>
        <button
          type="button"
          onClick={() => reset()}
          className="mt-6 inline-flex h-10 items-center justify-center rounded-md border border-black/15 px-4 text-sm font-medium transition-colors hover:bg-black/5 dark:border-white/20 dark:hover:bg-white/10"
        >
          Попробовать снова
        </button>
      </div>
    </main>
  );
}
