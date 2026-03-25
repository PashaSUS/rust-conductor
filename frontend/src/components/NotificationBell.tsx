import { useState, useEffect, useRef } from "react";
import { Bell } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface Notification {
  id: string;
  type: "failure" | "sla" | "info";
  message: string;
  timestamp: number;
}

const MAX_NOTIFICATIONS = 50;
const STORAGE_KEY = "conductor-notifications";

function loadNotifications(): Notification[] {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) return JSON.parse(stored);
  } catch { /* ignore */ }
  return [];
}

function saveNotifications(notifications: Notification[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(notifications.slice(0, MAX_NOTIFICATIONS)));
}

export function NotificationBell() {
  const [notifications, setNotifications] = useState<Notification[]>(loadNotifications);
  const [open, setOpen] = useState(false);
  const [unreadCount, setUnreadCount] = useState(0);
  const panelRef = useRef<HTMLDivElement>(null);

  // SSE real-time alerts disabled — /api/events/stream does not exist in backend.
  // Backend has /sse/workflow/{id} and /sse/queue/sizes but no general event stream.

  // Close panel on outside click
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const clearAll = () => {
    setNotifications([]);
    setUnreadCount(0);
    saveNotifications([]);
  };

  const formatTime = (ts: number) => {
    const diff = Date.now() - ts;
    if (diff < 60000) return "just now";
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
    return new Date(ts).toLocaleDateString();
  };

  return (
    <div className="relative" ref={panelRef}>
      <Button
        variant="ghost"
        size="icon"
        className="h-8 w-8 relative"
        onClick={() => {
          setOpen((o) => !o);
          if (!open) setUnreadCount(0);
        }}
      >
        <Bell className="h-4 w-4" />
        {unreadCount > 0 && (
          <span className="absolute -top-0.5 -right-0.5 flex h-4 min-w-4 items-center justify-center rounded-full bg-destructive px-1 text-[10px] font-bold text-destructive-foreground">
            {unreadCount > 9 ? "9+" : unreadCount}
          </span>
        )}
      </Button>

      {open && (
        <div className="absolute right-0 top-10 z-50 w-80 max-h-96 overflow-auto rounded-lg border bg-popover shadow-lg animate-in fade-in-0 zoom-in-95">
          <div className="flex items-center justify-between px-3 py-2 border-b">
            <span className="text-sm font-semibold">Notifications</span>
            {notifications.length > 0 && (
              <Button variant="ghost" size="sm" className="text-xs h-6" onClick={clearAll}>
                Clear all
              </Button>
            )}
          </div>
          {notifications.length === 0 ? (
            <div className="px-3 py-6 text-center text-sm text-muted-foreground">
              No notifications
            </div>
          ) : (
            <div className="divide-y">
              {notifications.map((n) => (
                <div key={n.id} className="px-3 py-2 hover:bg-muted transition-colors">
                  <div className="flex items-start gap-2">
                    <span
                      className={cn(
                        "mt-1 h-2 w-2 rounded-full shrink-0",
                        n.type === "failure" ? "bg-red-500" : n.type === "sla" ? "bg-amber-500" : "bg-blue-500",
                      )}
                    />
                    <div className="flex-1 min-w-0">
                      <p className="text-xs">{n.message}</p>
                      <p className="text-[10px] text-muted-foreground">{formatTime(n.timestamp)}</p>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
