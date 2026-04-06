"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { IBM_Plex_Mono, IBM_Plex_Sans, IBM_Plex_Sans_Thai } from "next/font/google";
import { adminLogin } from "@/lib/adminClient";
import { getAdminSession, setAdminSession } from "@/lib/adminSession";
import { computeLoginProof } from "@/lib/wasmProof";
import styles from "./page.module.css";

const ibmPlexSans = IBM_Plex_Sans({
  subsets: ["latin"],
  weight: ["300", "400", "500", "600", "700"],
  variable: "--font-ibm-plex-sans",
});
const ibmPlexSansThai = IBM_Plex_Sans_Thai({
  subsets: ["thai"],
  weight: ["300", "400", "500", "600", "700"],
  variable: "--font-ibm-plex-sans-thai",
});
const ibmPlexMono = IBM_Plex_Mono({
  subsets: ["latin"],
  weight: ["400", "600"],
  variable: "--font-ibm-plex-mono",
});

export default function AdminLoginPage() {
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const session = getAdminSession();
    if (session?.role === "admin") {
      router.replace("/admin-dashboard");
    }
  }, [router]);

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const nonce = `${Date.now()}`;
      const proof = await computeLoginProof(email.trim(), password, nonce);
      const session = await adminLogin({
        email: email.trim(),
        password,
        nonce,
        proof,
      });
      setAdminSession(session);
      router.push("/admin-dashboard");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Admin login failed");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className={`${styles.adminLoginRoot} ${ibmPlexSans.variable} ${ibmPlexSansThai.variable} ${ibmPlexMono.variable}`}>
      <div className={styles.shell}>
        <aside className={styles.panelInfo}>
          <p className={styles.kicker}>REVERZ ADMIN</p>
          <h1 className={styles.title}>Admin Control Panel</h1>
          <p className={styles.desc}>Secure sign-in for global service monitoring and transaction visibility.</p>
          <div className={styles.featureList}>
            <p>Global user/service aggregates</p>
            <p>Read-only operations</p>
            <p>Masked credential visibility</p>
          </div>
        </aside>

        <section className={styles.panelForm}>
          <h2 className={styles.formTitle}>/admin-login</h2>
          <p className={styles.formSub}>Sign in with admin email and password</p>

          <form onSubmit={handleSubmit} className={styles.form}>
            <label className={styles.label}>
              Email
              <input
                className={styles.input}
                type="email"
                autoComplete="username"
                value={email}
                onChange={(event) => setEmail(event.target.value)}
                placeholder="admin@reverz.in.th"
                required
              />
            </label>

            <label className={styles.label}>
              Password
              <div className={styles.passwordWrap}>
                <input
                  className={styles.input}
                  type={showPassword ? "text" : "password"}
                  autoComplete="current-password"
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                  placeholder="••••••••••"
                  required
                />
                <button
                  type="button"
                  className={styles.toggleBtn}
                  onClick={() => setShowPassword((prev) => !prev)}
                >
                  {showPassword ? "Hide" : "Show"}
                </button>
              </div>
            </label>

            {error ? <p className={styles.errorText}>{error}</p> : null}

            <button type="submit" className={styles.submitBtn} disabled={loading}>
              {loading ? "Signing in..." : "Sign In Admin"}
            </button>
          </form>
        </section>
      </div>
    </div>
  );
}
