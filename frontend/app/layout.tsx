import type { ReactNode } from "react";
import type { Metadata } from "next";
import localFont from "next/font/local";
import { defaultLocale } from "@/src/shared/config";
import "./globals.css";

const inter = localFont({
  src: [
    {
      path: "../public/font/Inter-VariableFont_opsz,wght.ttf",
      style: "normal",
      weight: "100 900",
    },
    {
      path: "../public/font/Inter-Italic-VariableFont_opsz,wght.ttf",
      style: "italic",
      weight: "100 900",
    },
  ],
  display: "swap",
  fallback: ["Arial", "Helvetica", "sans-serif"],
});

export const metadata: Metadata = {
  title: "DoroheDoro Dashboard",
  description: "Infrastructure dashboard with localized routes and authenticated entry flow.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: ReactNode;
}>) {
  return (
    <html lang={defaultLocale}>
      <body className={`${inter.className} antialiased text-[color:var(--foreground)]`}>
        {children}
      </body>
    </html>
  );
}
