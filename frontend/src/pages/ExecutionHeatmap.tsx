import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { workflowApi } from "@/api/conductor";
import { Card, CardContent } from "@/components/ui/card";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { Input } from "@/components/ui/input";

const STATUS_COLORS: Record<string, string> = {
  RUNNING: "#3b82f6",
  COMPLETED: "#10b981",
  FAILED: "#ef4444",
  TIMED_OUT: "#f97316",
  PAUSED: "#f59e0b",
  TERMINATED: "#9ca3af",
};

type PeriodPreset = "24h" | "7d" | "30d" | "custom";

/**
 * #178 — Execution heatmap view.
 * Shows a calendar-style heatmap of workflow executions over time,
 * with color intensity based on execution count per time bucket.
 */
export default function ExecutionHeatmap() {
  const [periodPreset, setPeriodPreset] = useState<PeriodPreset>("7d");
  const [customDays, setCustomDays] = useState(30);
  const [customIntervalHours, setCustomIntervalHours] = useState(1);
  const [statusFilter, setStatusFilter] = useState<string>("all");

  const { data: searchResult } = useQuery({
    queryKey: ["heatmapWorkflows", periodPreset, customDays, customIntervalHours, statusFilter],
    queryFn: () =>
      workflowApi.search({
        status: statusFilter === "all" ? undefined : statusFilter,
        size: 1000,
      }),
    refetchInterval: 30000,
  });

  const { buckets, maxCount, cols, bucketLabels } = useMemo(() => {
    const now = Date.now();
    let bucketSize: number;
    let totalBuckets: number;
    let colCount: number;

    if (periodPreset === "custom") {
      bucketSize = Math.max(customIntervalHours, 1) * 3600000;
      totalBuckets = Math.ceil((customDays * 86400000) / bucketSize);
      colCount = Math.min(Math.ceil(24 / customIntervalHours), 24);
    } else if (periodPreset === "24h") {
      bucketSize = 3600000; // 1 hour
      totalBuckets = 24;
      colCount = 24;
    } else if (periodPreset === "7d") {
      bucketSize = 3600000 * 4; // 4 hours
      totalBuckets = 42;
      colCount = 6; // 6 per day
    } else {
      bucketSize = 86400000; // 1 day
      totalBuckets = 30;
      colCount = 7; // week columns
    }

    const start = now - totalBuckets * bucketSize;
    const counts = Array(totalBuckets).fill(0) as number[];
    const bucketStatuses = Array.from({ length: totalBuckets }, () => new Map<string, number>());
    const labels: string[] = [];

    // Generate labels
    for (let i = 0; i < totalBuckets; i++) {
      const t = new Date(start + i * bucketSize);
      if (bucketSize <= 3600000) {
        labels.push(`${t.toLocaleDateString(undefined, { month: "short", day: "numeric" })} ${t.getHours()}:00`);
      } else if (bucketSize <= 3600000 * 4) {
        labels.push(`${t.toLocaleDateString(undefined, { weekday: "short" })} ${t.getHours()}:00`);
      } else {
        labels.push(t.toLocaleDateString(undefined, { month: "short", day: "numeric" }));
      }
    }

    if (searchResult?.results) {
      for (const wf of searchResult.results) {
        const wfTime = wf.startTime ? new Date(wf.startTime).getTime() : 0;
        if (wfTime < start || wfTime > now) continue;
        const idx = Math.min(Math.floor((wfTime - start) / bucketSize), totalBuckets - 1);
        if (idx >= 0) {
          counts[idx]++;
          const sm = bucketStatuses[idx];
          sm.set(wf.status, (sm.get(wf.status) ?? 0) + 1);
        }
      }
    }

    const max = Math.max(...counts, 1);

    return {
      buckets: counts,
      maxCount: max,
      cols: colCount,
      bucketLabels: labels,
      bucketStatuses,
    };
  }, [searchResult, periodPreset, customDays, customIntervalHours]);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between flex-wrap gap-2">
        <h2 className="text-2xl font-bold tracking-tight">Execution Heatmap</h2>
        <div className="flex gap-2 flex-wrap items-end">
          <div>
            <label className="text-[10px] text-muted-foreground block mb-0.5">Status</label>
            <Select value={statusFilter} onValueChange={setStatusFilter}>
              <SelectTrigger className="w-36">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All Statuses</SelectItem>
                {Object.keys(STATUS_COLORS).map((s) => (
                  <SelectItem key={s} value={s}>{s}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div>
            <label className="text-[10px] text-muted-foreground block mb-0.5">Time Period</label>
            <Select value={periodPreset} onValueChange={(v) => setPeriodPreset(v as PeriodPreset)}>
              <SelectTrigger className="w-32">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="24h">24 Hours</SelectItem>
                <SelectItem value="7d">7 Days</SelectItem>
                <SelectItem value="30d">30 Days</SelectItem>
                <SelectItem value="custom">Custom</SelectItem>
              </SelectContent>
            </Select>
          </div>
          {periodPreset === "custom" && (
            <>
              <div>
                <label className="text-[10px] text-muted-foreground block mb-0.5">Days</label>
                <Input
                  type="number"
                  min={1}
                  max={365}
                  value={customDays}
                  onChange={(e) => setCustomDays(Math.max(1, Math.min(365, Number(e.target.value) || 1)))}
                  className="w-20 h-9"
                />
              </div>
              <div>
                <label className="text-[10px] text-muted-foreground block mb-0.5">Interval (hours)</label>
                <Input
                  type="number"
                  min={1}
                  max={168}
                  value={customIntervalHours}
                  onChange={(e) => setCustomIntervalHours(Math.max(1, Math.min(168, Number(e.target.value) || 1)))}
                  className="w-24 h-9"
                />
              </div>
            </>
          )}
        </div>
      </div>

      <Card>
        <CardContent className="pt-6">
          <TooltipProvider>
            <div className="overflow-auto">
              <div
                className="grid gap-1 w-full"
                style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}
              >
                {buckets.map((count, i) => {
                  const intensity = count / maxCount;
                  const baseColor = statusFilter !== "all" ? (STATUS_COLORS[statusFilter] ?? "#3b82f6") : "#3b82f6";
                  const alpha = Math.max(0.05, intensity * 0.9);

                  return (
                    <Tooltip key={i}>
                      <TooltipTrigger asChild>
                        <div
                          className="rounded-sm border border-border/30 flex items-center justify-center text-[10px] font-medium cursor-pointer hover:ring-1 hover:ring-primary transition-all aspect-square min-h-7"
                          style={{
                            backgroundColor: count > 0 ? `${baseColor}${Math.round(alpha * 255).toString(16).padStart(2, "0")}` : undefined,
                            color: intensity > 0.5 ? "white" : undefined,
                          }}
                        >
                          {count > 0 ? count : ""}
                        </div>
                      </TooltipTrigger>
                      <TooltipContent side="top" className="text-xs">
                        <p className="font-semibold">{bucketLabels[i]}</p>
                        <p>{count} execution{count !== 1 ? "s" : ""}</p>
                      </TooltipContent>
                    </Tooltip>
                  );
                })}
              </div>
            </div>
          </TooltipProvider>

          {/* Color scale legend */}
          <div className="flex items-center gap-2 mt-4 text-xs text-muted-foreground">
            <span>Less</span>
            {[0.1, 0.3, 0.5, 0.7, 0.9].map((alpha) => (
              <div
                key={alpha}
                className="w-4 h-4 rounded-sm border border-border/30"
                style={{
                  backgroundColor: `#3b82f6${Math.round(alpha * 255).toString(16).padStart(2, "0")}`,
                }}
              />
            ))}
            <span>More</span>
          </div>

          <div className="mt-2 text-xs text-muted-foreground">
            Total: {searchResult?.totalHits ?? 0} executions in period
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
