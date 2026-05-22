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

/** Returns the configured API base, e.g. "" (same origin) or "http://localhost:8080". */
export function getApiBase(): string {
  const stored = (typeof localStorage !== "undefined") ? localStorage.getItem(API_URL_KEY) : null;
  if (stored && stored.trim().length > 0) {
    return stored.replace(/\/+$/, "");
  }
  return (import.meta.env.VITE_API_BASE as string | undefined) ?? "";
}

export function setApiBase(url: string | null): void {
  if (!url || url.trim().length === 0) {
    localStorage.removeItem(API_URL_KEY);
  } else {
    localStorage.setItem(API_URL_KEY, url.trim());
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
