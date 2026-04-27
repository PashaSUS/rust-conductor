import { useState, useEffect } from "react";
import { useNavigate } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { workflowApi, formatTs, type TaskResult, type WorkflowCheckpoint } from "@/api/conductor";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { CopyButton } from "@/components/CopyButton";
import { Clock, RotateCcw } from "lucide-react";

export function InfoRow({ label, value, copyable }: { label: string; value: string; copyable?: boolean }) {
  return (
    <div className="flex items-center">
      <span className="font-medium text-muted-foreground w-36">{label}</span>
      <span className="font-mono text-xs break-all">{value}</span>
      {copyable && value && value !== "\u2014" && <CopyButton value={value} className="ml-1" />}
    </div>
  );
}

export function DurationBadge({ startTime, endTime, status }: { startTime: number; endTime?: number; status: string }) {
  const [, setTick] = useState(0);

  useEffect(() => {
    if (status !== "RUNNING" && status !== "PAUSED") return;
    const interval = setInterval(() => setTick((t) => t + 1), 1000);
    return () => clearInterval(interval);
  }, [status]);

  const end = endTime ?? Date.now();
  const ms = Math.max(end - startTime, 0);
  const secs = Math.floor(ms / 1000) % 60;
  const mins = Math.floor(ms / 60000) % 60;
  const hrs = Math.floor(ms / 3600000);

  const formatted = hrs > 0 ? `${hrs}h ${mins}m ${secs}s` : mins > 0 ? `${mins}m ${secs}s` : `${secs}s`;

  return (
    <span className="text-xs text-muted-foreground flex items-center gap-1">
      <Clock className="h-3 w-3" />
      {formatted}
    </span>
  );
}

export function TaskProgressBar({ tasks }: { tasks: TaskResult[] }) {
  const total = tasks.length;
  const completed = tasks.filter((t) => t.status === "COMPLETED").length;
  const failed = tasks.filter((t) => t.status === "FAILED" || t.status === "FAILED_WITH_TERMINAL_ERROR").length;
  const inProgress = tasks.filter((t) => t.status === "IN_PROGRESS").length;
  const pctComplete = Math.round((completed / total) * 100);
  const pctFailed = Math.round((failed / total) * 100);
  const pctInProgress = Math.round((inProgress / total) * 100);

  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <span>{completed}/{total} tasks completed</span>
        <span>{pctComplete}%</span>
      </div>
      <div className="flex h-2 rounded-full overflow-hidden bg-muted">
        {pctComplete > 0 && <div className="bg-emerald-500 transition-all duration-500" style={{ width: `${pctComplete}%` }} />}
        {pctInProgress > 0 && <div className="bg-blue-500 animate-pulse transition-all duration-500" style={{ width: `${pctInProgress}%` }} />}
        {pctFailed > 0 && <div className="bg-red-500 transition-all duration-500" style={{ width: `${pctFailed}%` }} />}
      </div>
    </div>
  );
}

export function CheckpointsTab({ workflowId, isRunning }: { workflowId: string; isRunning: boolean }) {
  const navigate = useNavigate();
  const { data: checkpoints, isLoading, refetch } = useQuery({
    queryKey: ["checkpoints", workflowId],
    queryFn: () => workflowApi.listCheckpoints(workflowId),
  });

  const handleCreate = async () => {
    const label = prompt("Checkpoint label (optional):");
    try {
      await workflowApi.createCheckpoint(workflowId, label || undefined);
      refetch();
    } catch (e) {
      alert(`Failed to create checkpoint: ${e instanceof Error ? e.message : e}`);
    }
  };

  const handleRestore = async (cpId: string) => {
    if (!confirm("This will terminate the current execution and start a new one from this checkpoint. Continue?")) return;
    try {
      const newId = await workflowApi.restoreCheckpoint(workflowId, cpId);
      navigate(`/executions/${newId}`);
    } catch (e) {
      alert(`Failed to restore: ${e instanceof Error ? e.message : e}`);
    }
  };

  return (
    <Card>
      <CardContent className="pt-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-semibold">Workflow Checkpoints</h3>
          {isRunning && (
            <Button variant="outline" size="sm" onClick={handleCreate}>
              Create Checkpoint
            </Button>
          )}
        </div>
        {isLoading ? (
          <div className="text-sm text-muted-foreground">Loading...</div>
        ) : !checkpoints?.length ? (
          <div className="text-sm text-muted-foreground">No checkpoints</div>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Checkpoint ID</TableHead>
                <TableHead>Label</TableHead>
                <TableHead>Created At</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {checkpoints.map((cp: WorkflowCheckpoint) => (
                <TableRow key={cp.checkpointId}>
                  <TableCell className="font-mono text-xs">{cp.checkpointId.substring(0, 8)}...</TableCell>
                  <TableCell>{cp.label || "\u2014"}</TableCell>
                  <TableCell>{formatTs(cp.createdAt)}</TableCell>
                  <TableCell className="text-right">
                    <Button variant="ghost" size="sm" onClick={() => handleRestore(cp.checkpointId)}>
                      <RotateCcw className="h-3 w-3 mr-1" /> Restore
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </CardContent>
    </Card>
  );
}
