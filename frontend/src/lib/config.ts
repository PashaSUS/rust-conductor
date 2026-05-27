/**
 * Frontend runtime config: API base URL and "lite mode".
 *
 * When a custom API URL is set, the UI enters lite mode and hides all
 * rust-conductor-specific features (signals, checkpoints, designer,
 * stresser, templates, diff, compare, validation, dependency graph,
 * metrics, schedules, dashboard) so it can talk to a stock Netflix
 * Conductor OSS REST API.
 */

const API_URL_KEY = "conductor.api.url";
const LITE_MODE_KEY = "conductor.lite.mode";
const API_PROXY_KEY = "conductor.api.proxy";

/** Returns the configured API base, e.g. "" (same origin) or "http://localhost:8080". */
export function getApiBase(): string {
  const stored = (typeof localStorage !== "undefined") ? localStorage.getItem(API_URL_KEY) : null;
  if (stored && stored.trim().length > 0) {
    return normalizeApiBase(stored);
  }
  return normalizeApiBase((import.meta.env.VITE_API_BASE as string | undefined) ?? "");
}

export function conductorUrl(path: string): string {
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  const base = getApiBase().replace(/\/+$/, "");

  if (!base) {
    return normalizedPath;
  }

  if (shouldProxy(base)) {
    return `/api/proxy?target=${encodeURIComponent(`${base}${normalizedPath}`)}`;
  }

  return `${base}${normalizedPath}`;
}

export function normalizeApiBase(url: string): string {
  const trimmed = url.trim();
  if (!trimmed) return "";

  const withoutTrailingSlash = trimmed.replace(/\/+$/, "");
  try {
    const parsed = new URL(withoutTrailingSlash);
    parsed.hash = "";
    parsed.search = "";
    if (parsed.pathname.toLowerCase().endsWith("/api")) {
      parsed.pathname = parsed.pathname.slice(0, -4) || "/";
    }
    return parsed.toString().replace(/\/+$/, "");
  } catch {
    return withoutTrailingSlash.replace(/\/api$/i, "");
  }
}

export function shouldProxy(base: string): boolean {
  if (!getUseApiProxy()) return false;
  if (typeof window === "undefined") return false;
  try {
    const url = new URL(base);
    return ["http:", "https:"].includes(url.protocol) && url.origin !== window.location.origin;
  } catch {
    return false;
  }
}

export function setApiBase(url: string | null): void {
  if (!url || url.trim().length === 0) {
    localStorage.removeItem(API_URL_KEY);
  } else {
    localStorage.setItem(API_URL_KEY, normalizeApiBase(url));
  }
}

/** Returns true when the user has opted into lite mode (base Conductor compatibility). */
export function isLiteMode(): boolean {
  if (typeof localStorage === "undefined") return false;
  return localStorage.getItem(LITE_MODE_KEY) === "true";
}

export function setLiteMode(on: boolean): void {
  if (on) {
    localStorage.setItem(LITE_MODE_KEY, "true");
  } else {
    localStorage.removeItem(LITE_MODE_KEY);
  }
}

/** Pages allowed in lite mode (everything else is hidden). */
export const LITE_MODE_PATHS = new Set<string>([
  "/",
  "/executions",
  "/definitions",
  "/definitions/create",
  "/taskdefs",
  "/taskdefs/create",
  "/queues",
  "/settings",
  "/about",
]);

export function isPathAllowed(path: string): boolean {
  if (!isLiteMode()) return true;
  // Allow exact match or as a prefix segment.
  if (LITE_MODE_PATHS.has(path)) return true;
  return [...LITE_MODE_PATHS].some(
    (p) => p !== "/" && (path === p || path.startsWith(`${p}/`)),
  );
}

export function getUseApiProxy(): boolean {
  const envDefault = (import.meta.env.VITE_USE_API_PROXY as string | undefined) !== "false";
  if (typeof localStorage === "undefined") return envDefault;
  const stored = localStorage.getItem(API_PROXY_KEY);
  if (stored === "true") return true;
  if (stored === "false") return false;
  return envDefault;
}

export function setUseApiProxy(on: boolean): void {
  localStorage.setItem(API_PROXY_KEY, String(on));
}

export function resetUseApiProxy(): void {
  localStorage.removeItem(API_PROXY_KEY);
}
