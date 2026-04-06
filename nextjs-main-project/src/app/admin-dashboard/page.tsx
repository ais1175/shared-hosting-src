"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { IBM_Plex_Mono, IBM_Plex_Sans, IBM_Plex_Sans_Thai } from "next/font/google";
import {
  clearAdminSession,
  getAdminSession,
  type AdminSession,
} from "@/lib/adminSession";
import {
  adminLogout,
  getAdminRecentServices,
  getAdminRecentTransactions,
  getAdminSummary,
  type AdminServiceView,
  type AdminSummaryResponse,
  type AdminTransactionView,
  getAdminUserWallets,
  type AdminUserWalletView,
} from "@/lib/adminClient";
import { ApiRequestError } from "@/lib/authClient";
import styles from "../dashboard/page.module.css";

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

type AdminView = "overview" | "services" | "transactions";

function isAuthInvalidError(error: unknown): boolean {
  if (!(error instanceof ApiRequestError)) return false;
  if (error.status !== 401) return false;
  return error.errorCode === "UNAUTHENTICATED" || error.errorCode === "INVALID_REFRESH_TOKEN";
}

function formatDateTime(raw: string) {
  const parsed = new Date(raw);
  if (Number.isNaN(parsed.getTime())) return raw || "-";
  return parsed.toLocaleString("th-TH", { timeZone: "Asia/Bangkok" });
}

export default function AdminDashboardPage() {
  const router = useRouter();
  const [theme, setTheme] = useState<"light" | "dark">("dark");
  const [lang, setLang] = useState<"EN" | "TH">("EN");
  const [isSidebarOpen, setIsSidebarOpen] = useState(true);
  const [view, setView] = useState<AdminView>("overview");
  const [adminSession, setAdminSession] = useState<AdminSession | null>(null);

  const [summary, setSummary] = useState<AdminSummaryResponse | null>(null);
  const [services, setServices] = useState<AdminServiceView[]>([]);
  const [transactions, setTransactions] = useState<AdminTransactionView[]>([]);
  const [userWallets, setUserWallets] = useState<AdminUserWalletView[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refreshData = async () => {
    setLoading(true);
    try {
      const [summaryPayload, servicesPayload, transactionsPayload, userWalletsPayload] = await Promise.all([
        getAdminSummary(),
        getAdminRecentServices(50),
        getAdminRecentTransactions(50),
        getAdminUserWallets(),
      ]);
      setSummary(summaryPayload);
      setServices(servicesPayload.items);
      setTransactions(transactionsPayload.items);
      setUserWallets(userWalletsPayload.items);
      setError(null);
    } catch (err) {
      if (isAuthInvalidError(err)) {
        clearAdminSession();
        router.replace("/admin-login");
        return;
      }
      setError(err instanceof Error ? err.message : "Unable to load admin dashboard.");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    const session = getAdminSession();
    if (!session || session.role !== "admin") {
      router.replace("/admin-login");
      return;
    }
    setAdminSession(session);

    const storedLang = localStorage.getItem("lang");
    if (storedLang === "TH" || storedLang === "EN") setLang(storedLang);
    const storedTheme = document.documentElement.dataset.theme;
    if (storedTheme === "light" || storedTheme === "dark") setTheme(storedTheme);

    void refreshData();
  }, [router]);

  const notificationCount = summary?.unread_notifications_total ?? 0;
  const toggleTheme = () => {
    const nextTheme = theme === "light" ? "dark" : "light";
    setTheme(nextTheme);
    document.documentElement.dataset.theme = nextTheme;
    localStorage.setItem("theme", nextTheme);
  };

  const handleLogout = async () => {
    try {
      await adminLogout();
    } catch {
      // noop
    } finally {
      clearAdminSession();
      router.replace("/admin-login");
    }
  };

  const statusLabel = (status: string) => {
    if (status === "active") return "Active";
    if (status === "grace_suspended") return "Grace Suspended";
    if (status === "suspended_expired") return "Expired";
    return status;
  };

  const voucherMethodLabel = (method: string) => {
    if (method === "banking") return "Banking";
    if (method === "truemoney") return "TrueMoney";
    if (method === "hosting_order") return "Hosting Order";
    return method || "-";
  };

  return (
    <div className={`${styles.carbonRoot} ${ibmPlexSans.variable} ${ibmPlexSansThai.variable} ${ibmPlexMono.variable}`}>
      <header className={styles.header}>
        <div className={styles.headerLeft}>
          <button className={styles.headerBtn} onClick={() => setIsSidebarOpen((prev) => !prev)}>
            <span className={styles.icon}>menu</span>
          </button>
          <div className={styles.brand}>
            <span className={styles.brandText}>Reverz Admin</span>
            <span className={styles.brandSpan}>global-monitor</span>
          </div>
        </div>
        <div className={styles.headerRight}>
          <button className={styles.headerIcon} onClick={toggleTheme} title="Toggle Theme">
            <span className={styles.icon}>{theme === "light" ? "dark_mode" : "light_mode"}</span>
          </button>
          <button className={styles.headerIcon} title="Unread notifications">
            <span className={styles.icon}>notifications</span>
            {notificationCount > 0 ? <span className={styles.headerBadge}>{notificationCount}</span> : null}
          </button>
          <button
            className={styles.headerIcon}
            onClick={() => {
              const nextLang = lang === "EN" ? "TH" : "EN";
              setLang(nextLang);
              localStorage.setItem("lang", nextLang);
            }}
            title="Switch language"
          >
            <span style={{ fontSize: "14px", fontWeight: "bold" }}>{lang === "EN" ? "TH" : "EN"}</span>
          </button>
          <button className={styles.headerIcon} onClick={() => void handleLogout()} title="Sign out">
            <span className={styles.icon}>logout</span>
          </button>
        </div>
      </header>

      <div className={styles.mainWrapper}>
        <aside className={`${styles.sidebar} ${isSidebarOpen ? "" : styles.sidebarCollapsed}`}>
          <div className={styles.navGroup}>
            <button className={`${styles.navItem} ${view === "overview" ? styles.navItemActive : ""}`} onClick={() => setView("overview")}>
              <span className={`${styles.icon} ${styles.navItemIcon}`}>dashboard</span>
              <span className={styles.navItemText}>{lang === "TH" ? "ภาพรวม" : "Overview"}</span>
            </button>
            <button className={`${styles.navItem} ${view === "services" ? styles.navItemActive : ""}`} onClick={() => setView("services")}>
              <span className={`${styles.icon} ${styles.navItemIcon}`}>dns</span>
              <span className={styles.navItemText}>{lang === "TH" ? "บริการทั้งหมด" : "All Services"}</span>
            </button>
            <button className={`${styles.navItem} ${view === "transactions" ? styles.navItemActive : ""}`} onClick={() => setView("transactions")}>
              <span className={`${styles.icon} ${styles.navItemIcon}`}>receipt_long</span>
              <span className={styles.navItemText}>{lang === "TH" ? "ธุรกรรมล่าสุด" : "Recent Transactions"}</span>
            </button>
            <div className={styles.divider} />
            <span className={styles.navLabel}>ADMIN SESSION</span>
            <button className={styles.navItem} disabled>
              <span className={`${styles.icon} ${styles.navItemIcon}`}>shield</span>
              <span className={styles.navItemText}>{adminSession?.username ?? "admin"}</span>
            </button>
          </div>
        </aside>

        <main className={styles.content}>
          <div className={styles.container}>
            {error ? <p className={styles.errorText}>{error}</p> : null}

            {view === "overview" && (
              <>
                <div className={styles.pageHeader}>
                  <h1 className={styles.pageTitle}>{lang === "TH" ? "ภาพรวมระบบ (Admin)" : "System Overview (Admin)"}</h1>
                </div>
                <div className={styles.grid}>
                  <div className={styles.tile} style={{ height: "auto" }}>
                    <div className={styles.tileHeader}>
                      <h3 className={styles.tileTitle}>{lang === "TH" ? "ผู้ใช้ทั้งหมด" : "Total Users"}</h3>
                      <span className={`${styles.icon} ${styles.tileIcon}`}>group</span>
                    </div>
                    <div className={styles.tileValueRow}>
                      <span className={`${styles.tilePrimaryVal} ${styles.mono}`}>{loading ? "..." : summary?.total_users ?? 0}</span>
                    </div>
                  </div>
                  <div className={styles.tile} style={{ height: "auto" }}>
                    <div className={styles.tileHeader}>
                      <h3 className={styles.tileTitle}>{lang === "TH" ? "บริการที่ใช้งาน" : "Active Services"}</h3>
                      <span className={`${styles.icon} ${styles.tileIcon}`}>cloud_done</span>
                    </div>
                    <div className={styles.tileValueRow}>
                      <span className={`${styles.tilePrimaryVal} ${styles.mono}`}>{loading ? "..." : summary?.total_active_services ?? 0}</span>
                    </div>
                  </div>
                  <div className={styles.tile} style={{ height: "auto" }}>
                    <div className={styles.tileHeader}>
                      <h3 className={styles.tileTitle}>{lang === "TH" ? "ธุรกรรมทั้งหมด" : "Total Transactions"}</h3>
                      <span className={`${styles.icon} ${styles.tileIcon}`}>swap_horiz</span>
                    </div>
                    <div className={styles.tileValueRow}>
                      <span className={`${styles.tilePrimaryVal} ${styles.mono}`}>{loading ? "..." : summary?.total_transactions ?? 0}</span>
                    </div>
                  </div>
                </div>

                <div className={styles.colSpan3} style={{ marginTop: "16px" }}>
                  <div className={styles.tableWrap}>
                    <table className={styles.table}>
                      <thead>
                        <tr>
                          <th>{lang === "TH" ? "ผู้ใช้" : "User"}</th>
                          <th>{lang === "TH" ? "ยอดเงินคงเหลือ (THB)" : "Balance (THB)"}</th>
                        </tr>
                      </thead>
                      <tbody>
                        {userWallets.length === 0 ? (
                          <tr className={styles.tableRow}>
                            <td colSpan={2} style={{ textAlign: "center", color: "var(--text-muted)" }}>
                              {loading ? "Loading..." : "No users"}
                            </td>
                          </tr>
                        ) : (
                          userWallets.map((item) => (
                            <tr key={item.username} className={styles.tableRow}>
                              <td>{item.username}</td>
                              <td className={styles.mono}>{item.balance_thb.toFixed(2)}</td>
                            </tr>
                          ))
                        )}
                      </tbody>
                    </table>
                  </div>
                </div>
              </>
            )}

            {view === "services" && (
              <div className={styles.colSpan3}>
                <div className={styles.tableWrap}>
                  <table className={styles.table}>
                    <thead>
                      <tr>
                        <th>{lang === "TH" ? "เจ้าของ" : "Owner"}</th>
                        <th>{lang === "TH" ? "โดเมน" : "Domain"}</th>
                        <th>{lang === "TH" ? "แพ็กเกจ" : "Package"}</th>
                        <th>{lang === "TH" ? "สถานะ" : "Status"}</th>
                        <th>{lang === "TH" ? "หมดอายุ" : "Expires"}</th>
                        <th>{lang === "TH" ? "DA Username" : "DA Username"}</th>
                        <th>{lang === "TH" ? "DA Password" : "DA Password"}</th>
                        <th>{lang === "TH" ? "สร้างเมื่อ" : "Created"}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {services.length === 0 ? (
                        <tr className={styles.tableRow}>
                          <td colSpan={8} style={{ textAlign: "center", color: "var(--text-muted)" }}>
                            {loading ? "Loading..." : "No services"}
                          </td>
                        </tr>
                      ) : (
                        services.map((item) => (
                          <tr key={`${item.owner_username}-${item.domain}-${item.created_at}`} className={styles.tableRow}>
                            <td>{item.owner_username}</td>
                            <td className={styles.mono}>{item.domain}</td>
                            <td>{item.package_name}</td>
                            <td>{statusLabel(item.status)}</td>
                            <td>{formatDateTime(item.expires_at)}</td>
                            <td className={styles.mono}>{item.da_username_masked}</td>
                            <td className={styles.mono}>{item.da_password_masked}</td>
                            <td>{formatDateTime(item.created_at)}</td>
                          </tr>
                        ))
                      )}
                    </tbody>
                  </table>
                </div>
              </div>
            )}

            {view === "transactions" && (
              <div className={styles.colSpan3}>
                <div className={styles.tableWrap}>
                  <table className={styles.table}>
                    <thead>
                      <tr>
                        <th>{lang === "TH" ? "เจ้าของ" : "Owner"}</th>
                        <th>{lang === "TH" ? "Voucher" : "Voucher"}</th>
                        <th>{lang === "TH" ? "Channel" : "Method"}</th>
                        <th>{lang === "TH" ? "จำนวนเงิน" : "Amount"}</th>
                        <th>{lang === "TH" ? "สถานะ" : "Status"}</th>
                        <th>{lang === "TH" ? "ข้อความ" : "Message"}</th>
                        <th>{lang === "TH" ? "เวลา" : "Created"}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {transactions.length === 0 ? (
                        <tr className={styles.tableRow}>
                          <td colSpan={7} style={{ textAlign: "center", color: "var(--text-muted)" }}>
                            {loading ? "Loading..." : "No transactions"}
                          </td>
                        </tr>
                      ) : (
                        transactions.map((item) => (
                          <tr key={item.tx_id} className={styles.tableRow}>
                            <td>{item.owner_username}</td>
                            <td className={styles.mono}>{item.voucher_hash_masked}</td>
                            <td>{voucherMethodLabel(item.voucher_method)}</td>
                            <td className={styles.mono}>{item.amount_thb.toFixed(2)}</td>
                            <td>{item.status}</td>
                            <td>{item.message}</td>
                            <td>{formatDateTime(item.created_at)}</td>
                          </tr>
                        ))
                      )}
                    </tbody>
                  </table>
                </div>
              </div>
            )}
          </div>
        </main>
      </div>
    </div>
  );
}
