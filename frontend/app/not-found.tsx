export default function NotFound() {
  return (
    <main className="flex min-h-screen items-center justify-center p-6">
      <div className="w-full max-w-md rounded-lg border border-black/10 bg-white p-8 text-center shadow-sm dark:border-white/15 dark:bg-neutral-950">
        <h1 className="text-2xl font-semibold">404</h1>
        <p className="mt-2 text-sm text-zinc-600 dark:text-zinc-400">
          Страница не найдена.
        </p>
      </div>
    </main>
  );
}
