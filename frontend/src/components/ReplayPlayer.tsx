import { useState, useEffect, useCallback, useRef } from "react";
import type { TaskResult } from "@/api/conductor";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Play, Pause, SkipForward, SkipBack, RotateCcw, Clock, Zap, X } from "lucide-react";

const STATUS_COLORS: Record<string, string> = {
  COMPLETED: "#10b981",
  IN_PROGRESS: "#3b82f6",
  SCHEDULED: "#f59e0b",
  FAILED: "#ef4444",
  TIMED_OUT: "#f97316",
  CANCELED: "#9ca3af",
  SKIPPED: "#d1d5db",
};

const STATUS_BG: Record<string, string> = {
  COMPLETED: "bg-muted border-emerald-500",
  IN_PROGRESS: "bg-muted border-blue-500",
  SCHEDULED: "bg-muted border-amber-500",
  FAILED: "bg-muted border-red-500",
  TIMED_OUT: "bg-muted border-orange-500",
  CANCELED: "bg-muted border-muted-foreground",
  SKIPPED: "bg-muted border-muted-foreground",
};

function formatDuration(ms: number): string {
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

interface Props {
  tasks: TaskResult[];
  workflowStartTime: number;
  workflowEndTime?: number;
}

interface TimeEvent {
  time: number;
  taskRef: string;
  taskType: string;
  event: "scheduled" | "started" | "completed" | "failed";
  status: string;
}

/**
 * #177 — Execution replay animation.
 * Steps through workflow execution frame by frame, showing task state
 * transitions over time with play/pause controls and a scrubber.
 */
export function ReplayPlayer({ tasks, workflowStartTime, workflowEndTime }: Props) {
  const [currentTime, setCurrentTime] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [speed, setSpeed] = useState(1);
  const [expandedTask, setExpandedTask] = useState<string | null>(null);
  const animRef = useRef<number>(0);
  const lastTickRef = useRef<number>(0);

  const now = Date.now();
  const totalDuration = Math.max((workflowEndTime ?? now) - workflowStartTime, 1);

  // Build timeline events
  const events: TimeEvent[] = [];
  for (const t of tasks) {
    if (t.scheduledTime) {
      events.push({
        time: t.scheduledTime - workflowStartTime,
        taskRef: t.referenceTaskName,
        taskType: t.taskType,
        event: "scheduled",
        status: "SCHEDULED",
      });
    }
    if (t.startTime) {
      events.push({
        time: t.startTime - workflowStartTime,
        taskRef: t.referenceTaskName,
        taskType: t.taskType,
        event: "started",
        status: "IN_PROGRESS",
      });
    }
    if (t.endTime) {
      const terminal = t.status === "FAILED" || t.status === "TIMED_OUT" ? "failed" : "completed";
      events.push({
        time: t.endTime - workflowStartTime,
        taskRef: t.referenceTaskName,
        taskType: t.taskType,
        event: terminal,
        status: t.status,
      });
    }
  }
  events.sort((a, b) => a.time - b.time);

  // Current state of each task at currentTime
  const taskStates = new Map<string, { status: string; event: string }>();
  for (const ev of events) {
    if (ev.time <= currentTime) {
      taskStates.set(ev.taskRef, { status: ev.status, event: ev.event });
    }
  }

  // Active events (just happened in last 2% window)
  const recentWindow = totalDuration * 0.02;
  const recentEvents = events.filter((e) => e.time <= currentTime && e.time > currentTime - recentWindow);

  // Progress stats
  const tasksCompleted = [...taskStates.values()].filter((s) => s.status === "COMPLETED" || s.status === "FAILED" || s.status === "TIMED_OUT" || s.status === "CANCELED").length;
  const progress = Math.round((currentTime / totalDuration) * 100);

  // Animation loop
  const tick = useCallback(
    (ts: number) => {
      if (!lastTickRef.current) lastTickRef.current = ts;
      const delta = (ts - lastTickRef.current) * speed;
      lastTickRef.current = ts;

      setCurrentTime((prev) => {
        const next = prev + delta;
        if (next >= totalDuration) {
          setPlaying(false);
          return totalDuration;
        }
        return next;
      });
      animRef.current = requestAnimationFrame(tick);
    },
    [speed, totalDuration]
  );

  useEffect(() => {
    if (playing) {
      lastTickRef.current = 0;
      animRef.current = requestAnimationFrame(tick);
    } else {
      cancelAnimationFrame(animRef.current);
    }
    return () => cancelAnimationFrame(animRef.current);
  }, [playing, tick]);

  const stepForward = () => {
    const next = events.find((e) => e.time > currentTime);
    if (next) setCurrentTime(next.time);
    else setCurrentTime(totalDuration);
  };

  const stepBack = () => {
    const prev = [...events].reverse().find((e) => e.time < currentTime - 1);
    if (prev) setCurrentTime(prev.time);
    else setCurrentTime(0);
  };

  const reset = () => {
    setPlaying(false);
    setCurrentTime(0);
    setExpandedTask(null);
  };

  return (
    <div className="space-y-5">
      {/* Controls Card */}
      <Card>
        <CardContent className="pt-4 pb-4 space-y-3">
          <div className="flex items-center gap-3 flex-wrap">
            {/* Transport controls */}
            <div className="flex items-center gap-1 bg-muted rounded-lg p-1">
              <Button variant="ghost" size="icon" className="h-8 w-8" onClick={reset} title="Reset">
                <RotateCcw className="h-3.5 w-3.5" />
              </Button>
              <Button variant="ghost" size="icon" className="h-8 w-8" onClick={stepBack} title="Previous event">
                <SkipBack className="h-3.5 w-3.5" />
              </Button>
              <Button
                variant={playing ? "default" : "outline"}
                size="icon"
                className="h-9 w-9"
                onClick={() => setPlaying(!playing)}
                title={playing ? "Pause" : "Play"}
              >
                {playing ? <Pause className="h-4 w-4" /> : <Play className="h-4 w-4 ml-0.5" />}
              </Button>
              <Button variant="ghost" size="icon" className="h-8 w-8" onClick={stepForward} title="Next event">
                <SkipForward className="h-3.5 w-3.5" />
              </Button>
            </div>

            {/* Speed selector */}
            <div className="flex items-center gap-1">
              <Zap className="h-3 w-3 text-muted-foreground" />
              {[0.5, 1, 2, 5, 10].map((s) => (
                <Button
                  key={s}
                  variant={speed === s ? "default" : "ghost"}
                  size="sm"
                  className="h-7 px-2 text-xs"
                  onClick={() => setSpeed(s)}
                >
                  {s}x
                </Button>
              ))}
            </div>

            {/* Time + progress */}
            <div className="ml-auto flex items-center gap-3 text-xs text-muted-foreground">
              <div className="flex items-center gap-1">
                <Clock className="h-3 w-3" />
                <span className="font-mono">{formatDuration(currentTime)}</span>
                <span>/</span>
                <span className="font-mono">{formatDuration(totalDuration)}</span>
              </div>
              <Badge variant="secondary" className="gap-1">
                {tasksCompleted}/{tasks.length} tasks
              </Badge>
              <Badge variant="outline" className="font-mono">{progress}%</Badge>
            </div>
          </div>

          {/* Scrubber */}
          <input
            type="range"
            value={currentTime}
            min={0}
            max={totalDuration}
            step={Math.max(1, totalDuration / 1000)}
            onChange={(e) => setCurrentTime(Number(e.target.value))}
            className="w-full cursor-pointer accent-primary h-2"
          />

          {/* Progress bar with event markers */}
          <div className="relative h-8 bg-muted rounded-lg border overflow-hidden">
            <div
              className="absolute inset-y-0 left-0 bg-accent transition-none"
              style={{ width: `${progress}%` }}
            />
            {events.map((ev, i) => (
              <div
                key={i}
                className="absolute top-1 bottom-1 w-0.5 rounded-full"
                style={{
                  left: `${(ev.time / totalDuration) * 100}%`,
                  backgroundColor: STATUS_COLORS[ev.status] ?? "#9ca3af",
                  opacity: ev.time <= currentTime ? 1 : 0.25,
                }}
              />
            ))}
            {/* Playhead */}
            <div
              className="absolute top-0 bottom-0 w-0.5 bg-primary z-10"
              style={{ left: `${progress}%` }}
            >
              <div className="absolute -top-1 -left-1.5 w-3.5 h-3.5 rounded-full bg-primary border-2 border-background" />
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Task state visualization */}
      <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-3">
        {tasks.map((t) => {
          const state = taskStates.get(t.referenceTaskName);
          const statusClass = state ? (STATUS_BG[state.status] ?? "bg-muted border-muted-foreground") : "bg-muted border-muted-foreground";
          const color = state ? (STATUS_COLORS[state.status] ?? "#e5e7eb") : "#e5e7eb";
          const isRecent = recentEvents.some((e) => e.taskRef === t.referenceTaskName);
          const isSelected = expandedTask === t.referenceTaskName;
          const duration = t.startTime && t.endTime ? t.endTime - t.startTime : null;

          return (
            <div
              key={t.taskId}
              className={`rounded-lg border p-3 transition-all cursor-pointer ${statusClass} ${isSelected ? "ring-2 ring-primary shadow-md" : ""} ${isRecent && !isSelected ? "ring-2 ring-primary/50 shadow-sm scale-[1.01]" : "hover:shadow-sm"}`}
              onClick={() => setExpandedTask(isSelected ? null : t.referenceTaskName)}
            >
              <div className="flex items-start justify-between gap-2">
                <div className="min-w-0 flex-1">
                  <span className="font-semibold text-sm truncate block">{t.referenceTaskName}</span>
                  <span className="text-xs text-muted-foreground">{t.taskType}</span>
                </div>
                <div className="flex flex-col items-end gap-1 shrink-0">
                  {state ? (
                    <Badge
                      className="text-[10px] px-1.5 py-0.5 font-medium"
                      style={{ backgroundColor: color, color: "white" }}
                    >
                      {state.status}
                    </Badge>
                  ) : (
                    <Badge variant="outline" className="text-[10px] px-1.5 py-0.5">PENDING</Badge>
                  )}
                  {duration !== null && (
                    <span className="text-[10px] text-muted-foreground font-mono">{formatDuration(duration)}</span>
                  )}
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Selected task detail panel — shown below the grid */}
      {expandedTask && (() => {
        const selectedTask = tasks.find((t) => t.referenceTaskName === expandedTask);
        if (!selectedTask) return null;
        const state = taskStates.get(expandedTask);
        const color = state ? (STATUS_COLORS[state.status] ?? "#e5e7eb") : "#e5e7eb";
        const duration = selectedTask.startTime && selectedTask.endTime ? selectedTask.endTime - selectedTask.startTime : null;
        return (
          <Card>
            <CardHeader className="pb-2 pt-3 px-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <CardTitle className="text-sm font-semibold">{selectedTask.referenceTaskName}</CardTitle>
                  <span className="text-xs text-muted-foreground">{selectedTask.taskType}</span>
                  {state && (
                    <Badge className="text-[10px] px-1.5 py-0.5" style={{ backgroundColor: color, color: "white" }}>
                      {state.status}
                    </Badge>
                  )}
                  {duration !== null && (
                    <span className="text-xs text-muted-foreground font-mono">{formatDuration(duration)}</span>
                  )}
                </div>
                <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => setExpandedTask(null)}>
                  <X className="h-3.5 w-3.5" />
                </Button>
              </div>
            </CardHeader>
            <CardContent className="px-4 pb-4">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <DataSection label="Input" data={selectedTask.inputData} />
                <DataSection label="Output" data={selectedTask.outputData} />
              </div>
              {selectedTask.reasonForIncompletion && (
                <div className="mt-3">
                  <span className="text-[10px] font-medium uppercase text-destructive">Reason</span>
                  <p className="text-xs text-destructive mt-0.5">{selectedTask.reasonForIncompletion}</p>
                </div>
              )}
              <div className="flex gap-4 mt-3 text-[10px] text-muted-foreground">
                {selectedTask.workerId && <span>Worker: {selectedTask.workerId}</span>}
                {selectedTask.retryCount > 0 && <span>Retries: {selectedTask.retryCount}</span>}
                <span>Poll count: {selectedTask.pollCount}</span>
                <span>Seq: {selectedTask.seq}</span>
              </div>
            </CardContent>
          </Card>
        );
      })()}

      {/* Event log */}
      <Card>
        <CardHeader className="pb-2 pt-3 px-4">
          <CardTitle className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Event Log</CardTitle>
        </CardHeader>
        <CardContent className="px-4 pb-3">
          <div className="max-h-48 overflow-auto space-y-1">
            {events
              .filter((e) => e.time <= currentTime)
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
            {events.filter((e) => e.time <= currentTime).length === 0 && (
              <p className="text-muted-foreground text-xs py-2 text-center">No events yet — press play or step forward</p>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
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
