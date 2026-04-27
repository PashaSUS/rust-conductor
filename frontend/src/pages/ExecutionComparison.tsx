import { useState, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { workflowApi, type TaskResult } from "@/api/conductor";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { TaskStatusBadge } from "@/components/StatusBadge";
import { ExecutionTimeline } from "@/components/ExecutionTimeline";
import { JsonView } from "@/components/JsonView";
import { TaskOutputDiff } from "@/components/TaskOutputDiff";
import { SearchableSelect } from "@/components/SearchableSelect";
import { Search } from "lucide-react";
import { formatDuration, ExecutionSummaryCard, TimelineOverlay } from "./execution-comparison-helpers";

/**
 * #173 — Execution comparison view.
 * Side-by-side comparison of two workflow executions, highlighting
 * differences in task durations, statuses, and I/O.
 */
export default function ExecutionComparison() {
  const [leftId, setLeftId] = useState("");
  const [rightId, setRightId] = useState("");
  const [fetchIds, setFetchIds] = useState<{ left: string; right: string } | null>(null);

  const { data: recentExecs } = useQuery({
    queryKey: ["recentExecutions"],
    queryFn: () => workflowApi.search({ size: 100 }),
  });

  const executionOptions = useMemo(() => {
    if (!recentExecs?.results) return [];
    return recentExecs.results.map((wf) => ({
      label: `${wf.workflowType} · ${wf.status} · ${wf.workflowId.slice(0, 12)}…`,
      value: wf.workflowId,
    }));
  }, [recentExecs]);

  const { data: leftWf } = useQuery({
    queryKey: ["workflow", fetchIds?.left],
    queryFn: () => workflowApi.get(fetchIds!.left),
    enabled: !!fetchIds?.left,
  });

  const { data: rightWf } = useQuery({
    queryKey: ["workflow", fetchIds?.right],
    queryFn: () => workflowApi.get(fetchIds!.right),
    enabled: !!fetchIds?.right,
  });

  const compare = () => {
    const l = leftId.trim();
    const r = rightId.trim();
    if (l && r) {
      setFetchIds({ left: l, right: r });
    }
  };

  // Build task comparison rows
  const taskPairs: Array<{ refName: string; left?: TaskResult; right?: TaskResult }> = [];
  if (leftWf && rightWf) {
    const allRefs = new Set<string>();
    leftWf.tasks.forEach((t) => allRefs.add(t.referenceTaskName));
    rightWf.tasks.forEach((t) => allRefs.add(t.referenceTaskName));

    const leftMap = new Map(leftWf.tasks.map((t) => [t.referenceTaskName, t]));
    const rightMap = new Map(rightWf.tasks.map((t) => [t.referenceTaskName, t]));

    for (const ref of allRefs) {
      taskPairs.push({ refName: ref, left: leftMap.get(ref), right: rightMap.get(ref) });
    }
  }

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-bold tracking-tight">Execution Comparison</h2>

      {/* ID input */}
      <Card>
        <CardContent className="pt-6">
          <p className="text-xs text-muted-foreground mb-3">Pick from recent executions or paste a workflow ID manually.</p>
          <div className="flex gap-2 items-end">
            <div className="flex-1 space-y-2">
              <label className="text-xs text-muted-foreground block">Left Execution</label>
              <SearchableSelect
                options={executionOptions}
                value={leftId}
                onChange={setLeftId}
                placeholder="Select execution..."
                searchPlaceholder="Search by type, status, or ID..."
              />
              <Input
                value={leftId}
                onChange={(e) => setLeftId(e.target.value)}
                placeholder="...or paste workflow ID directly"
                className="text-xs"
              />
            </div>
            <div className="flex-1 space-y-2">
              <label className="text-xs text-muted-foreground block">Right Execution</label>
              <SearchableSelect
                options={executionOptions}
                value={rightId}
                onChange={setRightId}
                placeholder="Select execution..."
                searchPlaceholder="Search by type, status, or ID..."
              />
              <Input
                value={rightId}
                onChange={(e) => setRightId(e.target.value)}
                placeholder="...or paste workflow ID directly"
                className="text-xs"
              />
            </div>
            <Button onClick={compare} disabled={!leftId.trim() || !rightId.trim()} className="shrink-0">
              <Search className="h-4 w-4 mr-1" />
              Compare
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Side-by-side summary */}
      {leftWf && rightWf && (
        <>
          <div className="grid grid-cols-2 gap-4">
            <ExecutionSummaryCard wf={leftWf} label="Left" />
            <ExecutionSummaryCard wf={rightWf} label="Right" />
          </div>

          {/* Timeline overlay */}
          <Card>
            <CardContent className="pt-6 space-y-4">
              <div className="flex items-center justify-between">
                <h3 className="font-semibold text-sm">Timeline Comparison</h3>
                <div className="flex items-center gap-1">
                  <Badge variant="outline" className="gap-1 text-[10px]">
                    <span className="w-2 h-2 rounded-full bg-blue-500 inline-block" /> Left
                  </Badge>
                  <Badge variant="outline" className="gap-1 text-[10px]">
                    <span className="w-2 h-2 rounded-full bg-amber-500 inline-block" /> Right
                  </Badge>
                </div>
              </div>
              <TimelineOverlay left={leftWf} right={rightWf} />
              <div className="border-t pt-4 space-y-3">
                <p className="text-xs text-muted-foreground font-medium">Individual Timelines</p>
                <div className="space-y-2">
                  <p className="text-xs text-muted-foreground">Left execution</p>
                  <ExecutionTimeline tasks={leftWf.tasks} workflowStartTime={leftWf.startTime} workflowEndTime={leftWf.endTime} />
                </div>
                <div className="space-y-2">
                  <p className="text-xs text-muted-foreground">Right execution</p>
                  <ExecutionTimeline tasks={rightWf.tasks} workflowStartTime={rightWf.startTime} workflowEndTime={rightWf.endTime} />
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Task-by-task comparison */}
          <Card>
            <CardContent className="pt-6">
              <h3 className="font-semibold text-sm mb-3">Task Comparison</h3>
              <div className="divide-y">
                {taskPairs.map(({ refName, left, right }) => {
                  const leftDur = left ? (left.endTime ?? Date.now()) - (left.startTime ?? left.scheduledTime ?? 0) : 0;
                  const rightDur = right ? (right.endTime ?? Date.now()) - (right.startTime ?? right.scheduledTime ?? 0) : 0;
                  const diff = leftDur - rightDur;
                  const statusDiffer = left?.status !== right?.status;

                  return (
                    <div key={refName} className="py-2 grid grid-cols-[1fr_1fr_1fr_auto] gap-3 items-center text-xs">
                      <div className="font-medium">{refName}</div>
                      <div className="flex items-center gap-2">
                        {left ? (
                          <>
                            <TaskStatusBadge status={left.status} />
                            <span className="text-muted-foreground">{formatDuration(leftDur)}</span>
                          </>
                        ) : (
                          <span className="text-muted-foreground">—</span>
                        )}
                      </div>
                      <div className="flex items-center gap-2">
                        {right ? (
                          <>
                            <TaskStatusBadge status={right.status} />
                            <span className="text-muted-foreground">{formatDuration(rightDur)}</span>
                          </>
                        ) : (
                          <span className="text-muted-foreground">—</span>
                        )}
                      </div>
                      <div className="w-20 text-right">
                        {left && right && (
                          <Badge
                            variant={statusDiffer ? "destructive" : Math.abs(diff) > 1000 ? "secondary" : "outline"}
                            className="text-[10px]"
                          >
                            {diff > 0 ? `+${formatDuration(diff)}` : diff < 0 ? `-${formatDuration(-diff)}` : "same"}
                          </Badge>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </CardContent>
          </Card>

          {/* Input/Output diff */}
          <div className="grid grid-cols-2 gap-4">
            <Card>
              <CardContent className="pt-6">
                <h4 className="text-xs font-semibold text-muted-foreground mb-2">Left Input</h4>
                <JsonView data={leftWf.input} maxHeight="12rem" />
                <h4 className="text-xs font-semibold text-muted-foreground mb-2 mt-4">Left Output</h4>
                <JsonView data={leftWf.output} maxHeight="12rem" />
              </CardContent>
            </Card>
            <Card>
              <CardContent className="pt-6">
                <h4 className="text-xs font-semibold text-muted-foreground mb-2">Right Input</h4>
                <JsonView data={rightWf.input} maxHeight="12rem" />
                <h4 className="text-xs font-semibold text-muted-foreground mb-2 mt-4">Right Output</h4>
                <JsonView data={rightWf.output} maxHeight="12rem" />
              </CardContent>
            </Card>
          </div>

          {/* Task output diff viewer */}
          <Card>
            <CardContent className="pt-6">
              <h3 className="font-semibold text-sm mb-3">Task Output Diff</h3>
              <TaskOutputDiff leftTasks={leftWf.tasks} rightTasks={rightWf.tasks} />
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}
