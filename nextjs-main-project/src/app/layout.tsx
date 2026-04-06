import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Reverz Studio - Premium Web Hosting",
  description:
    "High-performance cloud hosting powered by Reverz Studio. Lightning-fast servers, 99.99% uptime, and enterprise-grade security.",
};

const themeScript = `
  (function () {
    try {
      var t = localStorage.getItem("theme");
      var isDark = t === "dark";
      document.documentElement.dataset.theme = isDark ? "dark" : "light";
      if (isDark) {
        document.documentElement.classList.add("dark");
      }
    } catch (e) {
      document.documentElement.dataset.theme = "light";
    }
  })();
`;

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" data-theme="light" suppressHydrationWarning>
      <head>
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />
        <link rel="icon" type="image/svg+xml" href="/favicon.svg" />
        <link rel="icon" href="/favicon.ico" />
        <script src="https://cdn.tailwindcss.com?plugins=forms,container-queries" async={false}></script>
        <link href="https://fonts.googleapis.com/css2?family=IBM+Plex+Sans+Thai:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
        <link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet" />
        <script dangerouslySetInnerHTML={{
          __html: `
            tailwind.config = {
                darkMode: "class",
                theme: {
                    extend: {
                        colors: {
                            "primary": "#29A65A",
                            "primary-dark": "#1F7F4A",
                            "background-light": "#f8fdfa",
                            "background-dark": "#000000",
                            "glass-border": "rgba(255, 255, 255, 0.4)",
                            "glass-surface": "rgba(255, 255, 255, 0.65)",
                        },
                        fontFamily: {
                            "display": ["'IBM Plex Sans Thai'", "sans-serif"],
                            "body": ["'IBM Plex Sans Thai'", "sans-serif"],
                            "mono": ["JetBrains Mono", "monospace"],
                        },
                        backgroundImage: {
                            'grid-pattern': "linear-gradient(to right, #E0E6ED 1px, transparent 1px), linear-gradient(to bottom, #E0E6ED 1px, transparent 1px)",
                        },
                        backgroundSize: {
                            'grid-64': '64px 64px',
                        }
                    },
                },
            }
          `
        }} />
      </head>
      <body>
        <a href="#main-content" className="skip-link">
          Skip to content
        </a>
        <main id="main-content">{children}</main>
      </body>
    </html>
  );
}
