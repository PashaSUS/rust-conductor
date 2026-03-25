import { useMemo } from "react";
import type { TaskResult } from "@/api/conductor";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";

const STATUS_COLORS: Record<string, string> = {
  COMPLETED: "#10b981",
  IN_PROGRESS: "#3b82f6",
  SCHEDULED: "#f59e0b",
  FAILED: "#ef4444",
  TIMED_OUT: "#f97316",
  CANCELED: "#9ca3af",
  SKIPPED: "#d1d5db",
};

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  const mins = Math.floor(ms / 60000);
  const secs = Math.round((ms % 60000) / 1000);
  return `${mins}m ${secs}s`;
}

interface Props {
  tasks: TaskResult[];
  workflowStartTime: number;
  workflowEndTime?: number;
}

interface FlameEntry {
  task: TaskResult;
  depth: number;
  startMs: number;
  endMs: number;
}

/**
 * #171 — Workflow execution flame chart.
 * Renders a hierarchical task duration visualization where overlapping tasks
 * stack vertically, producing a flame-chart-like view.
 */
export function FlameChart({ tasks, workflowStartTime, workflowEndTime }: Props) {
  const entries = useMemo(() => {
    if (tasks.length === 0) return [];

    const now = Date.now();
    const sorted = [...tasks].sort((a, b) => {
      const aStart = a.startTime ?? a.scheduledTime ?? workflowStartTime;
      const bStart = b.startTime ?? b.scheduledTime ?? workflowStartTime;
      return aStart - bStart;
    });

    // Assign depth based on overlap with previous tasks
    const result: FlameEntry[] = [];
    const depthEnds: number[] = []; // tracks when each depth level becomes free

    for (const task of sorted) {
      const startMs = (task.startTime ?? task.scheduledTime ?? workflowStartTime) - workflowStartTime;
      const endMs = (task.endTime ?? now) - workflowStartTime;

      // Find the shallowest depth where this task doesn't overlap
      let depth = 0;
      while (depth < depthEnds.length && depthEnds[depth] > startMs) {
        depth++;
      }
      depthEnds[depth] = endMs;
      result.push({ task, depth, startMs, endMs });
    }

    return result;
  }, [tasks, workflowStartTime, workflowEndTime]);

  if (entries.length === 0) return null;

  const now = Date.now();
  const totalDuration = Math.max((workflowEndTime ?? now) - workflowStartTime, 1);
  const maxDepth = Math.max(...entries.map((e) => e.depth), 0) + 1;
  const rowHeight = 28;
  const chartHeight = maxDepth * rowHeight + 40;

  return (
    <TooltipProvider>
      <div className="space-y-2">
        {/* Time axis */}
        <div className="flex justify-between text-[10px] text-muted-foreground px-1">
          {[0, 0.25, 0.5, 0.75, 1].map((frac) => (
            <span key={frac}>{formatDuration(totalDuration * frac)}</span>
          ))}
        </div>

        {/* Flame chart */}
        <div className="relative bg-muted rounded border overflow-hidden" style={{ height: chartHeight }}>
          {/* Grid lines */}
          {[25, 50, 75].map((pct) => (
            <div
              key={pct}
              className="absolute top-0 bottom-0 w-px bg-border"
              style={{ left: `${pct}%` }}
            />
          ))}

          {/* Flame entries */}
          {entries.map(({ task, depth, startMs, endMs }) => {
            const leftPct = (startMs / totalDuration) * 100;
            const widthPct = Math.max(((endMs - startMs) / totalDuration) * 100, 0.3);
            const color = STATUS_COLORS[task.status] ?? "#9ca3af";
            const duration = endMs - startMs;

            return (
              <Tooltip key={task.taskId}>
                <TooltipTrigger asChild>
                  <div
                    className="absolute rounded-sm cursor-pointer hover:brightness-110 transition-all flex items-center px-1 overflow-hidden"
                    style={{
                      left: `${leftPct}%`,
                      width: `${Math.min(widthPct, 100 - leftPct)}%`,
                      minWidth: 4,
                      top: depth * rowHeight + 4,
                      height: rowHeight - 4,
                      backgroundColor: color,
                    }}
                  >
                    <span className="text-[10px] text-white truncate font-medium">
                      {task.referenceTaskName}
                    </span>
                  </div>
                </TooltipTrigger>
                <TooltipContent side="top" className="text-xs">
                  <div className="space-y-0.5">
                    <p className="font-semibold">{task.referenceTaskName}</p>
                    <p>Type: {task.taskType}</p>
                    <p>Status: {task.status}</p>
                    <p>Duration: {formatDuration(duration)}</p>
                    {task.workerId && <p>Worker: {task.workerId}</p>}
                  </div>
                </TooltipContent>
              </Tooltip>
            );
          })}
        </div>

        {/* Legend */}
        <div className="flex flex-wrap gap-3 text-[10px] text-muted-foreground">
          {Object.entries(STATUS_COLORS).slice(0, 5).map(([label, color]) => (
            <span key={label} className="flex items-center gap-1">
              <span
                className="inline-block w-2.5 h-2.5 rounded-sm"
                style={{ backgroundColor: color }}
              />
              {label}
            </span>
          ))}
        </div>
      </div>
    </TooltipProvider>
  );
}
