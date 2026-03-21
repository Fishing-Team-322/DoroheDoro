export default function Loading() {
  return (
    <main className="flex min-h-screen items-center justify-center bg-white p-6 dark:bg-black">
      <div className="flex flex-col items-center gap-4">
        <div className="loader" />
        <p className="text-sm text-zinc-600 dark:text-zinc-400">Загрузка...</p>
      </div>
    </main>
  );
}