import Link from 'next/link';

import { Button } from '@/src/shared/ui/button';

export default function NotFound() {
  return (
    <main className="flex min-h-screen items-center justify-center bg-black p-6">
      <div className="flex flex-col items-center text-center">
        <p className="text-[100px] leading-none text-[#424242]">
          cтраница не найдена :(
        </p>

        <p className="mt-8 text-[30px] text-white">
          походу такой страницы нету, идите на главную
        </p>

        <Button className="mt-8">
          <Link href="/">На главную</Link>
        </Button>
      </div>
    </main>
  );
}