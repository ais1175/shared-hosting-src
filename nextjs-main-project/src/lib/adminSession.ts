export const ADMIN_SESSION_STORAGE_KEY = "reverz-admin-session";

export type AdminSession = {
  username: string;
  role: "admin";
  loggedInAt: string;
  accessToken: string;
};

export function getAdminSession(): AdminSession | null {
  if (typeof window === "undefined") return null;

  const raw = window.localStorage.getItem(ADMIN_SESSION_STORAGE_KEY);
  if (!raw) return null;

  try {
    const parsed = JSON.parse(raw) as AdminSession;
    if (!parsed.accessToken || !parsed.username || parsed.role !== "admin") {
      window.localStorage.removeItem(ADMIN_SESSION_STORAGE_KEY);
      return null;
    }
    return parsed;
  } catch {
    window.localStorage.removeItem(ADMIN_SESSION_STORAGE_KEY);
    return null;
  }
}

export function setAdminSession(session: AdminSession) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(ADMIN_SESSION_STORAGE_KEY, JSON.stringify(session));
}

export function clearAdminSession() {
  if (typeof window === "undefined") return;
  window.localStorage.removeItem(ADMIN_SESSION_STORAGE_KEY);
}
