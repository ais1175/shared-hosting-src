"use client";

import { useEffect, useState, useCallback } from "react";
import Link from "next/link";
import Image from "next/image";
import styles from "./Navbar.module.css";

type NavLinkItem = {
  href: string;
  label: string;
  thLabel: string;
  key: string;
  external?: boolean;
};

export default function Navbar({ activePage = "home" }: { activePage?: string }) {
  const [scrolled, setScrolled] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);
  const [activeLink, setActiveLink] = useState(activePage === "home" ? "/" : `#${activePage}`);
  const [lang, setLang] = useState<"EN" | "TH">("EN");

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 60);
    window.addEventListener("scroll", onScroll, { passive: true });
    onScroll();
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  useEffect(() => {
    const isHome = window.location.pathname === "/";
    if (!isHome) return;

    const sections = ["hero", "features", "pricing"];
    const sectionToHref: Record<string, string> = {
      hero: "/",
      features: "#features",
      pricing: "#pricing",
    };

    function updateActive() {
      let current = "hero";
      const scrollY = window.scrollY + 120;
      for (const id of sections) {
        const el = document.getElementById(id);
        if (el && el.offsetTop <= scrollY) current = id;
      }
      setActiveLink(sectionToHref[current] || "/");
    }

    window.addEventListener("scroll", updateActive, { passive: true });
    updateActive();
    return () => window.removeEventListener("scroll", updateActive);
  }, []);

  const applyLang = useCallback((nextLang: "EN" | "TH") => {
    document.querySelectorAll("[data-th][data-en]").forEach((el) => {
      const htmlEl = el as HTMLElement;
      htmlEl.textContent =
        nextLang === "TH"
          ? htmlEl.dataset.th ?? htmlEl.textContent
          : htmlEl.dataset.en ?? htmlEl.textContent;
    });
    localStorage.setItem("lang", nextLang);
  }, []);

  useEffect(() => {
    const stored = localStorage.getItem("lang") as "EN" | "TH" | null;
    if (stored === "TH") {
      setLang("TH");
      applyLang("TH");
    }
  }, [applyLang]);

  const toggleLang = () => {
    const nextLang = lang === "EN" ? "TH" : "EN";
    setLang(nextLang);
    applyLang(nextLang);
  };

  const toggleTheme = () => {
    const html = document.documentElement;
    const nextTheme = html.dataset.theme === "dark" ? "light" : "dark";
    html.dataset.theme = nextTheme;
    localStorage.setItem("theme", nextTheme);
  };

  const navLinks: NavLinkItem[] = [
    { href: "/", label: "Home", thLabel: "หน้าแรก", key: "/" },
    { href: "#features", label: "Features", thLabel: "ฟีเจอร์", key: "#features" },
    { href: "#pricing", label: "Pricing", thLabel: "ราคา", key: "#pricing" },
    {
      href: "https://discord.gg/wGQMHMfeua",
      label: "Support",
      thLabel: "ช่วยเหลือ",
      key: "/support",
      external: true,
    },
  ];

  return (
    <nav className={`${styles.navbar} ${scrolled ? styles.scrolled : ""}`} id="navbar">
      <div className={styles.navbarInner}>
        <Link href="/" className={styles.navbarBrand} aria-label="Reverz Studio Home">
          <Image
            src="/white-outline.png"
            alt="Reverz Studio"
            className={styles.navbarLogo}
            width={46}
            height={46}
            priority
          />
          <span className={styles.navbarBrandText}>
            REVERZ<span className={styles.brandDot}>.</span>STUDIO
          </span>
        </Link>

        <div className={styles.navbarLinks}>
          {navLinks.map((link) => (
            <a
              key={link.key}
              href={link.href}
              className={`${styles.navLink} ${activeLink === link.key ? styles.active : ""}`}
              data-th={link.thLabel}
              data-en={link.label}
              target={link.external ? "_blank" : undefined}
              rel={link.external ? "noreferrer" : undefined}
              onClick={() => setActiveLink(link.key)}
            >
              {link.label}
            </a>
          ))}
        </div>

        <div className={styles.navbarActions}>
          <button className={styles.themeToggle} onClick={toggleTheme} aria-label="Toggle theme">
            <svg className={styles.iconMoon} width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
            </svg>
            <svg className={styles.iconSun} width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="5" />
              <line x1="12" y1="1" x2="12" y2="3" />
              <line x1="12" y1="21" x2="12" y2="23" />
              <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
              <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
              <line x1="1" y1="12" x2="3" y2="12" />
              <line x1="21" y1="12" x2="23" y2="12" />
              <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
              <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
            </svg>
          </button>

          <button className={styles.langToggle} onClick={toggleLang} aria-label="Toggle language">
            <span className={styles.langOption} data-active={lang === "TH" ? "true" : "false"}>
              TH
            </span>
            <span className={styles.langSep}>/</span>
            <span className={styles.langOption} data-active={lang === "EN" ? "true" : "false"}>
              EN
            </span>
          </button>

          <Link href="/register" className={`${styles.navBtn} ${styles.navBtnGhost}`} data-th="สมัครสมาชิก" data-en="Sign Up">
            Sign Up
          </Link>
          <Link href="/register" className={`${styles.navBtn} ${styles.navBtnPrimary}`} data-th="เริ่มต้นใช้งาน" data-en="Get Started">
            Get Started
          </Link>
        </div>

        <button
          className={`${styles.navbarToggle} ${mobileOpen ? styles.open : ""}`}
          onClick={() => setMobileOpen(!mobileOpen)}
          aria-label="Toggle navigation"
        >
          <span className={styles.toggleBar}></span>
          <span className={styles.toggleBar}></span>
          <span className={styles.toggleBar}></span>
        </button>
      </div>

      <div className={`${styles.navbarMobile} ${mobileOpen ? styles.mobileOpen : ""}`}>
        {navLinks.map((link) => (
          <a
            key={link.key}
            href={link.href}
            className={`${styles.navLinkMobile} ${activeLink === link.key ? styles.active : ""}`}
            data-th={link.thLabel}
            data-en={link.label}
            target={link.external ? "_blank" : undefined}
            rel={link.external ? "noreferrer" : undefined}
            onClick={() => {
              setActiveLink(link.key);
              setMobileOpen(false);
            }}
          >
            {link.label}
          </a>
        ))}
        <div className={styles.mobileActions}>
          <Link
            href="/register"
            className={`${styles.navBtn} ${styles.navBtnPrimary}`}
            style={{ width: "100%", textAlign: "center" }}
            data-th="เริ่มต้นใช้งาน"
            data-en="Get Started"
          >
            Get Started
          </Link>
        </div>
      </div>
    </nav>
  );
}
