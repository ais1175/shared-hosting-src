import {
  clearAdminSession,
  getAdminSession,
  setAdminSession,
  type AdminSession,
} from "@/lib/adminSession";
import { ApiRequestError, refreshAccessToken } from "@/lib/authClient";

const API_BASE = process.env.NEXT_PUBLIC_RUST_PROXY_BASE ?? "/api/rust";

export type AdminLoginRequest = {
  email: string;
  password: string;
  nonce: string;
  proof: string;
};

export type AdminSummaryResponse = {
  ok: boolean;
  total_users: number;
  total_active_services: number;
  total_services_all_status: number;
  total_transactions: number;
  wallet_total_thb: number;
  unread_notifications_total: number;
};

export type AdminServiceView = {
  owner_username: string;
  domain: string;
  package_name: string;
  status: string;
  created_at: string;
  expires_at: string;
  grace_until: string;
  da_username_masked: string;
  da_password_masked: string;
};

export type AdminServicesResponse = {
  ok: boolean;
  items: AdminServiceView[];
};

export type AdminTransactionView = {
  tx_id: string;
  owner_username: string;
  voucher_hash_masked: string;
  voucher_method: string;
  amount_thb: number;
  status: string;
  message: string;
  created_at: string;
};

export type AdminTransactionsResponse = {
  ok: boolean;
  items: AdminTransactionView[];
};

export type AdminUserWalletView = {
  username: string;
  balance_thb: number;
};

export type AdminUserWalletsResponse = {
  ok: boolean;
  items: AdminUserWalletView[];
};

type ApiErrorPayload = {
  message?: string;
  error_code?: string;
};

function isAuthInvalidRefreshError(error: unknown): boolean {
  if (!(error instanceof ApiRequestError)) return false;
  if (error.status !== 401) return false;
  return error.errorCode === "UNAUTHENTICATED" || error.errorCode === "INVALID_REFRESH_TOKEN";
}

async function doRequest(path: string, init: RequestInit, accessToken: string): Promise<Response> {
  const headers = new Headers(init.headers ?? {});
  headers.set("Content-Type", "application/json");
  headers.set("Authorization", `Bearer ${accessToken}`);

  return fetch(`${API_BASE}${path}`, {
    ...init,
    credentials: "include",
    headers,
  });
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const session = getAdminSession();
  if (!session?.accessToken) {
    throw new ApiRequestError("Not authenticated", {
      status: 401,
      errorCode: "UNAUTHENTICATED",
    });
  }

  let response = await doRequest(path, init, session.accessToken);
  if (response.status === 401) {
    try {
      const refreshedAccessToken = await refreshAccessToken();
      const currentSession = getAdminSession();
      if (!currentSession) {
        clearAdminSession();
        throw new ApiRequestError("Session expired. Please login again.", {
          status: 401,
          errorCode: "INVALID_REFRESH_TOKEN",
        });
      }
      const nextSession: AdminSession = {
        ...currentSession,
        accessToken: refreshedAccessToken,
      };
      setAdminSession(nextSession);
      response = await doRequest(path, init, nextSession.accessToken);
    } catch (error) {
      if (isAuthInvalidRefreshError(error)) {
        clearAdminSession();
        throw new ApiRequestError("Session expired. Please login again.", {
          status: 401,
          errorCode: "INVALID_REFRESH_TOKEN",
        });
      }
      throw new ApiRequestError(
        error instanceof Error ? error.message : "Unable to refresh admin session right now.",
      );
    }
  }

  const payload = (await response.json().catch(() => ({}))) as T & ApiErrorPayload;

  if (!response.ok) {
    throw new ApiRequestError(payload.message ?? `HTTP ${response.status}`, {
      status: response.status,
      errorCode: payload.error_code,
    });
  }

  return payload;
}

export async function adminLogin(params: AdminLoginRequest): Promise<AdminSession> {
  const response = await fetch(`${API_BASE}/admin/login`, {
    method: "POST",
    credentials: "include",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(params),
  });

  const payload = (await response.json().catch(() => ({}))) as {
    ok?: boolean;
    message?: string;
    error_code?: string;
    session?: AdminSession;
  };

  if (!response.ok || !payload.session) {
    throw new ApiRequestError(payload.message ?? "Admin login failed", {
      status: response.status,
      errorCode: payload.error_code,
    });
  }

  return payload.session;
}

export async function getAdminSummary() {
  return request<AdminSummaryResponse>("/admin/summary", { method: "GET" });
}

export async function getAdminRecentServices(limit = 20) {
  return request<AdminServicesResponse>(`/admin/recent-services?limit=${encodeURIComponent(String(limit))}`, {
    method: "GET",
  });
}

export async function getAdminRecentTransactions(limit = 20) {
  return request<AdminTransactionsResponse>(`/admin/recent-transactions?limit=${encodeURIComponent(String(limit))}`, {
    method: "GET",
  });
}

export async function getAdminUserWallets() {
  return request<AdminUserWalletsResponse>("/admin/user-wallets", { method: "GET" });
}

export async function adminLogout() {
  await request<{ ok: boolean }>("/logout", { method: "POST", body: "{}" });
}
