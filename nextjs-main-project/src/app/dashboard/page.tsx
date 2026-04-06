"use client";

import { useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { IBM_Plex_Mono, IBM_Plex_Sans, IBM_Plex_Sans_Thai } from "next/font/google";
import { clearAuthSession, getAuthSession, setAuthSession, type AuthSession } from "@/lib/authSession";
import { ApiRequestError, listDeviceSessions, logoutApi, refreshAccessToken, revokeDeviceSession, type ApiDeviceSession } from "@/lib/authClient";
import {
  getNotifications,
  getHostingServices,
  markNotificationsRead,
  renewHostingService,
  getTransactions,
  getWallet,
  orderHosting,
  redeemBankingSlip,
  redeemTrueMoneyVoucher,
  type HostingServiceItem,
  type NotificationItem,
  type TopupTransaction,
} from "@/lib/topupClient";
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

type View = "overview" | "services" | "hosting" | "topup_method" | "transaction_history" | "settings" | "sessions" | "checkout";

type DeviceSession = {
  id: string;
  device: string;
  icon: string;
  color: string;
  ip: string;
  location: string;
  lastActiveValue: string;
  isCurrent: boolean;
};

const localSessions: DeviceSession[] = [
  {
    id: "local-1",
    device: "Windows 11 / Chrome",
    icon: "desktop_windows",
    color: "var(--brand)",
    ip: "127.0.0.1",
    location: "Localhost",
    lastActiveValue: "now",
    isCurrent: true,
  },
  {
    id: "local-2",
    device: "iOS 17 / Safari",
    icon: "smartphone",
    color: "var(--text-muted)",
    ip: "10.0.xx.xx",
    location: "Bangkok, Thailand",
    lastActiveValue: "2 hours ago",
    isCurrent: false,
  },
];

function mapSessionForTable(session: ApiDeviceSession): DeviceSession {
  const isMobile = /ios|android|iphone|ipad|mobile/i.test(session.device);
  const isMac = /mac/i.test(session.device);

  return {
    id: session.id,
    device: session.device,
    icon: isMobile ? "smartphone" : isMac ? "desktop_mac" : "desktop_windows",
    color: session.is_current ? "var(--brand)" : "var(--text-muted)",
    ip: session.ip,
    location: session.location,
    lastActiveValue: session.last_active,
    isCurrent: session.is_current,
  };
}

function formatLastActive(value: string): string {
  const parsed = Date.parse(value);
  if (Number.isNaN(parsed)) return value;

  const diffSec = Math.floor((Date.now() - parsed) / 1000);
  if (diffSec < 60) return "now";
  if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m ago`;
  if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`;
  return `${Math.floor(diffSec / 86400)}d ago`;
}

const staticPackages = [
  { name: "Start", price: "10.00", disk: 200 },
  { name: "Lite", price: "19.00", disk: 500 },
  { name: "Core", price: "29.00", disk: 1000 },
  { name: "Plus", price: "39.00", disk: 2000 },
  { name: "Prime", price: "69.00", disk: 5000 },
  { name: "Pro", price: "89.00", disk: 10000 },
  { name: "Max", price: "149.00", disk: 15000 },
  { name: "Apex", price: "189.00", disk: 20000 },
];

const DIRECTADMIN_PANEL_URL = "https://dcadmin.reverz.in.th/";

type TxCache = {
  items: TopupTransaction[];
  cachedAt: string;
};

function getTxCacheKey(username: string) {
  return `reverz-tx-cache:${username}`;
}

function isAuthInvalidError(error: unknown): boolean {
  if (!(error instanceof ApiRequestError)) return false;
  if (error.status !== 401) return false;
  return error.errorCode === "UNAUTHENTICATED" || error.errorCode === "INVALID_REFRESH_TOKEN";
}

export default function DashboardPage() {
  const router = useRouter();

  const [theme, setTheme] = useState<"light" | "dark">("dark");
  const [lang, setLang] = useState<"EN" | "TH">("EN");
  const [isSidebarOpen, setIsSidebarOpen] = useState(true);
  const [currentView, setCurrentView] = useState<View>("overview");
  const [isTopupOpen, setIsTopupOpen] = useState(false);
  const [topupMethodTab, setTopupMethodTab] = useState<"banking" | "truemoney">("banking");
  const [accountBalance, setAccountBalance] = useState(0);
  const [receiverPhone, setReceiverPhone] = useState("0931959423");
  const [bankingReceiverId, setBankingReceiverId] = useState("xxx-x-x8407-x");
  const [bankingReceiverName, setBankingReceiverName] = useState("MASTER MATHAKAN TONGEAM");
  const [walletLoading, setWalletLoading] = useState(false);
  const [walletError, setWalletError] = useState<string | null>(null);
  const [topupTransactions, setTopupTransactions] = useState<TopupTransaction[]>([]);
  const [transactionsLoading, setTransactionsLoading] = useState(false);
  const [transactionsError, setTransactionsError] = useState<string | null>(null);
  const [txCacheInfo, setTxCacheInfo] = useState<{ cachedAt: string } | null>(null);
  const [slipFile, setSlipFile] = useState<File | null>(null);
  const [bankingLoading, setBankingLoading] = useState(false);
  const [bankingError, setBankingError] = useState<string | null>(null);
  const [bankingSuccess, setBankingSuccess] = useState<string | null>(null);
  const [slipInputKey, setSlipInputKey] = useState(0);
  const [voucherUrl, setVoucherUrl] = useState("");
  const [redeemLoading, setRedeemLoading] = useState(false);
  const [redeemError, setRedeemError] = useState<string | null>(null);
  const [redeemSuccess, setRedeemSuccess] = useState<string | null>(null);
  const [checkoutPackage, setCheckoutPackage] = useState<{ name: string, price: string, disk: number } | null>(null);
  const [checkoutEmail, setCheckoutEmail] = useState("");
  const [checkoutDomain, setCheckoutDomain] = useState("");
  const [orderLoading, setOrderLoading] = useState(false);
  const [orderError, setOrderError] = useState<string | null>(null);
  const [orderSuccess, setOrderSuccess] = useState<string | null>(null);
  const [orderResult, setOrderResult] = useState<{ da_username?: string; da_password?: string; da_panel_url?: string } | null>(null);
  const [hostingServices, setHostingServices] = useState<HostingServiceItem[]>([]);
  const [hostingServicesLoading, setHostingServicesLoading] = useState(false);
  const [hostingServicesError, setHostingServicesError] = useState<string | null>(null);
  const [renewingDomain, setRenewingDomain] = useState<string | null>(null);

  const [notifications, setNotifications] = useState<NotificationItem[]>([]);
  const [unreadCount, setUnreadCount] = useState(0);
  const [notificationsLoading, setNotificationsLoading] = useState(false);
  const [notificationsError, setNotificationsError] = useState<string | null>(null);
  const [isNotificationOpen, setIsNotificationOpen] = useState(false);
  const notificationWrapRef = useRef<HTMLDivElement | null>(null);

  const [activeSessions, setActiveSessions] = useState<DeviceSession[]>([]);
  const [revokingSessionId, setRevokingSessionId] = useState<string | null>(null);
  const [sessionActionError, setSessionActionError] = useState<string | null>(null);

  const refreshWallet = async () => {
    setWalletLoading(true);
    try {
      const wallet = await getWallet();
      setAccountBalance(wallet.balance_thb);
      setReceiverPhone(wallet.receiver_phone);
      setBankingReceiverId(wallet.banking_receiver_id);
      setBankingReceiverName(wallet.banking_receiver_name);
      setWalletError(null);
    } catch (error) {
      setWalletError(error instanceof Error ? error.message : "Unable to load wallet.");
    } finally {
      setWalletLoading(false);
    }
  };

  const refreshTransactions = async (username?: string) => {
    const sessionUsername = username ?? getAuthSession()?.username;
    setTransactionsLoading(true);
    try {
      const payload = await getTransactions(20);
      setTopupTransactions(payload.items);
      setTransactionsError(null);
      setTxCacheInfo(null);
      if (sessionUsername && typeof window !== "undefined") {
        const cacheValue: TxCache = {
          items: payload.items,
          cachedAt: new Date().toISOString(),
        };
        window.localStorage.setItem(getTxCacheKey(sessionUsername), JSON.stringify(cacheValue));
      }
    } catch (error) {
      const key = sessionUsername ? getTxCacheKey(sessionUsername) : null;
      let usedCache = false;
      if (key && typeof window !== "undefined") {
        const raw = window.localStorage.getItem(key);
        if (raw) {
          try {
            const parsed = JSON.parse(raw) as TxCache;
            if (Array.isArray(parsed.items)) {
              setTopupTransactions(parsed.items);
              setTxCacheInfo({ cachedAt: parsed.cachedAt });
              setTransactionsError("Showing cached history. Live refresh is temporarily unavailable.");
              usedCache = true;
            }
          } catch {
            window.localStorage.removeItem(key);
          }
        }
      }
      if (!usedCache) {
        setTransactionsError(error instanceof Error ? error.message : "Unable to load transactions.");
      }
    } finally {
      setTransactionsLoading(false);
    }
  };

  const refreshHostingServices = async () => {
    setHostingServicesLoading(true);
    try {
      const payload = await getHostingServices();
      setHostingServices(payload.items);
      setHostingServicesError(null);
    } catch (error) {
      setHostingServicesError(error instanceof Error ? error.message : "Unable to load hosting services.");
    } finally {
      setHostingServicesLoading(false);
    }
  };

  const refreshNotifications = async () => {
    setNotificationsLoading(true);
    try {
      const payload = await getNotifications(20);
      setNotifications(payload.items);
      setUnreadCount(payload.unread_count);
      setNotificationsError(null);
    } catch (error) {
      setNotificationsError(error instanceof Error ? error.message : "Unable to load notifications.");
    } finally {
      setNotificationsLoading(false);
    }
  };

  const markNotificationsAsReadAndRefresh = async () => {
    try {
      await markNotificationsRead();
      await refreshNotifications();
    } catch (error) {
      setNotificationsError(error instanceof Error ? error.message : "Unable to mark notifications as read.");
    }
  };

  useEffect(() => {
    const currentSession = getAuthSession();
    if (!currentSession) {
      router.replace("/login");
      return;
    }

    const isLocalBypassSession = currentSession.accessToken.startsWith("local-dev-");
    if (isLocalBypassSession) {
      setActiveSessions(localSessions);
    } else {
      void listDeviceSessions()
        .then((sessions) => {
          setActiveSessions(sessions.map(mapSessionForTable));
          setSessionActionError(null);
        })
        .catch(() => {
          setSessionActionError("Unable to load device sessions.");
        });
    }

    const storedLang = localStorage.getItem("lang");
    if (storedLang === "TH" || storedLang === "EN") setLang(storedLang);

    const storedTheme = document.documentElement.dataset.theme;
    if (storedTheme === "light" || storedTheme === "dark") setTheme(storedTheme);

    if (typeof window !== "undefined") {
      const raw = window.localStorage.getItem(getTxCacheKey(currentSession.username));
      if (raw) {
        try {
          const parsed = JSON.parse(raw) as TxCache;
          if (Array.isArray(parsed.items)) {
            setTopupTransactions(parsed.items);
            setTxCacheInfo({ cachedAt: parsed.cachedAt });
          }
        } catch {
          window.localStorage.removeItem(getTxCacheKey(currentSession.username));
        }
      }
    }

    void refreshWallet();
    void refreshTransactions(currentSession.username);
    void refreshHostingServices();
    void refreshNotifications();

    const refreshTimer = window.setInterval(async () => {
      const liveSession = getAuthSession();
      if (!liveSession) return;
      try {
        const nextAccessToken = await refreshAccessToken();
        const nextSession: AuthSession = {
          ...liveSession,
          accessToken: nextAccessToken,
        };
        setAuthSession(nextSession);
      } catch (error) {
        if (isAuthInvalidError(error)) {
          clearAuthSession();
          router.replace("/login");
        }
      }
    }, 5 * 60 * 1000);
    const notificationsTimer = window.setInterval(() => {
      void refreshNotifications();
    }, 60 * 1000);

    return () => {
      window.clearInterval(refreshTimer);
      window.clearInterval(notificationsTimer);
    };
  }, [router]);

  const handleSignOut = async () => {
    try {
      await logoutApi();
    } catch {
      // keep local logout
    } finally {
      clearAuthSession();
      router.replace("/login");
    }
  };

  const handleRevokeSession = async (id: string) => {
    const currentSession = getAuthSession();
    const isLocalBypassSession = currentSession?.accessToken.startsWith("local-dev-") ?? false;
    if (isLocalBypassSession) {
      setActiveSessions((prev) => prev.filter((session) => session.id !== id));
      return;
    }

    setSessionActionError(null);
    setRevokingSessionId(id);

    try {
      await revokeDeviceSession(id);
      setActiveSessions((prev) => prev.filter((session) => session.id !== id));
    } catch {
      setSessionActionError("Unable to revoke session. Please try again.");
    } finally {
      setRevokingSessionId(null);
    }
  };

  const toggleTheme = () => {
    const nextTheme = theme === "light" ? "dark" : "light";
    setTheme(nextTheme);
    document.documentElement.dataset.theme = nextTheme;
    localStorage.setItem("theme", nextTheme);
  };

  const notificationCount = unreadCount;
  const activeHostingCount = hostingServices.filter((service) => service.status === "active").length;
  const formattedBalance = accountBalance.toLocaleString("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });

  const fileToDataUrl = (file: File) =>
    new Promise<string>((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        if (typeof reader.result === "string") {
          resolve(reader.result);
          return;
        }
        reject(new Error("Unable to read file"));
      };
      reader.onerror = () => reject(new Error("Unable to read file"));
      reader.readAsDataURL(file);
    });

  const handleRedeemBankingSlip = async () => {
    if (!slipFile) {
      setBankingError("Please select a slip image.");
      setBankingSuccess(null);
      return;
    }

    if (!["image/jpeg", "image/jpg", "image/png"].includes(slipFile.type.toLowerCase())) {
      setBankingError("Only JPG/JPEG/PNG files are allowed.");
      setBankingSuccess(null);
      return;
    }

    setBankingLoading(true);
    setBankingError(null);
    setBankingSuccess(null);

    try {
      const img = await fileToDataUrl(slipFile);
      const result = await redeemBankingSlip(img);

      if (result.success) {
        setBankingSuccess(result.message);
        setSlipFile(null);
        setSlipInputKey((prev) => prev + 1);
        await Promise.all([refreshWallet(), refreshTransactions()]);
      } else {
        setBankingError(result.message);
      }
    } catch (error) {
      setBankingError(error instanceof Error ? error.message : "Unable to verify slip.");
    } finally {
      setBankingLoading(false);
    }
  };

  const handleRedeemTrueMoney = async () => {
    const trimmedUrl = voucherUrl.trim();
    if (!trimmedUrl) {
      setRedeemError("Please provide a TrueMoney voucher URL.");
      setRedeemSuccess(null);
      return;
    }

    const isValidVoucherUrl = /^https:\/\/gift\.truemoney\.com\/campaign\/\?v=[A-Za-z0-9]+$/.test(trimmedUrl);
    if (!isValidVoucherUrl) {
      setRedeemError("Invalid voucher URL format.");
      setRedeemSuccess(null);
      return;
    }

    setRedeemLoading(true);
    setRedeemError(null);
    setRedeemSuccess(null);

    try {
      const result = await redeemTrueMoneyVoucher(trimmedUrl);
      if (result.success) {
        setRedeemSuccess(result.message);
        setVoucherUrl("");
        await Promise.all([refreshWallet(), refreshTransactions()]);
      } else {
        setRedeemError(result.message);
      }
    } catch (error) {
      setRedeemError(error instanceof Error ? error.message : "Unable to redeem voucher.");
    } finally {
      setRedeemLoading(false);
    }
  };

  useEffect(() => {
    const onClickOutside = (event: MouseEvent) => {
      if (!notificationWrapRef.current) return;
      if (!notificationWrapRef.current.contains(event.target as Node)) {
        setIsNotificationOpen(false);
      }
    };
    document.addEventListener("mousedown", onClickOutside);
    return () => {
      document.removeEventListener("mousedown", onClickOutside);
    };
  }, []);

  const formatDateTime = (raw: string) => {
    const parsed = new Date(raw);
    if (Number.isNaN(parsed.getTime())) return raw || "-";
    return parsed.toLocaleString("th-TH", { timeZone: "Asia/Bangkok" });
  };

  const canRenewService = (service: HostingServiceItem) => {
    if (service.status === "active") return true;
    if (service.status !== "grace_suspended") return false;
    const graceAt = Date.parse(service.grace_until);
    if (Number.isNaN(graceAt)) return false;
    return Date.now() <= graceAt;
  };

  const statusBadgeClass = (status: string) => {
    if (status === "active") return `${styles.statusBadge} ${styles.statusSuccess}`;
    if (status === "grace_suspended") return `${styles.statusBadge} ${styles.statusFailed}`;
    return `${styles.statusBadge} ${styles.statusRunning}`;
  };

  const statusLabel = (status: string) => {
    if (status === "active") return lang === "TH" ? "ใช้งาน" : "Active";
    if (status === "grace_suspended") return lang === "TH" ? "พักชั่วคราว (รอต่ออายุ)" : "Grace Suspended";
    if (status === "suspended_expired") return lang === "TH" ? "หมดอายุ" : "Expired";
    return status;
  };

  const handleRenew = async (domain: string) => {
    setRenewingDomain(domain);
    setOrderError(null);
    setOrderSuccess(null);
    try {
      const result = await renewHostingService(domain);
      setOrderSuccess(result.message);
      await Promise.all([refreshWallet(), refreshTransactions(), refreshHostingServices(), refreshNotifications()]);
    } catch (error) {
      if (isAuthInvalidError(error)) {
        clearAuthSession();
        router.push("/login");
        return;
      }
      setOrderError(error instanceof Error ? error.message : "Unable to renew service.");
    } finally {
      setRenewingDomain(null);
    }
  };

  return (
    <div className={`${styles.carbonRoot} ${ibmPlexSans.variable} ${ibmPlexSansThai.variable} ${ibmPlexMono.variable}`}>
      <header className={styles.header}>
        <div className={styles.headerLeft}>
          <button className={styles.headerBtn} onClick={() => setIsSidebarOpen(!isSidebarOpen)}>
            <span className={styles.icon}>menu</span>
          </button>
          <div className={styles.brand}>
            <span className={styles.brandText}>Reverz-hosting</span>
            <span className={styles.brandSpan}>shared-hosting</span>
          </div>
        </div>
        <div className={styles.headerRight}>
          <button className={styles.headerIcon} onClick={toggleTheme} title="Toggle Theme">
            <span className={styles.icon}>{theme === "light" ? "dark_mode" : "light_mode"}</span>
          </button>
          <div className={styles.notificationWrap} ref={notificationWrapRef}>
            <button
              className={styles.headerIcon}
              title="Notifications"
              onClick={() => {
                const willOpen = !isNotificationOpen;
                setIsNotificationOpen(willOpen);
                if (willOpen) {
                  void markNotificationsAsReadAndRefresh();
                }
              }}
            >
              <span className={styles.icon}>notifications</span>
              {notificationCount > 0 ? <span className={styles.headerBadge}>{notificationCount}</span> : null}
            </button>
            {isNotificationOpen ? (
              <div className={styles.notificationDropdown}>
                <div className={styles.notificationHeader}>
                  <span>{lang === "TH" ? "แจ้งเตือน" : "Notifications"}</span>
                </div>
                {notificationsError ? (
                  <p className={styles.errorText} style={{ padding: "10px 12px", margin: 0 }}>{notificationsError}</p>
                ) : notificationsLoading ? (
                  <p className={styles.notificationEmpty}>{lang === "TH" ? "กำลังโหลด..." : "Loading..."}</p>
                ) : notifications.length === 0 ? (
                  <p className={styles.notificationEmpty}>{lang === "TH" ? "ไม่มีแจ้งเตือน" : "No notifications"}</p>
                ) : (
                  <div className={styles.notificationList}>
                    {notifications.map((item) => (
                      <div key={item.id} className={styles.notificationItem}>
                        <p className={styles.notificationItemTitle}>{item.title}</p>
                        <p className={styles.notificationItemMessage}>{item.message}</p>
                        <p className={`${styles.notificationItemTime} ${styles.mono}`}>{formatDateTime(item.created_at)}</p>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ) : null}
          </div>
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
          <div className={styles.accountCount} title="Account balance">
            <span>THB {walletLoading ? "..." : formattedBalance}</span>
          </div>
          <button
            className={styles.headerIcon}
            onClick={() => {
              void handleSignOut();
            }}
            title="Sign out"
          >
            <span className={styles.icon}>logout</span>
          </button>
        </div>
      </header>

      <div className={styles.mainWrapper}>
        <aside className={`${styles.sidebar} ${isSidebarOpen ? "" : styles.sidebarCollapsed}`}>
          <div className={styles.navGroup}>
            <button className={`${styles.navItem} ${currentView === "overview" ? styles.navItemActive : ""}`} onClick={() => setCurrentView("overview")}>
              <span className={`${styles.icon} ${styles.navItemIcon}`}>dashboard</span>
              <span className={styles.navItemText}>{lang === "TH" ? "à¸«à¸™à¹‰à¸²à¸«à¸¥à¸±à¸" : "Dashboard"}</span>
            </button>
            <button className={`${styles.navItem} ${currentView === "services" ? styles.navItemActive : ""}`} onClick={() => setCurrentView("services")}>
              <span className={`${styles.icon} ${styles.navItemIcon}`}>dns</span>
              <span className={styles.navItemText}>{lang === "TH" ? "à¸šà¸£à¸´à¸à¸²à¸£" : "Service"}</span>
            </button>
            <button className={`${styles.navItem} ${currentView === "hosting" ? styles.navItemActive : ""}`} onClick={() => setCurrentView("hosting")}>
              <span className={`${styles.icon} ${styles.navItemIcon}`}>cloud</span>
              <span className={styles.navItemText}>{lang === "TH" ? "à¹‚à¸®à¸ªà¸•à¸´à¹‰à¸‡" : "Hosting"}</span>
            </button>

            <div style={{ display: 'flex', flexDirection: 'column' }}>
              <button
                className={`${styles.navItem} ${currentView === 'topup_method' || currentView === 'transaction_history' ? styles.navItemActive : ""}`}
                onClick={() => setIsTopupOpen(!isTopupOpen)}
              >
                <div style={{ display: 'flex', alignItems: 'center' }}>
                  <span className={`${styles.icon} ${styles.navItemIcon}`}>account_balance_wallet</span>
                  <span className={styles.navItemText}>{lang === "TH" ? "à¹€à¸•à¸´à¸¡à¹€à¸‡à¸´à¸™" : "Topup"}</span>
                </div>
                <span className={`${styles.icon} ${styles.navChevron} ${isTopupOpen ? styles.navChevronOpen : ""}`}>keyboard_arrow_down</span>
              </button>
              {isTopupOpen && (
                <div className={styles.navSubGroup}>
                  <button className={`${styles.navSubItem} ${currentView === "topup_method" ? styles.navSubItemActive : ""}`} onClick={() => setCurrentView("topup_method")}>
                    <span className={styles.navItemText}>{lang === "TH" ? "à¸Šà¹ˆà¸­à¸‡à¸—à¸²à¸‡à¹€à¸•à¸´à¸¡à¹€à¸‡à¸´à¸™" : "Topup method"}</span>
                  </button>
                  <button className={`${styles.navSubItem} ${currentView === "transaction_history" ? styles.navSubItemActive : ""}`} onClick={() => setCurrentView("transaction_history")}>
                    <span className={styles.navItemText}>{lang === "TH" ? "à¸›à¸£à¸°à¸§à¸±à¸•à¸´à¸à¸²à¸£à¸—à¸³à¸£à¸²à¸¢à¸à¸²à¸£" : "Transaction History"}</span>
                  </button>
                </div>
              )}
            </div>

            <div className={styles.divider}></div>
            <span className={styles.navLabel}>{lang === "TH" ? "à¸à¸²à¸£à¸ˆà¸±à¸”à¸à¸²à¸£à¸šà¸±à¸à¸Šà¸µ" : "Account Management"}</span>

            <button className={`${styles.navItem} ${currentView === "settings" ? styles.navItemActive : ""}`} onClick={() => setCurrentView("settings")}>
              <span className={`${styles.icon} ${styles.navItemIcon}`}>settings</span>
              <span className={styles.navItemText}>{lang === "TH" ? "à¸à¸²à¸£à¸•à¸±à¹‰à¸‡à¸„à¹ˆà¸²" : "Settings"}</span>
            </button>
            <button className={`${styles.navItem} ${currentView === "sessions" ? styles.navItemActive : ""}`} onClick={() => setCurrentView("sessions")}>
              <span className={`${styles.icon} ${styles.navItemIcon}`}>security</span>
              <span className={styles.navItemText}>{lang === "TH" ? "à¸­à¸¸à¸›à¸à¸£à¸“à¹Œà¸—à¸µà¹ˆà¹€à¸‚à¹‰à¸²à¸ªà¸¹à¹ˆà¸£à¸°à¸šà¸š" : "Device Sessions"}</span>
            </button>
          </div>
        </aside>

        <main className={styles.content}>
          <div className={styles.container}>
            {currentView === "overview" && (
              <>
                <div className={styles.pageHeader}>
                  <h1 className={styles.pageTitle}>{lang === "TH" ? "à¸ªà¸–à¸²à¸™à¸°à¸£à¸°à¸šà¸š" : "System Status"}</h1>
                </div>
                <div className={styles.grid}>
                  <div className={styles.tile} style={{ height: "auto" }}>
                    <div className={styles.tileHeader}>
                      <h3 className={styles.tileTitle}>{lang === "TH" ? "à¸šà¸£à¸´à¸à¸²à¸£à¸—à¸µà¹ˆà¸à¸³à¸¥à¸±à¸‡à¸—à¸³à¸‡à¸²à¸™" : "Running Services"}</h3>
                      <span className={`${styles.icon} ${styles.tileIcon}`}>dns</span>
                    </div>
                    <div className={styles.tileValueRow}>
                      <span className={`${styles.tilePrimaryVal} ${styles.mono}`}>
                        {hostingServicesLoading ? "..." : activeHostingCount}
                      </span>
                    </div>
                  </div>
                  <div className={styles.tile} style={{ height: "auto" }}>
                    <div className={styles.tileHeader}>
                      <h3 className={styles.tileTitle}>{lang === "TH" ? "à¹‚à¸›à¸£à¹€à¸ˆà¹‡à¸à¸•à¹Œà¸—à¸µà¹ˆà¹ƒà¸Šà¹‰à¸‡à¸²à¸™à¸­à¸¢à¸¹à¹ˆ" : "Active Projects"}</h3>
                      <span className={`${styles.icon} ${styles.tileIcon}`}>view_module</span>
                    </div>
                    <div className={styles.tileValueRow}>
                      <span className={`${styles.tilePrimaryVal} ${styles.mono}`}>
                        {hostingServicesLoading ? "..." : activeHostingCount}
                      </span>
                    </div>
                  </div>
                  <div className={styles.tile} style={{ height: "auto" }}>
                    <div className={styles.tileHeader}>
                      <h3 className={styles.tileTitle}>{lang === "TH" ? "à¸­à¸¸à¸›à¸à¸£à¸“à¹Œà¸—à¸µà¹ˆà¹€à¸‚à¹‰à¸²à¸ªà¸¹à¹ˆà¸£à¸°à¸šà¸š" : "Device Sessions"}</h3>
                      <span className={`${styles.icon} ${styles.tileIcon}`}>security</span>
                    </div>
                    <div className={styles.tileValueRow}>
                      <span className={`${styles.tilePrimaryVal} ${styles.mono}`}>{activeSessions.length}</span>
                    </div>
                  </div>
                </div>
              </>
            )}

                        {currentView === "services" && (
              <>
                {hostingServicesError ? <p className={styles.errorText}>{hostingServicesError}</p> : null}
                {hostingServicesLoading ? (
                  <div className={styles.emptyStateTile}>
                    <span className={styles.icon}>hourglass_top</span>
                    <p>{lang === "TH" ? "à¸à¸³à¸¥à¸±à¸‡à¹‚à¸«à¸¥à¸”à¸šà¸£à¸´à¸à¸²à¸£..." : "Loading services..."}</p>
                  </div>
                ) : hostingServices.length === 0 ? (
                  <div className={styles.emptyStateTile}>
                    <span className={styles.icon}>folder_open</span>
                    <p>{lang === "TH" ? "à¹„à¸¡à¹ˆà¸¡à¸µà¸šà¸£à¸´à¸à¸²à¸£à¸—à¸µà¹ˆà¸à¸³à¸¥à¸±à¸‡à¸—à¸³à¸‡à¸²à¸™" : "No Active Services"}</p>
                  </div>
                ) : (
                  <div className={styles.colSpan3}>
                    <p
                      style={{
                        marginBottom: "10px",
                        color: "var(--text-muted)",
                        fontSize: "0.875rem",
                      }}
                    >
                      {lang === "TH" ? "DirectAdmin Panel (Domain):" : "DirectAdmin Panel (Domain):"}{" "}
                      <a
                        href={DIRECTADMIN_PANEL_URL}
                        target="_blank"
                        rel="noreferrer"
                        className={styles.mono}
                        style={{ color: "var(--brand)" }}
                      >
                        {DIRECTADMIN_PANEL_URL}
                      </a>
                    </p>
                    <div className={styles.tableWrap}>
                      <table className={styles.table}>
                        <thead>
                          <tr>
                            <th>{lang === "TH" ? "โดเมน" : "Domain"}</th>
                            <th>{lang === "TH" ? "แพ็กเกจ" : "Package"}</th>
                            <th>{lang === "TH" ? "สถานะ" : "Status"}</th>
                            <th>{lang === "TH" ? "หมดอายุ" : "Expires At"}</th>
                            <th>{lang === "TH" ? "หมดช่วงต่ออายุ" : "Grace Until"}</th>
                            <th>{lang === "TH" ? "ผู้ใช้ DirectAdmin" : "DA Username"}</th>
                            <th>{lang === "TH" ? "รหัสผ่าน DirectAdmin" : "DA Password"}</th>
                            <th>{lang === "TH" ? "ลิงก์เข้า Panel" : "Panel URL"}</th>
                            <th>{lang === "TH" ? "สร้างเมื่อ" : "Created At"}</th>
                            <th>{lang === "TH" ? "ต่ออายุ" : "Renew"}</th>
                          </tr>
                        </thead>
                        <tbody>
                          {hostingServices.map((service) => {
                            const canRenew = canRenewService(service);
                            return (
                              <tr key={`${service.domain}:${service.created_at}`} className={styles.tableRow}>
                                <td className={styles.mono}>{service.domain}</td>
                                <td>{service.package_name}</td>
                                <td>
                                  <span className={statusBadgeClass(service.status)}>{statusLabel(service.status)}</span>
                                </td>
                                <td>{formatDateTime(service.expires_at)}</td>
                                <td>{formatDateTime(service.grace_until)}</td>
                                <td className={styles.mono}>{service.da_username?.trim() ? service.da_username : "-"}</td>
                                <td className={styles.mono}>{service.da_password?.trim() ? service.da_password : "-"}</td>
                                <td className={styles.mono}>
                                  <a
                                    href={service.da_panel_url?.trim() || DIRECTADMIN_PANEL_URL}
                                    target="_blank"
                                    rel="noreferrer"
                                    style={{ color: "var(--brand)" }}
                                  >
                                    {service.da_panel_url?.trim() || DIRECTADMIN_PANEL_URL}
                                  </a>
                                </td>
                                <td>{formatDateTime(service.created_at)}</td>
                                <td>
                                  <button
                                    className={styles.renewButton}
                                    disabled={!canRenew || renewingDomain === service.domain}
                                    onClick={() => {
                                      void handleRenew(service.domain);
                                    }}
                                  >
                                    {!canRenew
                                      ? (lang === "TH" ? "หมดสิทธิ์" : "Unavailable")
                                      : renewingDomain === service.domain
                                        ? (lang === "TH" ? "กำลังต่อ..." : "Renewing...")
                                        : (lang === "TH" ? "ต่ออายุ 1 เดือน" : "Renew 1 Month")}
                                  </button>
                                </td>
                              </tr>
                            );
                          })}
                        </tbody>
                      </table>
                    </div>
                  </div>
                )}
              </>
            )}

            {currentView === "topup_method" && (
              <>
                <div className={styles.pageHeader}>
                  <h1 className={styles.pageTitle}>{lang === "TH" ? "à¸Šà¹ˆà¸­à¸‡à¸—à¸²à¸‡à¹€à¸•à¸´à¸¡à¹€à¸‡à¸´à¸™" : "Topup Method"}</h1>
                </div>
                <div className={styles.topupTabContainer}>
                  <button
                    className={`${styles.topupTab} ${topupMethodTab === "banking" ? styles.topupTabActive : ""}`}
                    onClick={() => setTopupMethodTab("banking")}
                  >
                    <span className={`${styles.icon} ${styles.topupTabIcon}`}>account_balance</span> Banking
                  </button>
                  <button
                    className={`${styles.topupTab} ${topupMethodTab === "truemoney" ? styles.topupTabActive : ""}`}
                    onClick={() => setTopupMethodTab("truemoney")}
                  >
                    <span className={`${styles.icon} ${styles.topupTabIcon}`}>account_balance_wallet</span> TrueMoney Wallet
                  </button>
                </div>

                <div className={styles.topupGrid}>
                  {topupMethodTab === "banking" && (
                    <div className={styles.topupCard} style={{ gridColumn: "1 / -1", maxWidth: "600px", margin: "0 auto", width: "100%" }}>
                      <div className={styles.topupCardHeader}>
                        <h2 className={styles.topupCardTitle}>
                          <span className={styles.icon}>account_balance</span> Banking
                        </h2>
                      </div>

                      <div className={styles.topupDetailRow}>
                        <span className={styles.topupDetailLabel}>{lang === "TH" ? "à¸˜à¸™à¸²à¸„à¸²à¸£:" : "Bank:"}</span>
                        <span className={styles.topupDetailValue}>KBank</span>
                      </div>
                      <div className={styles.topupDetailRow}>
                        <span className={styles.topupDetailLabel}>{lang === "TH" ? "à¹€à¸¥à¸‚à¸šà¸±à¸à¸Šà¸µ:" : "Account:"}</span>
                        <span className={`${styles.topupDetailValue} ${styles.mono}`}>{bankingReceiverId}</span>
                      </div>
                      <div className={styles.topupDetailRow}>
                        <span className={styles.topupDetailLabel}>{lang === "TH" ? "à¸Šà¸·à¹ˆà¸­à¸šà¸±à¸à¸Šà¸µ:" : "Name:"}</span>
                        <span className={styles.topupDetailValue}>{bankingReceiverName}</span>
                      </div>

                      <input
                        key={slipInputKey}
                        type="file"
                        className={styles.topupFileInput}
                        accept="image/png, image/jpeg, image/jpg"
                        disabled={bankingLoading}
                        onChange={(event) => setSlipFile(event.target.files?.[0] ?? null)}
                      />

                      {bankingError ? <p className={styles.errorText}>{bankingError}</p> : null}
                      {bankingSuccess ? <p className={styles.successText}>{bankingSuccess}</p> : null}
                      {walletError ? <p className={styles.errorText}>{walletError}</p> : null}

                      <div className={styles.topupGuidelines}>
                        <h4 className={styles.topupGuidelinesTitle}>{lang === "TH" ? "à¸•à¸±à¸§à¸­à¸¢à¹ˆà¸²à¸‡à¸ªà¸¥à¸´à¸›à¸—à¸µà¹ˆà¹à¸™à¸°à¸™à¸³" : "Slip Guidelines"}</h4>
                        <div className={styles.topupGuidelineItem}>
                          <span className={styles.icon} style={{ fontSize: '16px', color: 'var(--success)' }}>check_circle</span>
                          <span>{lang === "TH" ? "1. à¸Šà¸·à¹ˆà¸­à¸œà¸¹à¹‰à¸£à¸±à¸šà¸•à¹‰à¸­à¸‡à¹€à¸›à¹‡à¸™à¸šà¸±à¸à¸Šà¸µà¸—à¸µà¹ˆà¸•à¸±à¹‰à¸‡à¹„à¸§à¹‰à¹ƒà¸™à¸£à¸°à¸šà¸š" : "1. The recipient name must match."}</span>
                        </div>
                        <div className={styles.topupGuidelineItem}>
                          <span className={styles.icon} style={{ fontSize: '16px', color: 'var(--success)' }}>check_circle</span>
                          <span>{lang === "TH" ? "2. à¸ˆà¸³à¸™à¸§à¸™à¹€à¸‡à¸´à¸™à¹à¸¥à¸°à¸§à¸±à¸™à¹€à¸§à¸¥à¸²à¸•à¹‰à¸­à¸‡à¸­à¹ˆà¸²à¸™à¹„à¸”à¹‰à¸Šà¸±à¸”à¹€à¸ˆà¸™" : "2. Amount and datetime must be legible."}</span>
                        </div>
                        <div className={styles.topupGuidelineItem}>
                          <span className={styles.icon} style={{ fontSize: '16px', color: 'var(--success)' }}>check_circle</span>
                          <span>{lang === "TH" ? "3. à¹„à¸¡à¹ˆà¸„à¸£à¸­à¸›à¸ˆà¸™à¸•à¸±à¸”à¸‚à¹‰à¸­à¸¡à¸¹à¸¥à¸ªà¸³à¸„à¸±à¸" : "3. Do not crop important details."}</span>
                        </div>
                      </div>

                      <button
                        className={styles.btnPrimary}
                        style={{ width: '100%', marginTop: 'auto' }}
                        disabled={bankingLoading}
                        onClick={() => {
                          void handleRedeemBankingSlip();
                        }}
                      >
                        {bankingLoading
                          ? (lang === "TH" ? "à¸à¸³à¸¥à¸±à¸‡à¸•à¸£à¸§à¸ˆà¸ªà¸­à¸š..." : "Verifying...")
                          : (lang === "TH" ? "à¸•à¸£à¸§à¸ˆà¸ªà¸­à¸šà¸ªà¸¥à¸´à¸›à¹à¸¥à¸°à¹€à¸•à¸´à¸¡à¹€à¸‡à¸´à¸™" : "Verify Slip and Topup")}
                      </button>
                    </div>
                  )}

                  {topupMethodTab === "truemoney" && (
                    <div className={styles.topupCard} style={{ gridColumn: "1 / -1", maxWidth: "600px", margin: "0 auto", width: "100%" }}>
                      <div className={styles.topupCardHeader}>
                        <h2 className={styles.topupCardTitle}>
                          <span className={styles.icon}>account_balance_wallet</span> TrueMoney Wallet
                        </h2>
                      </div>

                      <div className={styles.topupDetailRow}>
                        <span className={styles.topupDetailLabel}>{lang === "TH" ? "à¸ªà¸–à¸²à¸™à¸°à¸£à¸°à¸šà¸š" : "System Status"}</span>
                        <div>
                          <span className={styles.topupStatusBadge}>
                            <span className={styles.pulseDot} style={{ position: 'relative', width: '8px', height: '8px' }}></span>
                            {lang === "TH" ? "à¸žà¸£à¹‰à¸­à¸¡à¹ƒà¸Šà¹‰à¸‡à¸²à¸™" : "Available"}
                          </span>
                        </div>
                      </div>
                      <div className={styles.topupDetailRow}>
                        <span className={styles.topupDetailLabel}>{lang === "TH" ? "à¹€à¸šà¸­à¸£à¹Œà¸£à¸±à¸šà¸‹à¸­à¸‡" : "Gift Voucher No."}</span>
                        <span className={`${styles.topupDetailValue} ${styles.mono}`}>{receiverPhone}</span>
                      </div>

                      <div className={styles.topupInputWrapper}>
                        <input
                          type="url"
                          className={styles.input}
                          style={{ width: '100%' }}
                          placeholder="https://gift.truemoney.com/campaign/?v=..."
                          value={voucherUrl}
                          onChange={(event) => setVoucherUrl(event.target.value)}
                          disabled={redeemLoading}
                        />
                      </div>

                      {redeemError ? <p className={styles.errorText}>{redeemError}</p> : null}
                      {redeemSuccess ? <p className={styles.successText}>{redeemSuccess}</p> : null}
                      {walletError ? <p className={styles.errorText}>{walletError}</p> : null}

                      <button
                        className={styles.btnPrimary}
                        style={{ width: '100%', marginTop: 'auto' }}
                        onClick={() => {
                          void handleRedeemTrueMoney();
                        }}
                        disabled={redeemLoading}
                      >
                        {redeemLoading
                          ? (lang === "TH" ? "à¸à¸³à¸¥à¸±à¸‡à¸•à¸£à¸§à¸ˆà¸ªà¸­à¸š..." : "Redeeming...")
                          : (lang === "TH" ? "à¹€à¸•à¸´à¸¡à¹€à¸‡à¸´à¸™à¸”à¹‰à¸§à¸¢ TrueMoney" : "Topup via TrueMoney")}
                      </button>
                    </div>
                  )}
                </div>
              </>
            )}

            {currentView === "transaction_history" && (
              <div className={styles.colSpan3}>
                <div className={styles.tableWrap}>
                  {txCacheInfo ? (
                    <p
                      style={{
                        margin: "12px 16px 0 16px",
                        color: "var(--warning)",
                        fontSize: "0.8rem",
                      }}
                    >
                      Showing cached history ({new Date(txCacheInfo.cachedAt).toLocaleString("th-TH", { timeZone: "Asia/Bangkok" })})
                    </p>
                  ) : null}
                  {transactionsError ? <p className={styles.errorText} style={{ margin: "12px 16px" }}>{transactionsError}</p> : null}
                  <table className={styles.table}>
                    <thead>
                      <tr>
                        <th>{lang === "TH" ? "à¹€à¸§à¸¥à¸²" : "Created At"}</th>
                        <th>{lang === "TH" ? "à¸ˆà¸³à¸™à¸§à¸™à¹€à¸‡à¸´à¸™ (THB)" : "Amount (THB)"}</th>
                        <th>{lang === "TH" ? "à¸ªà¸–à¸²à¸™à¸°" : "Status"}</th>
                        <th>{lang === "TH" ? "à¹€à¸¥à¸‚à¸­à¹‰à¸²à¸‡à¸­à¸´à¸‡" : "Reference"}</th>
                        <th>{lang === "TH" ? "à¸‚à¹‰à¸­à¸„à¸§à¸²à¸¡" : "Message"}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {transactionsLoading ? (
                        <tr className={styles.tableRow}>
                          <td colSpan={5} style={{ textAlign: "center", color: "var(--text-muted)" }}>
                            {lang === "TH" ? "à¸à¸³à¸¥à¸±à¸‡à¹‚à¸«à¸¥à¸”..." : "Loading..."}
                          </td>
                        </tr>
                      ) : topupTransactions.length === 0 ? (
                        <tr className={styles.tableRow}>
                          <td colSpan={5} style={{ textAlign: "center", color: "var(--text-muted)" }}>
                            {lang === "TH" ? "à¸¢à¸±à¸‡à¹„à¸¡à¹ˆà¸¡à¸µà¸£à¸²à¸¢à¸à¸²à¸£à¹€à¸•à¸´à¸¡à¹€à¸‡à¸´à¸™" : "No topup transactions yet."}
                          </td>
                        </tr>
                      ) : (
                        topupTransactions.map((item) => (
                          <tr key={item.tx_id} className={styles.tableRow}>
                            <td>{new Date(item.created_at).toLocaleString("th-TH", { timeZone: "Asia/Bangkok" })}</td>
                            <td className={styles.mono}>{item.amount_thb.toFixed(2)}</td>
                            <td>{item.status}</td>
                            <td className={styles.mono}>{item.voucher_hash}</td>
                            <td>{item.message}</td>
                          </tr>
                        ))
                      )}
                    </tbody>
                  </table>
                </div>
              </div>
            )}

            {currentView === "hosting" && (
              <>
                <div className={styles.pageHeader}>
                  <h1 className={styles.pageTitle}>{lang === "TH" ? "à¹à¸„à¸•à¸•à¸²à¸¥à¹‡à¸­à¸à¹‚à¸®à¸ªà¸•à¸´à¹‰à¸‡" : "Hosting Catalog"}</h1>
                </div>
                <div className={styles.pricingGrid}>
                  {staticPackages.map((pkg) => (
                    <div key={pkg.name} className={styles.pricingCard}>
                      <div className={styles.pricingHeader}>
                        <h4 className={styles.pricingTitle}>{pkg.name}</h4>
                        <p className={styles.pricingSubtitle}>DirectAdmin TH</p>
                      </div>
                      <ul className={styles.pricingFeatures}>
                        <li className={styles.pricingFeatureItem}>
                          <span className={`${styles.icon} ${styles.pricingFeatureIcon}`}>check</span>
                          {lang === "TH" ? "à¸žà¸·à¹‰à¸™à¸—à¸µà¹ˆà¸ˆà¸±à¸”à¹€à¸à¹‡à¸š: " : "Disk Space: "} <strong style={{ marginLeft: "4px" }}>{pkg.disk} MB</strong>
                        </li>
                        <li className={styles.pricingFeatureItem}>
                          <span className={`${styles.icon} ${styles.pricingFeatureIcon}`}>check</span>
                          {lang === "TH" ? "à¹à¸šà¸™à¸”à¹Œà¸§à¸´à¸”à¸—à¹Œ: " : "Bandwidth: "} <strong style={{ marginLeft: "4px" }}>unlimited MB</strong>
                        </li>
                        <li className={styles.pricingFeatureItem}>
                          <span className={`${styles.icon} ${styles.pricingFeatureIcon}`}>check</span>
                          {lang === "TH" ? "à¹‚à¸”à¹€à¸¡à¸™: " : "Domains: "} <strong style={{ marginLeft: "4px" }}>unlimited</strong>
                        </li>
                        <li className={styles.pricingFeatureItem}>
                          <span className={`${styles.icon} ${styles.pricingFeatureIcon}`}>check</span>
                          {lang === "TH" ? "à¸à¸²à¸™à¸‚à¹‰à¸­à¸¡à¸¹à¸¥: " : "Databases: "} <strong style={{ marginLeft: "4px" }}>unlimited</strong>
                        </li>
                      </ul>
                      <div className={styles.pricingPriceWrap}>
                        <span className={styles.pricingCurrency}>à¸¿</span>
                        <span className={`${styles.pricingPrice} ${styles.mono}`}>{pkg.price}</span>
                        <span className={styles.pricingPeriod}>/monthly</span>
                      </div>
                      <button
                        className={styles.btnPrimary}
                        style={{ width: "100%" }}
                        onClick={() => {
                          setCheckoutPackage(pkg);
                          setCheckoutEmail("");
                          setCheckoutDomain("");
                          setOrderError(null);
                          setOrderSuccess(null);
                          setOrderResult(null);
                          setCurrentView("checkout");
                        }}
                      >
                        {lang === "TH" ? "à¸ªà¸±à¹ˆà¸‡à¸‹à¸·à¹‰à¸­à¹€à¸¥à¸¢" : "Order Now"}
                      </button>
                    </div>
                  ))}
                </div>
              </>
            )}

            {currentView === "checkout" && checkoutPackage && (
              <>
                <div className={styles.pageHeader}>
                  <h1 className={styles.pageTitle}>{lang === "TH" ? "à¸•à¸£à¸§à¸ˆà¸ªà¸­à¸šà¹à¸¥à¸°à¸—à¸³à¸£à¸²à¸¢à¸à¸²à¸£" : "Review & Checkout"}</h1>
                  <p className={styles.pageSubtitle} style={{ marginTop: '8px', color: 'var(--text-muted)' }}>
                    {lang === "TH" ? "à¸à¸£à¸­à¸à¸£à¸²à¸¢à¸¥à¸°à¹€à¸­à¸µà¸¢à¸”à¹à¸¥à¸°à¸¢à¸·à¸™à¸¢à¸±à¸™à¸à¸²à¸£à¸•à¸±à¹‰à¸‡à¸„à¹ˆà¸²à¹à¸žà¹‡à¸à¹€à¸à¸ˆà¹‚à¸®à¸ªà¸•à¸´à¹‰à¸‡" : "Complete your hosting order configuration."}
                  </p>
                </div>

                <div className={styles.checkoutLayout}>
                  <div className={styles.checkoutMain}>
                    <div className={styles.checkoutCard}>
                      <h3 className={styles.checkoutSectionTitle}>{lang === "TH" ? "à¸à¸²à¸£à¸•à¸±à¹‰à¸‡à¸„à¹ˆà¸²à¹‚à¸”à¹€à¸¡à¸™" : "Domain Configuration"}</h3>
                      <p className={styles.checkoutSectionSubtitle}>{lang === "TH" ? "à¸›à¹‰à¸­à¸™à¸Šà¸·à¹ˆà¸­à¸­à¹‚à¸”à¹€à¸¡à¸™à¸—à¸µà¹ˆà¸„à¸¸à¸“à¸•à¹‰à¸­à¸‡à¸à¸²à¸£à¹ƒà¸Šà¹‰à¸‡à¸²à¸™à¸à¸±à¸šà¹à¸žà¹‡à¸à¹€à¸à¸ˆà¸™à¸µà¹‰" : "Enter the domain name you want to use with this hosting package."}</p>

                      <div className={styles.checkoutInputGroupRow} style={{ marginTop: "8px" }}>
                        <input
                          className={styles.input}
                          placeholder="example.com"
                          value={checkoutDomain}
                          onChange={(e) => setCheckoutDomain(e.target.value)}
                        />
                      </div>

                      <div className={styles.checkoutInputGroup} style={{ marginTop: "16px" }}>
                        <label className={styles.label}>{lang === "TH" ? "à¸­à¸µà¹€à¸¡à¸¥à¸œà¸¹à¹‰à¸”à¸¹à¹à¸¥à¸£à¸°à¸šà¸š" : "Admin Email"}</label>
                        <input
                          className={styles.input}
                          value={checkoutEmail}
                          onChange={(e) => setCheckoutEmail(e.target.value)}
                          placeholder={lang === "TH" ? "à¸à¸£à¸­à¸à¸­à¸µà¹€à¸¡à¸¥à¸‚à¸­à¸‡à¸„à¸¸à¸“ à¹€à¸Šà¹ˆà¸™ root@gmail.com" : "Enter your email, e.g. root@gmail.com"}
                        />
                        <p className={styles.checkoutSectionSubtitle} style={{ marginTop: "4px" }}>
                          {lang === "TH" ? "à¸£à¸°à¸šà¸šà¸ˆà¸°à¸ªà¹ˆà¸‡à¸‚à¹‰à¸­à¸¡à¸¹à¸¥à¸à¸²à¸£à¹€à¸‚à¹‰à¸²à¸ªà¸¹à¹ˆà¸£à¸°à¸šà¸šà¹„à¸›à¸—à¸µà¹ˆà¸­à¸µà¹€à¸¡à¸¥à¸™à¸µà¹‰" : "System notifications and login details will be sent to this email."}
                        </p>
                      </div>
                    </div>

                    <div className={styles.checkoutFeaturesRow}>
                      <div className={styles.checkoutFeatureBox}>
                        <span className={`${styles.icon} ${styles.checkoutFeatureIcon}`}>flash_on</span>
                        <div>
                          <h4 className={styles.checkoutFeatureTitle}>{lang === "TH" ? "à¸•à¸±à¹‰à¸‡à¸„à¹ˆà¸²à¸£à¸°à¸šà¸šà¸—à¸±à¸™à¸—à¸µ" : "Instant Setup"}</h4>
                          <p className={styles.checkoutFeatureDesc}>{lang === "TH" ? "à¸žà¸£à¹‰à¸­à¸¡à¹ƒà¸Šà¹‰à¸‡à¸²à¸™à¸ à¸²à¸¢à¹ƒà¸™à¹„à¸¡à¹ˆà¸à¸µà¹ˆà¸§à¸´à¸™à¸²à¸—à¸µ" : "Ready in seconds"}</p>
                        </div>
                      </div>
                      <div className={styles.checkoutFeatureBox}>
                        <span className={`${styles.icon} ${styles.checkoutFeatureIcon}`}>lock</span>
                        <div>
                          <h4 className={styles.checkoutFeatureTitle}>{lang === "TH" ? "à¸„à¸§à¸²à¸¡à¸›à¸¥à¸­à¸”à¸ à¸±à¸¢" : "Secure"}</h4>
                          <p className={styles.checkoutFeatureDesc}>{lang === "TH" ? "à¸Ÿà¸£à¸µ SSL Certificate" : "SSL included"}</p>
                        </div>
                      </div>
                      <div className={styles.checkoutFeatureBox}>
                        <span className={`${styles.icon} ${styles.checkoutFeatureIcon}`}>verified</span>
                        <div>
                          <h4 className={styles.checkoutFeatureTitle}>{lang === "TH" ? "Uptime 99.9%" : "99.9% Uptime"}</h4>
                          <p className={styles.checkoutFeatureDesc}>{lang === "TH" ? "à¸£à¸±à¸šà¸›à¸£à¸°à¸à¸±à¸™à¸„à¸§à¸²à¸¡à¹€à¸ªà¸–à¸µà¸¢à¸£" : "Guaranteed reliability"}</p>
                        </div>
                      </div>
                    </div>
                  </div>

                  <div className={styles.checkoutSidebar}>
                    <h2 className={styles.checkoutSummaryTitle}>{lang === "TH" ? "à¸ªà¸£à¸¸à¸›à¸„à¸³à¸ªà¸±à¹ˆà¸‡à¸‹à¸·à¹‰à¸­" : "Order Summary"}</h2>

                    <div className={styles.checkoutPlanTitle}>{checkoutPackage.name}</div>
                    <div className={styles.checkoutPlanSub}>{lang === "TH" ? "à¹à¸žà¹‡à¸à¹€à¸à¸ˆà¸£à¸²à¸¢à¹€à¸”à¸·à¸­à¸™" : "Monthly Plan"}</div>

                    <div className={styles.checkoutSpecsTitle} style={{ fontSize: "0.875rem", fontWeight: "600", marginBottom: "8px", color: "var(--text-main)" }}>
                      {lang === "TH" ? "à¸ªà¹€à¸›à¸à¹à¸žà¹‡à¸à¹€à¸à¸ˆ" : "Package Specs"}
                    </div>

                    <div className={styles.checkoutSpecRow}>
                      <span className={styles.checkoutSpecLabel}>{lang === "TH" ? "à¸žà¸·à¹‰à¸™à¸—à¸µà¹ˆà¸ˆà¸±à¸”à¹€à¸à¹‡à¸š" : "Disk Space"}</span>
                      <span className={styles.mono}>{checkoutPackage.disk} MB</span>
                    </div>
                    <div className={styles.checkoutSpecRow}>
                      <span className={styles.checkoutSpecLabel}>{lang === "TH" ? "à¹à¸šà¸™à¸”à¹Œà¸§à¸´à¸”à¸—à¹Œ" : "Bandwidth"}</span>
                      <span className={styles.mono}>unlimited MB</span>
                    </div>
                    <div className={styles.checkoutSpecRow}>
                      <span className={styles.checkoutSpecLabel}>{lang === "TH" ? "à¹‚à¸”à¹€à¸¡à¸™" : "Domains"}</span>
                      <span className={styles.mono}>unlimited</span>
                    </div>
                    <div className={styles.checkoutSpecRow}>
                      <span className={styles.checkoutSpecLabel}>{lang === "TH" ? "à¸à¸²à¸™à¸‚à¹‰à¸­à¸¡à¸¹à¸¥" : "Databases"}</span>
                      <span className={styles.mono}>unlimited</span>
                    </div>

                    <div className={styles.checkoutTotals}>
                      <div className={styles.checkoutTotalRow}>
                        <span className={styles.checkoutSpecLabel}>{lang === "TH" ? "à¸¢à¸­à¸”à¸£à¸§à¸¡" : "Subtotal"}</span>
                        <span className={styles.mono}>à¸¿{checkoutPackage.price}</span>
                      </div>
                      <div className={styles.checkoutTotalRow}>
                        <span className={styles.checkoutSpecLabel}>{lang === "TH" ? "à¸„à¹ˆà¸²à¸•à¸´à¸”à¸•à¸±à¹‰à¸‡" : "Setup Fee"}</span>
                        <span>{lang === "TH" ? "à¸Ÿà¸£à¸µ" : "Free"}</span>
                      </div>
                      <div className={styles.checkoutTotalDue}>
                        <span>{lang === "TH" ? "à¸¢à¸­à¸”à¸Šà¸³à¸£à¸°" : "Total Due"}</span>
                        <span className={styles.mono}>à¸¿{checkoutPackage.price}</span>
                      </div>
                    </div>

                    {orderError ? (
                      <p className={styles.errorText} style={{ marginTop: "16px" }}>{orderError}</p>
                    ) : null}
                    {orderSuccess ? (
                      <div style={{ marginTop: "16px" }}>
                        <p className={styles.successText}>{orderSuccess}</p>
                        {orderResult ? (
                          <div style={{
                            marginTop: "12px",
                            padding: "16px",
                            backgroundColor: "var(--surface-hover)",
                            border: "1px solid var(--border)",
                          }}>
                            <p style={{ fontSize: "0.875rem", fontWeight: 600, color: "var(--text-main)", marginBottom: "8px" }}>
                              {lang === "TH" ? "à¸‚à¹‰à¸­à¸¡à¸¹à¸¥à¸à¸²à¸£à¹€à¸‚à¹‰à¸²à¸ªà¸¹à¹ˆà¸£à¸°à¸šà¸š DirectAdmin" : "DirectAdmin Login Details"}
                            </p>
                            <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.875rem", marginBottom: "4px" }}>
                              <span style={{ color: "var(--text-muted)" }}>Username</span>
                              <span className={styles.mono}>{orderResult.da_username}</span>
                            </div>
                            <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.875rem", marginBottom: "4px" }}>
                              <span style={{ color: "var(--text-muted)" }}>Password</span>
                              <span className={styles.mono}>{orderResult.da_password}</span>
                            </div>
                            <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.875rem", gap: "8px" }}>
                              <span style={{ color: "var(--text-muted)" }}>
                                {lang === "TH" ? "DirectAdmin Panel (Login URL)" : "DirectAdmin Panel (Login URL)"}
                              </span>
                              <a
                                href={orderResult.da_panel_url?.trim() || "https://dcadmin.reverz.in.th/"}
                                target="_blank"
                                rel="noreferrer"
                                className={styles.mono}
                                style={{ color: "var(--brand)", wordBreak: "break-all", textAlign: "right" }}
                              >
                                {orderResult.da_panel_url?.trim() || "https://dcadmin.reverz.in.th/"}
                              </a>
                            </div>
                            <p style={{ fontSize: "0.8rem", color: "var(--text-muted)", marginTop: "6px" }}>
                              {lang === "TH" ? "Panel Domain: dcadmin.reverz.in.th" : "Panel Domain: dcadmin.reverz.in.th"}
                            </p>
                            <p style={{ fontSize: "0.8rem", color: "var(--text-muted)", marginTop: "8px" }}>
                              {lang === "TH"
                                ? "à¹ƒà¸«à¹‰à¹€à¸‚à¹‰à¸² URL à¸™à¸µà¹‰ à¹à¸¥à¹‰à¸§à¸¥à¹‡à¸­à¸à¸­à¸´à¸™à¸”à¹‰à¸§à¸¢ Username/Password à¸”à¹‰à¸²à¸™à¸šà¸™"
                                : "Open this URL and sign in with the username/password above."}
                            </p>
                          </div>
                        ) : null}
                      </div>
                    ) : null}

                    <button
                      className={styles.btnPrimary}
                      style={{ width: "100%", marginTop: "24px" }}
                      disabled={orderLoading || !!orderSuccess}
                      onClick={() => {
                        setOrderError(null);
                        setOrderSuccess(null);
                        setOrderResult(null);

                        const domain = checkoutDomain.trim();
                        const email = checkoutEmail.trim();

                        if (!domain) {
                          setOrderError(lang === "TH" ? "à¸à¸£à¸¸à¸“à¸²à¸à¸£à¸­à¸à¹‚à¸”à¹€à¸¡à¸™" : "Please enter a domain");
                          return;
                        }
                        if (!email) {
                          setOrderError(lang === "TH" ? "à¸à¸£à¸¸à¸“à¸²à¸à¸£à¸­à¸à¸­à¸µà¹€à¸¡à¸¥" : "Please enter an email");
                          return;
                        }

                        const price = parseFloat(checkoutPackage!.price);
                        if (accountBalance < price) {
                          setOrderError(lang === "TH" ? "à¸¢à¸­à¸”à¹€à¸‡à¸´à¸™à¹„à¸¡à¹ˆà¹€à¸žà¸µà¸¢à¸‡à¸žà¸­ à¸à¸£à¸¸à¸“à¸²à¹€à¸•à¸´à¸¡à¹€à¸‡à¸´à¸™à¸à¹ˆà¸­à¸™" : "Insufficient balance. Please top up first.");
                          return;
                        }

                        setOrderLoading(true);
                        void (async () => {
                          try {
                            const result = await orderHosting({
                              domain,
                              email,
                              package_name: checkoutPackage!.name,
                              price,
                            });
                            setOrderSuccess(result.message);
                            setOrderResult({
                              da_username: result.da_username,
                              da_password: result.da_password,
                              da_panel_url: result.da_panel_url,
                            });
                            void refreshWallet();
                            void refreshTransactions();
                            void refreshHostingServices();
                          } catch (error) {
                            if (isAuthInvalidError(error)) {
                              clearAuthSession();
                              router.push("/login");
                              return;
                            }
                            setOrderError(error instanceof Error ? error.message : (lang === "TH" ? "à¹€à¸à¸´à¸”à¸‚à¹‰à¸­à¸œà¸´à¸”à¸žà¸¥à¸²à¸”" : "Order failed"));
                          } finally {
                            setOrderLoading(false);
                          }
                        })();
                      }}
                    >
                      {orderLoading
                        ? (lang === "TH" ? "à¸à¸³à¸¥à¸±à¸‡à¸”à¸³à¹€à¸™à¸´à¸™à¸à¸²à¸£..." : "Processing...")
                        : (lang === "TH" ? "à¸¢à¸·à¸™à¸¢à¸±à¸™à¸à¸²à¸£à¸—à¸³à¸£à¸²à¸¢à¸à¸²à¸£" : "Complete Order")}
                    </button>
                  </div>
                </div>
              </>
            )}

            {currentView === "settings" && (
              <div className={styles.formContainer}>
                <div className={styles.formGroup}>
                  <label className={styles.label}>{lang === "TH" ? "à¸—à¸µà¹ˆà¸­à¸¢à¸¹à¹ˆà¸­à¸µà¹€à¸¡à¸¥" : "Email Address"}</label>
                  <input className={styles.input} value="root@reverz.local" readOnly />
                </div>
                <div className={styles.formGroup}>
                  <label className={styles.label}>{lang === "TH" ? "à¸Šà¸·à¹ˆà¸­à¸œà¸¹à¹‰à¹ƒà¸Šà¹‰" : "Username"}</label>
                  <input className={styles.input} value={getAuthSession()?.username ?? "root"} readOnly />
                </div>
              </div>
            )}

            {currentView === "sessions" && (
              <>
                {sessionActionError ? (
                  <div
                    style={{
                      marginBottom: "12px",
                      color: "var(--danger)",
                      fontSize: "0.875rem",
                      border: "1px solid var(--danger)",
                      backgroundColor: "color-mix(in srgb, var(--danger) 12%, transparent)",
                      padding: "10px 12px",
                    }}
                  >
                    {sessionActionError}
                  </div>
                ) : null}
                <div className={styles.colSpan3}>
                  <div className={styles.tableWrap}>
                    <table className={styles.table}>
                      <thead>
                        <tr>
                          <th>{lang === "TH" ? "à¸­à¸¸à¸›à¸à¸£à¸“à¹Œ / à¹€à¸šà¸£à¸²à¸§à¹Œà¹€à¸‹à¸­à¸£à¹Œ" : "Device / Browser"}</th>
                          <th>{lang === "TH" ? "à¹„à¸­à¸žà¸µà¹à¸­à¸”à¹€à¸”à¸£à¸ª" : "IP Address"}</th>
                          <th>{lang === "TH" ? "à¸ªà¸–à¸²à¸™à¸—à¸µà¹ˆ" : "Location"}</th>
                          <th>{lang === "TH" ? "à¹ƒà¸Šà¹‰à¸‡à¸²à¸™à¸¥à¹ˆà¸²à¸ªà¸¸à¸”" : "Last Active"}</th>
                          <th style={{ textAlign: "right", minWidth: "100px" }}></th>
                        </tr>
                      </thead>
                      <tbody>
                        {activeSessions.length === 0 ? (
                          <tr className={styles.tableRow}>
                            <td colSpan={5} style={{ textAlign: "center", color: "var(--text-muted)" }}>
                              {lang === "TH" ? "à¹„à¸¡à¹ˆà¸¡à¸µà¹€à¸‹à¸ªà¸Šà¸±à¸™à¸—à¸µà¹ˆà¹€à¸›à¸´à¸”à¹ƒà¸Šà¹‰à¸‡à¸²à¸™" : "No active sessions."}
                            </td>
                          </tr>
                        ) : (
                          activeSessions.map((session) => (
                            <tr key={session.id} className={styles.tableRow}>
                              <td>{session.device}</td>
                              <td className={styles.mono}>{session.ip}</td>
                              <td>{session.location}</td>
                              <td>{formatLastActive(session.lastActiveValue)}</td>
                              <td className={styles.tdAction}>
                                {session.isCurrent ? (
                                  <button
                                    disabled
                                    style={{
                                      display: "inline-flex",
                                      alignItems: "center",
                                      justifyContent: "center",
                                      height: "32px",
                                      padding: "0 12px",
                                      border: "1px solid var(--border)",
                                      backgroundColor: "transparent",
                                      color: "var(--text-muted)",
                                      borderRadius: "2px",
                                      cursor: "not-allowed",
                                      fontSize: "0.875rem",
                                    }}
                                  >
                                    {lang === "TH" ? "à¸¢à¸à¹€à¸¥à¸´à¸" : "Revoke"}
                                  </button>
                                ) : (
                                  <button
                                    className={styles.qBtnDanger}
                                    style={{ display: "inline-flex", width: "auto", justifyContent: "center", borderRadius: "2px" }}
                                    disabled={revokingSessionId === session.id}
                                    onClick={() => {
                                      void handleRevokeSession(session.id);
                                    }}
                                  >
                                    {revokingSessionId === session.id ? (lang === "TH" ? "à¸à¸³à¸¥à¸±à¸‡à¸¢à¸à¹€à¸¥à¸´à¸..." : "Revoking...") : (lang === "TH" ? "à¸¢à¸à¹€à¸¥à¸´à¸" : "Revoke")}
                                  </button>
                                )}
                              </td>
                            </tr>
                          ))
                        )}
                      </tbody>
                    </table>
                  </div>
                </div>
              </>
            )}
          </div>
        </main>
      </div>
    </div>
  );
}
