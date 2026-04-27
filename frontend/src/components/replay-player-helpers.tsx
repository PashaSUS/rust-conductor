import type { TaskResult } from "@/api/conductor";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { X } from "lucide-react";

export const STATUS_COLORS: Record<string, string> = {
  COMPLETED: "#10b981",
  IN_PROGRESS: "#3b82f6",
  SCHEDULED: "#f59e0b",
  FAILED: "#ef4444",
  TIMED_OUT: "#f97316",
  CANCELED: "#9ca3af",
  SKIPPED: "#d1d5db",
};

export const STATUS_BG: Record<string, string> = {
  COMPLETED: "bg-muted border-emerald-500",
  IN_PROGRESS: "bg-muted border-blue-500",
  SCHEDULED: "bg-muted border-amber-500",
  FAILED: "bg-muted border-red-500",
  TIMED_OUT: "bg-muted border-orange-500",
  CANCELED: "bg-muted border-muted-foreground",
  SKIPPED: "bg-muted border-muted-foreground",
};

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`;
}

function formatJson(data: unknown): string {
  if (data === null || data === undefined) return "—";
  if (typeof data === "string") return data;
  try {
    return JSON.stringify(data, null, 2);
  } catch {
    return String(data);
  }
}

export interface TimeEvent {
  time: number;
  taskRef: string;
  taskType: string;
  event: "scheduled" | "started" | "completed" | "failed";
  status: string;
}

function DataSection({ label, data }: { label: string; data: unknown }) {
  const formatted = formatJson(data);
  const isEmpty = data === null || data === undefined || (typeof data === "object" && Object.keys(data as Record<string, unknown>).length === 0);

  return (
    <div>
      <span className="text-[10px] font-medium uppercase text-muted-foreground">{label}</span>
      {isEmpty ? (
        <p className="text-xs text-muted-foreground mt-0.5">—</p>
      ) : (
        <pre className="text-[11px] mt-0.5 bg-muted rounded p-2 overflow-auto max-h-32 whitespace-pre-wrap break-all font-mono leading-relaxed">
          {formatted}
        </pre>
      )}
    </div>
  );
}

export function TaskDetailPanel({
  task,
  taskStates,
  onClose,
}: {
  task: TaskResult;
  taskStates: Map<string, { status: string; event: string }>;
  onClose: () => void;
}) {
  const state = taskStates.get(task.referenceTaskName);
  const color = state ? (STATUS_COLORS[state.status] ?? "#e5e7eb") : "#e5e7eb";
  const duration = task.startTime && task.endTime ? task.endTime - task.startTime : null;

  return (
    <Card>
      <CardHeader className="pb-2 pt-3 px-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <CardTitle className="text-sm font-semibold">{task.referenceTaskName}</CardTitle>
            <span className="text-xs text-muted-foreground">{task.taskType}</span>
            {state && (
              <Badge className="text-[10px] px-1.5 py-0.5" style={{ backgroundColor: color, color: "white" }}>
                {state.status}
              </Badge>
            )}
            {duration !== null && (
              <span className="text-xs text-muted-foreground font-mono">{formatDuration(duration)}</span>
            )}
          </div>
          <Button variant="ghost" size="icon" className="h-6 w-6" onClick={onClose}>
            <X className="h-3.5 w-3.5" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="px-4 pb-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <DataSection label="Input" data={task.inputData} />
          <DataSection label="Output" data={task.outputData} />
        </div>
        {task.reasonForIncompletion && (
          <div className="mt-3">
            <span className="text-[10px] font-medium uppercase text-destructive">Reason</span>
            <p className="text-xs text-destructive mt-0.5">{task.reasonForIncompletion}</p>
          </div>
        )}
        <div className="flex gap-4 mt-3 text-[10px] text-muted-foreground">
          {task.workerId && <span>Worker: {task.workerId}</span>}
          {task.retryCount > 0 && <span>Retries: {task.retryCount}</span>}
          <span>Poll count: {task.pollCount}</span>
          <span>Seq: {task.seq}</span>
        </div>
      </CardContent>
    </Card>
  );
}

export function EventLogCard({
  events,
  currentTime,
}: {
  events: TimeEvent[];
  currentTime: number;
}) {
  const visible = events.filter((e) => e.time <= currentTime);

  return (
    <Card>
      <CardHeader className="pb-2 pt-3 px-4">
        <CardTitle className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Event Log</CardTitle>
      </CardHeader>
      <CardContent className="px-4 pb-3">
        <div className="max-h-48 overflow-auto space-y-1">
          {visible
            .reverse()
            .slice(0, 30)
            .map((ev, i) => (
              <div key={i} className="flex items-center gap-2 py-0.5 text-xs">
                <span className="text-muted-foreground font-mono w-16 text-right shrink-0">
                  {formatDuration(ev.time)}
                </span>
                <span
                  className="w-2 h-2 rounded-full shrink-0"
                  style={{ backgroundColor: STATUS_COLORS[ev.status] ?? "#9ca3af" }}
                />
                <span className="font-medium truncate">{ev.taskRef}</span>
                <Badge variant="outline" className="text-[10px] px-1.5 py-0 shrink-0">
                  {ev.event}
                </Badge>
              </div>
            ))}
          {visible.length === 0 && (
            <p className="text-muted-foreground text-xs py-2 text-center">No events yet — press play or step forward</p>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
