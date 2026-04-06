import {
  clearAuthSession,
  SESSION_STORAGE_KEY,
  setAuthSession,
  type AuthSession,
} from "@/lib/authSession";
import { ApiRequestError, refreshAccessToken } from "@/lib/authClient";

const TOPUP_API_BASE = process.env.NEXT_PUBLIC_RUST_PROXY_BASE ?? "/api/rust";

export type WalletResponse = {
  ok: boolean;
  username: string;
  balance_thb: number;
  receiver_phone: string;
  banking_receiver_id: string;
  banking_receiver_name: string;
};

export type TopupTransaction = {
  tx_id: string;
  voucher_hash: string;
  amount_thb: number;
  status: "success" | "failed" | "pending" | string;
  error_code: string | null;
  message: string;
  created_at: string;
};

export type TransactionsResponse = {
  ok: boolean;
  items: TopupTransaction[];
};

export type RedeemResponse = {
  success: boolean;
  amount: number;
  message: string;
  error_code?: string | null;
};

export type ApiError = {
  message: string;
  error_code?: string;
};

function isAuthInvalidRefreshError(error: unknown): boolean {
  if (!(error instanceof ApiRequestError)) return false;
  if (error.status !== 401) return false;
  return error.errorCode === "UNAUTHENTICATED" || error.errorCode === "INVALID_REFRESH_TOKEN";
}

function getSession(): AuthSession | null {
  if (typeof window === "undefined") return null;
  const raw = window.localStorage.getItem(SESSION_STORAGE_KEY);
  if (!raw) return null;

  try {
    return JSON.parse(raw) as AuthSession;
  } catch {
    return null;
  }
}

function setSessionAccessToken(nextAccessToken: string): AuthSession | null {
  const session = getSession();
  if (!session) return null;

  const nextSession: AuthSession = {
    ...session,
    accessToken: nextAccessToken,
  };
  setAuthSession(nextSession);
  return nextSession;
}

async function doRequest<T>(path: string, init: RequestInit, accessToken: string): Promise<Response> {
  const headers = new Headers(init.headers ?? {});
  headers.set("Content-Type", "application/json");
  headers.set("Authorization", `Bearer ${accessToken}`);

  return fetch(`${TOPUP_API_BASE}${path}`, {
    ...init,
    credentials: "include",
    headers,
  });
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const session = getSession();
  if (!session?.accessToken) {
    throw new ApiRequestError("Not authenticated", {
      status: 401,
      errorCode: "UNAUTHENTICATED",
    });
  }

  let response = await doRequest<T>(path, init, session.accessToken);
  if (response.status === 401) {
    try {
      const refreshedAccessToken = await refreshAccessToken();
      const nextSession = setSessionAccessToken(refreshedAccessToken);
      if (!nextSession?.accessToken) {
        clearAuthSession();
        throw new ApiRequestError("Session expired. Please login again.", {
          status: 401,
          errorCode: "INVALID_REFRESH_TOKEN",
        });
      }
      response = await doRequest<T>(path, init, nextSession.accessToken);
    } catch (error) {
      if (isAuthInvalidRefreshError(error)) {
        clearAuthSession();
        throw new ApiRequestError("Session expired. Please login again.", {
          status: 401,
          errorCode: "INVALID_REFRESH_TOKEN",
        });
      }

      throw new ApiRequestError(
        error instanceof Error ? error.message : "Unable to refresh session right now.",
      );
    }
  }

  const payload = (await response.json().catch(() => ({}))) as T & ApiError;

  if (!response.ok) {
    throw new ApiRequestError(payload.message ?? `HTTP ${response.status}`, {
      status: response.status,
      errorCode: payload.error_code,
    });
  }

  return payload;
}

export async function getWallet() {
  return request<WalletResponse>("/topup/wallet", { method: "GET" });
}

export async function getTransactions(limit = 20) {
  return request<TransactionsResponse>(`/topup/transactions?limit=${encodeURIComponent(String(limit))}`, {
    method: "GET",
  });
}

export async function redeemTrueMoneyVoucher(voucherUrl: string) {
  return request<RedeemResponse>("/topup/truemoney/redeem", {
    method: "POST",
    body: JSON.stringify({ voucher_url: voucherUrl }),
  });
}

export async function redeemBankingSlip(imgDataUrl: string) {
  return request<RedeemResponse>("/topup/banking/slip/redeem", {
    method: "POST",
    body: JSON.stringify({ img: imgDataUrl }),
  });
}

export type OrderHostingRequest = {
  domain: string;
  email: string;
  package_name: string;
  price: number;
};

export type OrderHostingResponse = {
  ok: boolean;
  message: string;
  da_username?: string;
  da_password?: string;
  da_panel_url?: string;
};

export async function orderHosting(params: OrderHostingRequest) {
  return request<OrderHostingResponse>("/hosting/order", {
    method: "POST",
    body: JSON.stringify(params),
  });
}

export type HostingServiceItem = {
  domain: string;
  package_name: string;
  created_at: string;
  status: "active" | "grace_suspended" | "suspended_expired" | string;
  expires_at: string;
  grace_until: string;
  suspended_at?: string | null;
  billing_price_thb: number;
  notified_d1_at?: string | null;
  notified_expired_at?: string | null;
  notified_grace_end_at?: string | null;
  da_username?: string | null;
  da_password?: string | null;
  da_panel_url?: string | null;
};

export type HostingServicesResponse = {
  ok: boolean;
  total_active: number;
  items: HostingServiceItem[];
};

export async function getHostingServices() {
  return request<HostingServicesResponse>("/hosting/services", { method: "GET" });
}

export type RenewHostingResponse = {
  ok: boolean;
  message: string;
  domain: string;
  status: "active" | "grace_suspended" | "suspended_expired" | string;
  expires_at: string;
  grace_until: string;
  charged_amount: number;
  balance_thb: number;
};

export async function renewHostingService(domain: string) {
  return request<RenewHostingResponse>("/hosting/services/renew", {
    method: "POST",
    body: JSON.stringify({ domain }),
  });
}

export type NotificationItem = {
  id: string;
  notification_type: string;
  title: string;
  message: string;
  created_at: string;
  read: boolean;
};

export type NotificationsResponse = {
  ok: boolean;
  items: NotificationItem[];
  unread_count: number;
};

export type MarkReadNotificationsResponse = {
  ok: boolean;
  unread_count: number;
};

export async function getNotifications(limit = 20) {
  return request<NotificationsResponse>(`/notifications?limit=${encodeURIComponent(String(limit))}`, {
    method: "GET",
  });
}

export async function markNotificationsRead() {
  return request<MarkReadNotificationsResponse>("/notifications/mark-read", { method: "POST", body: "{}" });
}
