import { useState, useCallback } from "react";
import { useQuery } from "@tanstack/react-query";
import { taskApi } from "@/api/conductor";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Layers, Search, Inbox, Bell, BellOff, AlertTriangle, X } from "lucide-react";
import { useThemeText } from "@/components/ThemeContext";

function loadThresholds(): Record<string, number> {
  try {
    const stored = localStorage.getItem("queue-alert-thresholds");
    if (stored) return JSON.parse(stored);
  } catch { /* ignore */ }
  return {};
}

function saveThresholds(thresholds: Record<string, number>) {
  localStorage.setItem("queue-alert-thresholds", JSON.stringify(thresholds));
}

export default function TaskQueues() {
  const [filter, setFilter] = useState("");
  const [alertsOpen, setAlertsOpen] = useState(false);
  const [thresholds, setThresholds] = useState<Record<string, number>>(loadThresholds);
  const [editingQueue, setEditingQueue] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const t = useThemeText();

  const setThreshold = useCallback((queue: string, value: number) => {
    setThresholds((prev) => {
      const next = { ...prev, [queue]: value };
      saveThresholds(next);
      return next;
    });
  }, []);

  const clearThreshold = useCallback((queue: string) => {
    setThresholds((prev) => {
      const next = { ...prev };
      delete next[queue];
      saveThresholds(next);
      return next;
    });
  }, []);

  const { data: sizes, isLoading } = useQuery({
    queryKey: ["queue-sizes"],
    queryFn: taskApi.queueSizes,
    refetchInterval: 5000,
  });

  const allEntries = Object.entries(sizes ?? {}).sort((a, b) => b[1] - a[1]);
  const total = allEntries.reduce((sum, [, v]) => sum + v, 0);
  const maxCount = allEntries.length > 0 ? allEntries[0][1] : 1;

  const entries = filter
    ? allEntries.filter(([name]) => name.toLowerCase().includes(filter.toLowerCase()))
    : allEntries;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold tracking-tight">{t.taskQueuesTitle}</h2>
        <div className="flex items-center gap-3">
          <Button variant="outline" size="sm" onClick={() => setAlertsOpen((o) => !o)}>
            {alertsOpen ? <BellOff className="h-3 w-3 mr-1" /> : <Bell className="h-3 w-3 mr-1" />}
            {t.configureAlerts}
          </Button>
          <div className="flex items-center gap-1 text-xs text-muted-foreground">
            <span className="inline-block w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
            {t.liveInterval}
          </div>
          <Badge variant="secondary" className="gap-1">
            <Layers className="h-3 w-3" />
            {total} {t.totalQueued}
          </Badge>
        </div>
      </div>

      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <Input
          placeholder={t.filterQueues}
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="pl-9 max-w-sm"
        />
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium">
            {t.activeQueues} {filter && <span className="text-muted-foreground font-normal">({entries.length} {t.of} {allEntries.length})</span>}
          </CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <p className="text-muted-foreground text-sm">{t.loading}</p>
          ) : entries.length === 0 ? (
            <div className="py-8 text-center">
              <Inbox className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
              <p className="text-muted-foreground text-sm">
                {filter ? t.noQueuesMatch : t.noTasksInQueue}
              </p>
            </div>
          ) : (
            <div className="space-y-2">
              {entries.map(([name, count]) => {
                const pct = Math.max((count / maxCount) * 100, 2);
                const threshold = thresholds[name];
                const exceeded = threshold !== undefined && count >= threshold;
                return (
                  <div key={name} className={`group rounded-lg border p-3 transition-colors ${exceeded ? "border-destructive/50 bg-destructive/5" : "hover:bg-muted/30"}`}>
                    <div className="flex items-center justify-between mb-1.5">
                      <div className="flex items-center gap-2">
                        <span className="font-medium text-sm">{name}</span>
                        {exceeded && (
                          <span className="flex items-center gap-1 text-destructive text-xs">
                            <AlertTriangle className="h-3 w-3" />
                            {t.thresholdExceeded}
                          </span>
                        )}
                      </div>
                      <div className="flex items-center gap-2">
                        {threshold !== undefined && (
                          <span className="text-xs text-muted-foreground">
                            {t.alertThreshold}: {threshold}
                          </span>
                        )}
                        {alertsOpen && (
                          editingQueue === name ? (
                            <form
                              className="flex items-center gap-1"
                              onSubmit={(e) => {
                                e.preventDefault();
                                const val = parseInt(editValue, 10);
                                if (!isNaN(val) && val > 0) setThreshold(name, val);
                                setEditingQueue(null);
                              }}
                            >
                              <Input
                                type="number"
                                min="1"
                                value={editValue}
                                onChange={(e) => setEditValue(e.target.value)}
                                className="h-6 w-20 text-xs"
                                autoFocus
                                onBlur={() => setEditingQueue(null)}
                              />
                            </form>
                          ) : (
                            <div className="flex items-center gap-1">
                              <Button
                                variant="ghost"
                                size="sm"
                                className="h-6 text-xs px-2"
                                onClick={() => {
                                  setEditingQueue(name);
                                  setEditValue(String(threshold ?? ""));
                                }}
                              >
                                <Bell className="h-3 w-3 mr-1" />
                                {t.alertThreshold}
                              </Button>
                              {threshold !== undefined && (
                                <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => clearThreshold(name)}>
                                  <X className="h-3 w-3" />
                                </Button>
                              )}
                            </div>
                          )
                        )}
                        <span className="text-xs text-muted-foreground">
                          {total > 0 ? `${((count / total) * 100).toFixed(0)}%` : ""}
                        </span>
                        <Badge variant="secondary" className="font-mono">{count}</Badge>
                      </div>
                    </div>
                    <div className="w-full bg-muted rounded-full h-2">
                      <div
                        className={`h-2 rounded-full transition-all duration-500 ${exceeded ? "bg-destructive" : "bg-chart-1"}`}
                        style={{ width: `${pct}%` }}
                      />
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
