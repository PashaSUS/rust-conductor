import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { taskApi } from "@/api/conductor";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Layers, Search, Inbox } from "lucide-react";
import { useThemeText } from "@/components/ThemeContext";

export default function TaskQueues() {
  const [filter, setFilter] = useState("");
  const t = useThemeText();

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
                return (
                  <div key={name} className="group rounded-lg border p-3 hover:bg-muted/30 transition-colors">
                    <div className="flex items-center justify-between mb-1.5">
                      <span className="font-medium text-sm">{name}</span>
                      <div className="flex items-center gap-2">
                        <span className="text-xs text-muted-foreground">
                          {total > 0 ? `${((count / total) * 100).toFixed(0)}%` : ""}
                        </span>
                        <Badge variant="secondary" className="font-mono">{count}</Badge>
                      </div>
                    </div>
                    <div className="w-full bg-muted rounded-full h-2">
                      <div
                        className="bg-chart-1 h-2 rounded-full transition-all duration-500"
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
