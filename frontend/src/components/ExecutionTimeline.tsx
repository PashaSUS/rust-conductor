import type { TaskResult } from "@/api/conductor";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { useThemeText } from "@/components/ThemeContext";

const STATUS_COLORS: Record<string, string> = {
  COMPLETED: "bg-emerald-500",
  IN_PROGRESS: "bg-blue-500 animate-pulse",
  SCHEDULED: "bg-amber-400",
  FAILED: "bg-red-500",
  FAILED_WITH_TERMINAL_ERROR: "bg-red-700",
  TIMED_OUT: "bg-orange-500",
  CANCELED: "bg-gray-400",
  SKIPPED: "bg-gray-300",
};

function statusColor(status: string) {
  return STATUS_COLORS[status] ?? "bg-gray-400";
}

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

export function ExecutionTimeline({ tasks, workflowStartTime, workflowEndTime }: Props) {
  const t = useThemeText();
  if (tasks.length === 0) return null;

  const now = Date.now();
  const timelineEnd = workflowEndTime ?? now;
  const totalDuration = Math.max(timelineEnd - workflowStartTime, 1);

  // Sort tasks by sequence
  const sorted = [...tasks].sort((a, b) => a.seq - b.seq);

  return (
    <TooltipProvider>
      <div className="space-y-1">
        {/* Time axis labels */}
        <div className="flex justify-between text-[10px] text-muted-foreground px-1 mb-2">
          <span>0s</span>
          <span>{formatDuration(totalDuration / 4)}</span>
          <span>{formatDuration(totalDuration / 2)}</span>
          <span>{formatDuration((totalDuration * 3) / 4)}</span>
          <span>{formatDuration(totalDuration)}</span>
        </div>

        {sorted.map((task) => {
          const taskStart = (task.startTime ?? task.scheduledTime ?? workflowStartTime) - workflowStartTime;
          const taskEnd = (task.endTime ?? now) - workflowStartTime;
          const leftPct = Math.max((taskStart / totalDuration) * 100, 0);
          const widthPct = Math.max(((taskEnd - taskStart) / totalDuration) * 100, 0.5);
          const duration = taskEnd - taskStart;

          return (
            <Tooltip key={task.taskId}>
              <TooltipTrigger asChild>
                <div className="flex items-center gap-2 group">
                  <span className="text-[11px] text-muted-foreground w-28 truncate text-right shrink-0 group-hover:text-foreground transition-colors">
                    {task.referenceTaskName}
                  </span>
                  <div className="flex-1 h-6 relative bg-muted rounded-sm overflow-hidden">
                    {/* Grid lines */}
                    <div className="absolute inset-0 flex">
                      {[25, 50, 75].map((pct) => (
                        <div
                          key={pct}
                          className="absolute top-0 bottom-0 w-px bg-border"
                          style={{ left: `${pct}%` }}
                        />
                      ))}
                    </div>
                    {/* Task bar */}
                    <div
                      className={`absolute top-0.5 bottom-0.5 rounded-sm ${statusColor(task.status)} hover:ring-2 hover:ring-primary transition-all cursor-pointer`}
                      style={{
                        left: `${leftPct}%`,
                        width: `${Math.min(widthPct, 100 - leftPct)}%`,
                        minWidth: "3px",
                      }}
                    />
                  </div>
                  <span className="text-[10px] text-muted-foreground w-14 shrink-0">
                    {formatDuration(duration)}
                  </span>
                </div>
              </TooltipTrigger>
              <TooltipContent side="top" className="text-xs">
                <div className="space-y-1">
                  <p className="font-semibold">{task.referenceTaskName}</p>
                  <p>{t.status}: {task.status}</p>
                  <p>{t.type}: {task.taskType}</p>
                  <p>{t.duration}: {formatDuration(duration)}</p>
                  {task.workerId && <p>{t.worker}: {task.workerId}</p>}
                </div>
              </TooltipContent>
            </Tooltip>
          );
        })}

        {/* Legend */}
        <div className="flex flex-wrap gap-3 pt-3 text-[10px] text-muted-foreground">
          {[
            ["COMPLETED", "bg-emerald-500"],
            ["IN_PROGRESS", "bg-blue-500"],
            ["SCHEDULED", "bg-amber-400"],
            ["FAILED", "bg-red-500"],
          ].map(([label, color]) => (
            <span key={label} className="flex items-center gap-1">
              <span className={`inline-block w-2.5 h-2.5 rounded-sm ${color}`} />
              {label}
            </span>
          ))}
        </div>
      </div>
    </TooltipProvider>
  );
}
