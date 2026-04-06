"use client";
import React, { useState, useEffect, useCallback } from 'react';
import Link from 'next/link';

export default function HomePage() {
  const [lang, setLang] = useState<"EN" | "TH">("EN");
  const [theme, setTheme] = useState<"light" | "dark">("light");

  const applyLang = useCallback((nextLang: "EN" | "TH") => {
    document.querySelectorAll("[data-th][data-en]").forEach((el) => {
      const htmlEl = el as HTMLElement;
      htmlEl.textContent =
        nextLang === "TH"
          ? htmlEl.dataset.th ?? ""
          : htmlEl.dataset.en ?? "";
    });
    localStorage.setItem("lang", nextLang);
  }, []);

  useEffect(() => {
    const stored = localStorage.getItem("lang") as "EN" | "TH" | null;
    if (stored === "TH") {
      setLang("TH");
      applyLang("TH");
    } else {
      applyLang("EN");
    }

    const currentTheme = localStorage.getItem("theme");
    if (currentTheme === "dark") {
      document.documentElement.classList.add("dark");
      setTheme("dark");
    }
  }, [applyLang]);

  const toggleLang = () => {
    const nextLang = lang === "EN" ? "TH" : "EN";
    setLang(nextLang);
    applyLang(nextLang);
  };

  const toggleTheme = () => {
    const html = document.documentElement;
    const isDark = html.classList.contains("dark");
    if (isDark) {
      html.classList.remove("dark");
      html.dataset.theme = "light";
      localStorage.setItem("theme", "light");
      setTheme("light");
    } else {
      html.classList.add("dark");
      html.dataset.theme = "dark";
      localStorage.setItem("theme", "dark");
      setTheme("dark");
    }
  };

  return (
    <div className="bg-background-light dark:bg-background-dark text-slate-900 dark:text-slate-100 font-body antialiased selection:bg-primary/20 selection:text-primary relative overflow-x-hidden min-h-screen">
      <style dangerouslySetInnerHTML={{
        __html: `
        /* Custom Utilities */
        .glass-panel {
            background: rgba(255, 255, 255, 0.65);
            backdrop-filter: blur(12px);
            -webkit-backdrop-filter: blur(12px);
            border: 1px solid rgba(255, 255, 255, 0.8);
            border-bottom-color: rgba(224, 230, 237, 0.6);
            border-right-color: rgba(224, 230, 237, 0.6);
        }
        
        .dark .glass-panel {
            background: #111111;
            border: 1px solid rgba(255, 255, 255, 0.03);
            box-shadow: none;
        }

        .dark .glass-panel:hover {
            border-color: rgba(41, 166, 90, 0.2);
            box-shadow: 0 8px 32px rgba(41, 166, 90, 0.05);
        }

        .glass-panel:hover {
            box-shadow: 0 8px 32px rgba(41, 166, 90, 0.08);
            transform: translateY(-2px);
            border-color: rgba(41, 166, 90, 0.3);
        }

        .text-glow {
            text-shadow: 0 0 20px rgba(41, 166, 90, 0.3);
        }
        
        .dark .text-glow {
            text-shadow: 0 0 20px rgba(41, 166, 90, 0.5);
        }

        /* Scanline animation for the ticker */
        @keyframes scan {
            0% { background-position: 0% 0%; }
            100% { background-position: 100% 100%; }
        }
        
        .grid-bg {
            background-image: linear-gradient(to right, rgba(41, 166, 90, 0.1) 1px, transparent 1px), 
                              linear-gradient(to bottom, rgba(41, 166, 90, 0.1) 1px, transparent 1px);
            background-size: 64px 64px;
        }
        
        .dark .grid-bg {
            opacity: 0.1;
            background-image: linear-gradient(to right, rgba(255, 255, 255, 0.02) 1px, transparent 1px), 
                              linear-gradient(to bottom, rgba(255, 255, 255, 0.02) 1px, transparent 1px);
        }

        .mesh-gradient {
            background: radial-gradient(at 0% 0%, hsla(140,69%,89%,1) 0, transparent 50%), 
                        radial-gradient(at 100% 0%, hsla(130,60%,92%,1) 0, transparent 50%), 
                        radial-gradient(at 100% 100%, hsla(150,50%,95%,1) 0, transparent 50%);
        }
        
        .dark .mesh-gradient {
            display: none;
        }
      `}} />

      {/* Background Grid Layer */}
      <div className="fixed inset-0 z-0 pointer-events-none grid-bg"></div>

      {/* Ambient Mesh Gradient */}
      <div className="fixed inset-0 z-0 pointer-events-none mesh-gradient opacity-60"></div>

      {/* Main Wrapper */}
      <div className="relative z-10 flex flex-col min-h-screen">
        {/* Navbar */}
        <header className="sticky top-0 z-50 w-full glass-panel dark:!bg-black dark:!border-b dark:!border-white/5 border-b-0 border-x-0 rounded-none bg-white/70">
          <div className="max-w-[1440px] mx-auto px-6 h-20 flex items-center justify-between">
            <div className="flex items-center gap-3">
              <img src="/white-outline.png" alt="Reverz Studio" className="w-12 h-12 opacity-90 invert dark:invert-0" />
              <span className="font-display font-bold text-xl tracking-widest uppercase text-slate-900 dark:text-white">REVERZ<span className="text-slate-400">.</span>STUDIO</span>
            </div>

            <nav className="hidden md:flex items-center gap-8">
            </nav>

            <div className="flex items-center gap-4">
              {/* Theme Toggle */}
              <button
                onClick={toggleTheme}
                className="text-slate-600 dark:text-neutral-300 hover:text-primary transition-colors flex items-center justify-center p-2"
                aria-label="Toggle theme"
              >
                {theme === 'dark' ? (
                  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
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
                ) : (
                  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
                  </svg>
                )}
              </button>

              {/* Lang Toggle */}
              <button onClick={toggleLang} className="flex items-center gap-2 text-sm font-bold text-slate-800 dark:text-neutral-300 hover:text-primary transition-colors pr-6 border-r border-slate-200 dark:border-neutral-900 tracking-widest">
                <span className={lang === "TH" ? "text-primary" : "opacity-70"}>TH</span>
                <span className="opacity-30">/</span>
                <span className={lang === "EN" ? "text-primary" : "opacity-70"}>EN</span>
              </button>

              <Link href="/login" className="hidden sm:flex text-sm tracking-widest font-bold text-slate-900 dark:text-white hover:text-primary transition-colors px-6 py-2 uppercase" data-th="เข้าสู่ระบบ" data-en="Login">
                Login
              </Link>
              <Link href="/register" className="flex items-center justify-center bg-primary hover:bg-primary-dark text-white text-sm tracking-widest font-bold px-8 py-3.5 rounded-none transition-all ml-2 uppercase">
                <span data-th="เริ่มต้นใช้งาน" data-en="GET STARTED">GET STARTED</span>
              </Link>
            </div>
          </div>
        </header>

        {/* Hero Section */}
        <main className="flex-grow flex flex-col justify-center">
          <div className="max-w-[1440px] mx-auto px-6 py-12 lg:py-24 w-full">
            <div className="grid lg:grid-cols-2 gap-12 lg:gap-24 items-center">

              {/* Left Column: Typography & CTA */}
              <div className="flex flex-col gap-8 max-w-2xl relative">
                <div className="space-y-4">
                  <h1 className="font-display font-bold text-5xl sm:text-6xl lg:text-7xl leading-[1.1] tracking-tight text-slate-900 dark:text-white">
                    <span data-th="เว็บโฮสติ้งที่" data-en="Hosting at the">Hosting at the</span> <br />
                    <span className="text-primary text-glow" data-th="เร็วระดับแสง" data-en="Speed of Light">Speed of Light</span>
                  </h1>
                  <p className="text-lg text-slate-600 dark:text-neutral-300 font-body leading-relaxed max-w-lg" data-th="เพิ่มประสิทธิภาพขั้นสุดให้โปรเจ็กต์ของคุณ เร็วกว่า ปลอดภัยกว่า และเชื่อถือได้มากกว่า ด้วยคลาวด์แพลตฟอร์มของเรา" data-en="Precision-engineered shared hosting. Zero clutter. Replaces budget hosting with a transparent, grid-locked interface that proves reliability.">
                    Precision-engineered shared hosting. Zero clutter. Replaces budget hosting with a transparent, grid-locked interface that proves reliability.
                  </p>
                </div>

                <div className="flex flex-col sm:flex-row gap-4 pt-4">
                  <Link href="/register" className="flex items-center justify-center gap-3 bg-primary hover:bg-primary-dark text-white font-display font-bold text-base px-8 py-4 rounded-none transition-all">
                    <span data-th="เริ่ม Deploy เลย" data-en="Start Deploying">Start Deploying</span>
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M5 12h14" /><path d="m12 5 7 7-7 7" /></svg>
                  </Link>
                </div>

                {/* Trust Indicators */}
                <div className="flex items-center gap-6 pt-8 border-t border-slate-200/60 dark:border-white/10 mt-4">
                  <div className="flex flex-col">
                    <span className="font-display font-bold text-2xl text-slate-900 dark:text-white">99.99%</span>
                    <span className="text-xs text-slate-500 dark:text-neutral-400 uppercase tracking-wider font-medium" data-th="รับประกัน Uptime" data-en="Uptime Guarantee">Uptime Guarantee</span>
                  </div>
                  <div className="w-px h-10 bg-slate-900 dark:bg-white"></div>
                  <div className="flex flex-col">
                    <span className="font-display font-bold text-2xl text-slate-900 dark:text-white">5000+</span>
                    <span className="text-xs text-slate-500 dark:text-neutral-400 uppercase tracking-wider font-medium" data-th="โหนดที่ทำงานอยู่" data-en="Nodes Active">Nodes Active</span>
                  </div>
                  <div className="w-px h-10 bg-slate-900 dark:bg-white"></div>
                  <div className="flex flex-col">
                    <span className="font-display font-bold text-2xl text-slate-900 dark:text-white">&lt; 100ms</span>
                    <span className="text-xs text-slate-500 dark:text-neutral-400 uppercase tracking-wider font-medium" data-th="ความหน่วงต่ำที่สุด" data-en="Global Latency">Global Latency</span>
                  </div>
                </div>
              </div>

              {/* Right Column: Abstract Visualization */}
              <div className="relative hidden lg:flex items-center justify-center h-full min-h-[500px]">
                {/* Abstract Geometric Shape / Crystal Server Representation */}
                <div className="relative w-full aspect-square max-w-[500px]">

                  {/* Main Crystal */}
                  <div className="absolute inset-0 glass-panel border border-white/40 dark:border-white/5 shadow-2xl shadow-primary/10 rounded-xl overflow-hidden transform rotate-3 hover:rotate-0 transition-transform duration-700 ease-out z-20 flex flex-col">
                    <div className="bg-slate-50/50 dark:bg-neutral-900/50 p-4 border-b border-slate-100 dark:border-neutral-800 flex items-center justify-between">
                      <div className="flex gap-2">
                        <div className="w-3 h-3 rounded-full bg-red-400"></div>
                        <div className="w-3 h-3 rounded-full bg-amber-400"></div>
                        <div className="w-3 h-3 rounded-full bg-emerald-400"></div>
                      </div>
                      <span className="font-mono text-xs text-slate-400">server-cluster-alpha</span>
                    </div>

                    <div className="p-6 flex-1 flex flex-col gap-4 relative bg-gradient-to-br from-white/40 to-white/10 dark:from-neutral-900/40 dark:to-neutral-900/10">
                      {/* Server Rack Visual */}
                      <div className="h-8 w-full bg-slate-100 dark:bg-neutral-900 rounded-sm flex items-center px-3 justify-between border border-slate-200 dark:border-neutral-800">
                        <div className="flex gap-2 items-center">
                          <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse"></div>
                          <span className="font-mono text-[10px] text-slate-500 dark:text-neutral-400">Node-01 // ACTIVE</span>
                        </div>
                        <span className="font-mono text-[10px] text-primary">34% LOAD</span>
                      </div>
                      <div className="h-8 w-full bg-slate-100 dark:bg-neutral-900 rounded-sm flex items-center px-3 justify-between border border-slate-200 dark:border-neutral-800 opacity-80">
                        <div className="flex gap-2 items-center">
                          <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse delay-75"></div>
                          <span className="font-mono text-[10px] text-slate-500 dark:text-neutral-400">Node-02 // ACTIVE</span>
                        </div>
                        <span className="font-mono text-[10px] text-primary">12% LOAD</span>
                      </div>
                      <div className="h-8 w-full bg-slate-100 dark:bg-neutral-900 rounded-sm flex items-center px-3 justify-between border border-slate-200 dark:border-neutral-800 opacity-60">
                        <div className="flex gap-2 items-center">
                          <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse delay-150"></div>
                          <span className="font-mono text-[10px] text-slate-500 dark:text-neutral-400">Node-03 // IDLE</span>
                        </div>
                        <span className="font-mono text-[10px] text-slate-400">--</span>
                      </div>

                      {/* Graph Area */}
                      <div className="mt-auto h-32 w-full border border-slate-200 dark:border-neutral-800 bg-slate-50/50 dark:bg-neutral-900/50 rounded-sm relative overflow-hidden flex items-end gap-1 px-1 pb-1">
                        <div className="w-1/12 bg-primary/20 h-[40%] rounded-sm"></div>
                        <div className="w-1/12 bg-primary/30 h-[60%] rounded-sm"></div>
                        <div className="w-1/12 bg-primary/40 h-[30%] rounded-sm"></div>
                        <div className="w-1/12 bg-primary/50 h-[80%] rounded-sm"></div>
                        <div className="w-1/12 bg-primary/60 h-[55%] rounded-sm"></div>
                        <div className="w-1/12 bg-primary/40 h-[45%] rounded-sm"></div>
                        <div className="w-1/12 bg-primary/30 h-[70%] rounded-sm"></div>
                        <div className="w-1/12 bg-primary/20 h-[50%] rounded-sm"></div>
                        <div className="w-1/12 bg-primary/10 h-[30%] rounded-sm"></div>
                        <div className="w-1/12 bg-primary/20 h-[60%] rounded-sm"></div>
                        <div className="w-1/12 bg-primary/30 h-[90%] rounded-sm"></div>
                        <div className="w-1/12 bg-primary h-[75%] rounded-sm animate-pulse"></div>
                        {/* Grid overlay on chart */}
                        <div className="absolute inset-0 bg-[url('data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxMCIgaGVpZ2h0PSIxMCI+PHBhdGggZD0iTTEwIDBMMCAwTDAgMTAiIGZpbGw9Im5vbmUiIHN0cm9rZT0icmdiYSgwLDAsMCwwLjA1KSIgc3Ryb2tlLXdpZHRoPSIxIi8+PC9zdmc+')] opacity-50 pointer-events-none"></div>
                      </div>
                    </div>
                  </div>

                  {/* Backing Element for Depth */}
                  <div className="absolute inset-0 bg-primary/5 rounded-xl transform -rotate-6 scale-95 translate-y-4 z-10 border border-primary/10"></div>

                  {/* Floating Decorative Elements */}
                  <div className="absolute -top-8 -right-8 w-24 h-24 bg-gradient-to-br from-primary/20 to-transparent rounded-full blur-2xl animate-pulse"></div>
                  <div className="absolute -bottom-12 -left-12 w-40 h-40 bg-gradient-to-tr from-blue-400/20 to-transparent rounded-full blur-3xl"></div>
                </div>
              </div>
            </div>

            {/* Feature Grid */}
            <div className="mt-24 lg:mt-32">
              <div className="grid md:grid-cols-3 gap-8">
                {/* Feature 1 */}
                <div className="glass-panel p-8 rounded-sm transition-all duration-300 group cursor-default">
                  <div className="flex justify-between items-start mb-6">
                    <div className="p-3 bg-slate-50 dark:bg-neutral-900 border border-slate-100 dark:border-neutral-800 rounded-sm text-primary group-hover:bg-primary group-hover:text-white transition-colors duration-300">
                      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M13 2 3 14h9l-1 8 10-12h-9l1-8z" /></svg>
                    </div>
                    <span className="font-mono text-xs text-slate-400 group-hover:text-primary transition-colors">SPEC_01</span>
                  </div>
                  <h3 className="font-display font-bold text-xl mb-3 text-slate-900 dark:text-white group-hover:text-primary transition-colors" data-th="สถาปัตยกรรม NVMe" data-en="NVMe Architecture">NVMe Architecture</h3>
                  <p className="text-slate-600 dark:text-neutral-400 text-sm leading-relaxed mb-6" data-th="พื้นที่เก็บข้อมูลความเร็วสูงสุดถึง 5000MB/s โหลดหน้าเว็บเร็วขึ้นทันใจ ไร้การรอคอย" data-en="Direct-attach storage delivering up to 5000MB/s read/write speeds for instant data retrieval.">
                    Direct-attach storage delivering up to 5000MB/s read/write speeds for instant data retrieval.
                  </p>
                  <div className="w-full bg-slate-100 dark:bg-neutral-900 h-1.5 rounded-full overflow-hidden">
                    <div className="bg-primary h-full w-[85%] rounded-full group-hover:animate-pulse"></div>
                  </div>
                  <div className="flex justify-between mt-2 font-mono text-[10px] text-slate-400">
                    <span>I/O LOAD</span>
                    <span>85% OPTIMAL</span>
                  </div>
                </div>

                {/* Feature 2 */}
                <div className="glass-panel p-8 rounded-sm transition-all duration-300 group cursor-default">
                  <div className="flex justify-between items-start mb-6">
                    <div className="p-3 bg-slate-50 dark:bg-neutral-900 border border-slate-100 dark:border-neutral-800 rounded-sm text-primary group-hover:bg-primary group-hover:text-white transition-colors duration-300">
                      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10" /><line x1="2" y1="12" x2="22" y2="12" /><path d="M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z" /></svg>
                    </div>
                    <span className="font-mono text-xs text-slate-400 group-hover:text-primary transition-colors">SPEC_02</span>
                  </div>
                  <h3 className="font-display font-bold text-xl mb-3 text-slate-900 dark:text-white group-hover:text-primary transition-colors" data-th="เครือข่าย CDN ทั่วโลก" data-en="Global CDN Mesh">Global CDN Mesh</h3>
                  <p className="text-slate-600 dark:text-neutral-400 text-sm leading-relaxed mb-6" data-th="กระจายแอปพลิเคชันของคุณไปตามขอบเครือข่าย ลดค่าความหน่วงให้ผู้ใช้งานได้อย่างสมบูรณ์" data-en="Content distributed across 24 edge locations. Your data is always local to your user.">
                    Content distributed across 24 edge locations. Your data is always local to your user.
                  </p>
                  <div className="flex gap-1 mt-auto">
                    <div className="h-1.5 w-1.5 rounded-full bg-emerald-400"></div>
                    <div className="h-1.5 w-1.5 rounded-full bg-emerald-400"></div>
                    <div className="h-1.5 w-1.5 rounded-full bg-emerald-400"></div>
                    <div className="h-1.5 w-1.5 rounded-full bg-emerald-400"></div>
                    <div className="h-1.5 w-1.5 rounded-full bg-emerald-400"></div>
                    <span className="text-[10px] font-mono text-slate-400 ml-2 leading-none">24/24 ONLINE</span>
                  </div>
                </div>

                {/* Feature 3 */}
                <div className="glass-panel p-8 rounded-sm transition-all duration-300 group cursor-default">
                  <div className="flex justify-between items-start mb-6">
                    <div className="p-3 bg-slate-50 dark:bg-neutral-900 border border-slate-100 dark:border-neutral-800 rounded-sm text-primary group-hover:bg-primary group-hover:text-white transition-colors duration-300">
                      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" /></svg>
                    </div>
                    <span className="font-mono text-xs text-slate-400 group-hover:text-primary transition-colors">SPEC_03</span>
                  </div>
                  <h3 className="font-display font-bold text-xl mb-3 text-slate-900 dark:text-white group-hover:text-primary transition-colors" data-th="ระบบป้องกัน DDoS" data-en="Active DDoS Shield">Active DDoS Shield</h3>
                  <p className="text-slate-600 dark:text-neutral-400 text-sm leading-relaxed mb-6" data-th="ป้องกันภัยคุกคามและการโจมตีบนเครือข่ายก่อนที่จะเข้าถึงโหลดของคุณแบบเรียลไทม์" data-en="Real-time traffic analysis filters malicious packets before they reach your node.">
                    Real-time traffic analysis filters malicious packets before they reach your node.
                  </p>
                  <div className="flex items-center gap-3 border border-slate-100 dark:border-neutral-800 bg-slate-50/50 dark:bg-neutral-900/50 p-2 rounded-sm">
                    <svg width="14" height="14" className="text-emerald-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><polyline points="20 6 9 17 4 12" /></svg>
                    <span className="font-mono text-[10px] text-slate-500 dark:text-neutral-400 uppercase" data-th="ป้องกันทำงานปกติ" data-en="Protection Active">Protection Active</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </main>

        {/* Minimal Footer */}
        <footer className="border-t border-slate-200 dark:border-white/10 bg-white/40 dark:bg-black/40 backdrop-blur-sm z-20">
          <div className="max-w-[1440px] mx-auto px-6 py-6 flex flex-col md:flex-row justify-between items-center gap-4">
            <div className="flex items-center gap-2">
              <svg width="16" height="16" className="text-slate-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10" /><path d="M14.83 14.83a4 4 0 1 1 0-5.66" /></svg>
              <span className="text-xs text-slate-500 dark:text-neutral-400 font-medium">2026 REVERZ.IN.TH Inc. All rights reserved.</span>
            </div>

            <div className="flex items-center gap-6">
              <a className="text-xs font-bold text-slate-500 dark:text-neutral-400 hover:text-primary transition-colors uppercase tracking-wide" href="#" data-th="ความเป็นส่วนตัว" data-en="Privacy">Privacy</a>
              <a className="text-xs font-bold text-slate-500 dark:text-neutral-400 hover:text-primary transition-colors uppercase tracking-wide" href="#" data-th="ข้อตกลง" data-en="Terms">Terms</a>
              <div className="h-4 w-px bg-slate-300 dark:bg-neutral-800"></div>
              <div className="flex items-center gap-2">
                <span className="relative flex h-1.5 w-1.5">
                  <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-emerald-500"></span>
                </span>
                <span className="text-xs font-mono text-slate-500 dark:text-white">ALL SYSTEMS GO</span>
              </div>
            </div>
          </div>
        </footer>
      </div>
    </div>
  );
}
