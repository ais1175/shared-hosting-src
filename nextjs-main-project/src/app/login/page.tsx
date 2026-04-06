"use client";

import { useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { setAuthSession } from "@/lib/authSession";
import { loginWithPassword } from "@/lib/authClient";
import { computeLoginProof } from "@/lib/wasmProof";
import styles from "./page.module.css";

const TEST_USERNAME = "root";
const TEST_PASSWORD = "root";

export default function LoginPage() {
  const router = useRouter();
  const [showPassword, setShowPassword] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setLoading(true);
    try {
      const nonce = `${Date.now()}`;
      const proof = await computeLoginProof(username, password, nonce);
      const session = await loginWithPassword({ username, password, nonce, proof });
      setAuthSession(session);
      router.push("/dashboard");
    } catch (error) {
      setError(error instanceof Error ? error.message : "Login failed. Check credentials or backend API.");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className={styles.page}>
      <div className={styles.bg}>
        <div className={`${styles.orb} ${styles.orb1}`}></div>
        <div className={`${styles.orb} ${styles.orb2}`}></div>
        <div className={styles.gridOverlay}></div>
      </div>

      {/* Left branding */}
      <div className={styles.branding}>
        <div className={styles.brandingContent}>
          <Link href="/" className={styles.logoLink}>
            <img src="/white-outline.png" alt="Reverz Studio" className={styles.logo} loading="lazy" width={64} height={64} />
          </Link>
          <h2 className={styles.brandingTitle}>Welcome <span className="gradient-text">back</span></h2>
          <p className={styles.brandingDesc}>Sign in to manage your deployments and services on Reverz Studio.</p>

          <div className={styles.brandingFeatures}>
            {[
              { icon: <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />, title: "Secure Access", desc: "End-to-end encrypted authentication" },
              { icon: <><polyline points="22 12 18 12 15 21 9 3 6 12 2 12" /></>, title: "Real-time Dashboard", desc: "Monitor all your services instantly" },
              { icon: <><rect x="3" y="11" width="18" height="11" rx="2" ry="2" /><path d="M7 11V7a5 5 0 0110 0v4" /></>, title: "2FA Protected", desc: "Optional two-factor authentication" },
            ].map((f, i) => (
              <div key={i} className={styles.brandingFeature}>
                <div className={styles.bfIcon}>
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">{f.icon}</svg>
                </div>
                <div>
                  <span className={styles.bfTitle}>{f.title}</span>
                  <span className={styles.bfDesc}>{f.desc}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Right form */}
      <div className={styles.formPanel}>
        <div className={styles.formInner}>
          <div className={styles.formHeader}>
            <h1 className={styles.formTitle}>Sign In</h1>
            <p className={styles.formSubtitle}>Access your Reverz Studio dashboard and manage your services.</p>
          </div>

          <div className={styles.testCard}>
            <span className={styles.testBadge}>Test Account</span>
            <p className={styles.testText}>For testing: username <strong>root</strong> and password <strong>root</strong>.</p>
            <button
              type="button"
              className={styles.testFillBtn}
              onClick={() => {
                setUsername(TEST_USERNAME);
                setPassword(TEST_PASSWORD);
                setError("");
              }}
            >
              Fill root/root
            </button>
          </div>

          <div className={styles.socialSignup}>
            <button type="button" className={styles.socialBtn}>
              <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"><path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"/></svg>
              <span>Continue with GitHub</span>
            </button>
            <button type="button" className={styles.socialBtn}>
              <svg width="20" height="20" viewBox="0 0 24 24"><path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92a5.06 5.06 0 01-2.2 3.32v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.1z" fill="#4285F4"/><path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" fill="#34A853"/><path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" fill="#FBBC05"/><path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" fill="#EA4335"/></svg>
              <span>Continue with Google</span>
            </button>
          </div>

          <div className={styles.divider}>
            <span className={styles.dividerLine}></span>
            <span className={styles.dividerText}>or sign in with email</span>
            <span className={styles.dividerLine}></span>
          </div>

          <form className={styles.form} onSubmit={handleSubmit} noValidate>
            <div className={styles.formGroup}>
              <label className={styles.formLabel} htmlFor="username">Username</label>
              <input
                type="text"
                id="username"
                className={styles.formInput}
                placeholder="Enter your username"
                required
                autoComplete="username"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
              />
            </div>

            <div className={styles.formGroup}>
              <div className={styles.labelRow}>
                <label className={styles.formLabel} htmlFor="password">Password</label>
                <Link href="/forgot-password" className={styles.forgotLink}>Forgot password?</Link>
              </div>
              <div className={styles.passwordWrap}>
                <input
                  type={showPassword ? "text" : "password"}
                  id="password"
                  className={styles.formInput}
                  placeholder="Enter your password"
                  required
                  autoComplete="current-password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                />
                <button type="button" className={styles.passwordToggle} onClick={() => setShowPassword(!showPassword)} aria-label="Show password">
                  {showPassword ? (
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 015.06-5.94M9.9 4.24A9.12 9.12 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19m-6.72-1.07a3 3 0 11-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
                  ) : (
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
                  )}
                </button>
              </div>
            </div>

            {error ? <p className={styles.errorText}>{error}</p> : null}

            <div className={styles.formGroup}>
              <label className={styles.checkboxWrap}>
                <input type="checkbox" className={styles.checkboxInput} />
                <span className={styles.checkboxCustom}></span>
                <span className={styles.checkboxLabel}>Remember me for 30 days</span>
              </label>
            </div>

            <button type="submit" className={styles.btnSubmit} disabled={loading}>
              {loading ? (
                <svg className={styles.spinner} width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><circle cx="12" cy="12" r="10" strokeOpacity="0.25"/><path d="M12 2a10 10 0 019.95 9"/></svg>
              ) : (
                <span>Sign In</span>
              )}
            </button>
          </form>

          <p className={styles.formFooter}>
            Don&apos;t have an account? <Link href="/register" className={styles.formLink}>Create one</Link>
          </p>
        </div>
      </div>
    </div>
  );
}
