import { SESSION_STORAGE_KEY, type AuthSession } from "@/lib/authSession";

const API_BASE = process.env.NEXT_PUBLIC_RUST_PROXY_BASE ?? "/api/rust";

export type ApiDeviceSession = {
  id: string;
  device: string;
  ip: string;
  location: string;
  last_active: string;
  is_current: boolean;
};

export class ApiRequestError extends Error {
  status?: number;
  errorCode?: string;

  constructor(message: string, options?: { status?: number; errorCode?: string }) {
    super(message);
    this.name = "ApiRequestError";
    this.status = options?.status;
    this.errorCode = options?.errorCode;
  }
}

function getAccessToken(): string | null {
  if (typeof window === "undefined") return null;
  const raw = window.localStorage.getItem(SESSION_STORAGE_KEY);
  if (!raw) return null;

  try {
    const parsed = JSON.parse(raw) as AuthSession;
    return parsed.accessToken ?? null;
  } catch {
    return null;
  }
}

async function request<T>(path: string, init: RequestInit = {}, auth = true): Promise<T> {
  const headers = new Headers(init.headers ?? {});
  headers.set("Content-Type", "application/json");

  if (auth) {
    const token = getAccessToken();
    if (!token) {
      throw new ApiRequestError("Not authenticated", {
        status: 401,
        errorCode: "UNAUTHENTICATED",
      });
    }
    headers.set("Authorization", `Bearer ${token}`);
  }

  let response: Response;
  try {
    response = await fetch(`${API_BASE}${path}`, {
      ...init,
      credentials: "include",
      headers,
    });
  } catch (error) {
    throw new ApiRequestError(
      error instanceof Error ? error.message : "Network request failed",
    );
  }

  const payload = (await response.json().catch(() => ({}))) as T & {
    message?: string;
    ok?: boolean;
    error_code?: string;
  };

  if (!response.ok) {
    throw new ApiRequestError(payload.message ?? `HTTP ${response.status}`, {
      status: response.status,
      errorCode: payload.error_code,
    });
  }

  return payload;
}

export async function loginWithPassword(params: {
  username: string;
  password: string;
  proof: string;
  nonce: string;
}): Promise<AuthSession> {
  const payload = await request<{ ok: boolean; session: AuthSession }>(
    "/login",
    {
      method: "POST",
      body: JSON.stringify(params),
    },
    false,
  );

  return payload.session;
}

export async function logoutApi(): Promise<void> {
  await request<{ ok: boolean }>("/logout", { method: "POST", body: "{}" });
}

export async function refreshAccessToken(): Promise<string> {
  const payload = await request<{ ok: boolean; accessToken: string }>(
    "/auth/refresh",
    { method: "POST", body: "{}" },
    false,
  );
  return payload.accessToken;
}

export async function listDeviceSessions(): Promise<ApiDeviceSession[]> {
  return request<ApiDeviceSession[]>("/sessions", { method: "GET" });
}

export async function revokeDeviceSession(sessionId: string): Promise<void> {
  await request<{ ok: boolean; message: string }>(`/sessions/${encodeURIComponent(sessionId)}/revoke`, {
    method: "POST",
    body: "{}",
  });
}
