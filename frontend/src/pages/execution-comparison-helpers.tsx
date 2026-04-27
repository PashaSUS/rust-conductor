import { Card, CardContent } from "@/components/ui/card";
import { StatusBadge } from "@/components/StatusBadge";
import { formatTs, type Workflow } from "@/api/conductor";

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`;
}

export function ExecutionSummaryCard({ wf, label }: { wf: Workflow; label: string }) {
  const duration = (wf.endTime ?? Date.now()) - wf.startTime;
  return (
    <Card>
      <CardContent className="pt-4 space-y-1">
        <div className="flex items-center justify-between">
          <span className="text-xs text-muted-foreground">{label}</span>
          <StatusBadge status={wf.status} />
        </div>
        <h4 className="font-semibold text-sm">{wf.workflowName} v{wf.workflowVersion}</h4>
        <p className="text-xs text-muted-foreground font-mono">{wf.workflowId}</p>
        <div className="flex gap-4 text-xs text-muted-foreground mt-2">
          <span>Duration: {formatDuration(duration)}</span>
          <span>Tasks: {wf.tasks.length}</span>
          <span>Started: {formatTs(wf.startTime)}</span>
        </div>
      </CardContent>
    </Card>
  );
}

export function TimelineOverlay({ left, right }: { left: Workflow; right: Workflow }) {
  const allRefs = new Set<string>();
  left.tasks.forEach((t) => allRefs.add(t.referenceTaskName));
  right.tasks.forEach((t) => allRefs.add(t.referenceTaskName));

  const leftMap = new Map(left.tasks.map((t) => [t.referenceTaskName, t]));
  const rightMap = new Map(right.tasks.map((t) => [t.referenceTaskName, t]));

  const maxDurationL = Math.max((left.endTime ?? Date.now()) - left.startTime, 1);
  const maxDurationR = Math.max((right.endTime ?? Date.now()) - right.startTime, 1);
  const maxDuration = Math.max(maxDurationL, maxDurationR);

  const refs = [...allRefs];

  return (
    <div className="space-y-1">
      <div className="flex justify-between text-[10px] text-muted-foreground px-1 mb-2">
        <span>0s</span>
        <span>{formatDuration(maxDuration / 2)}</span>
        <span>{formatDuration(maxDuration)}</span>
      </div>
      {refs.map((refName) => {
        const lt = leftMap.get(refName);
        const rt = rightMap.get(refName);

        const lStart = lt ? ((lt.startTime ?? lt.scheduledTime ?? left.startTime) - left.startTime) : 0;
        const lEnd = lt ? ((lt.endTime ?? Date.now()) - left.startTime) : 0;
        const rStart = rt ? ((rt.startTime ?? rt.scheduledTime ?? right.startTime) - right.startTime) : 0;
        const rEnd = rt ? ((rt.endTime ?? Date.now()) - right.startTime) : 0;

        return (
          <div key={refName} className="flex items-center gap-2 group">
            <span className="text-[11px] text-muted-foreground w-28 truncate text-right shrink-0">
              {refName}
            </span>
            <div className="flex-1 h-5 relative bg-muted rounded-sm overflow-hidden">
              {/* Grid line */}
              <div className="absolute top-0 bottom-0 w-px bg-border" style={{ left: "50%" }} />
              {/* Left bar (blue) */}
              {lt && (
                <div
                  className="absolute top-0 h-2 rounded-sm bg-blue-500"
                  style={{
                    left: `${(lStart / maxDuration) * 100}%`,
                    width: `${Math.max(((lEnd - lStart) / maxDuration) * 100, 0.5)}%`,
                  }}
                />
              )}
              {/* Right bar (amber) */}
              {rt && (
                <div
                  className="absolute bottom-0 h-2 rounded-sm bg-amber-500"
                  style={{
                    left: `${(rStart / maxDuration) * 100}%`,
                    width: `${Math.max(((rEnd - rStart) / maxDuration) * 100, 0.5)}%`,
                  }}
                />
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
