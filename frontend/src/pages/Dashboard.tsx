import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "react-router";
import { workflowApi, taskApi, healthApi, metadataApi } from "@/api/conductor";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Activity,
  CheckCircle2,
  XCircle,
  Clock,
  FileCode2,
  ListChecks,
  Server,
  Plus,
  ArrowRight,
} from "lucide-react";
import { StartWorkflowDialog } from "@/components/StartWorkflowDialog";
import { RelativeTime } from "@/components/RelativeTime";

export default function Dashboard() {
  const navigate = useNavigate();
  const [startOpen, setStartOpen] = useState(false);

  const healthQ = useQuery({ queryKey: ["health"], queryFn: healthApi.check });
  const statsQ = useQuery({ queryKey: ["workflow-stats"], queryFn: workflowApi.stats });
  const recentQ = useQuery({
    queryKey: ["workflows-recent"],
    queryFn: () => workflowApi.search({ size: 8 }),
  });
  const queueQ = useQuery({ queryKey: ["queue-sizes"], queryFn: taskApi.queueSizes });
  const wfDefsQ = useQuery({ queryKey: ["wf-defs"], queryFn: metadataApi.listWorkflowDefs });
  const taskDefsQ = useQuery({ queryKey: ["task-defs"], queryFn: metadataApi.listTaskDefs });

  const stats = statsQ.data ?? {};
  const running = stats["RUNNING"] ?? 0;
  const completed = stats["COMPLETED"] ?? 0;
  const failed = stats["FAILED"] ?? 0;
  const queueTotal = Object.values(queueQ.data ?? {}).reduce((a, b) => a + b, 0);
  const recent = recentQ.data?.results ?? [];

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Dashboard</h2>
          <p className="text-muted-foreground">Rust Conductor orchestration overview</p>
        </div>
        <div className="flex items-center gap-2">
          <Button size="sm" onClick={() => setStartOpen(true)}>
            <Plus className="h-4 w-4 mr-1" />
            Start Workflow
          </Button>
          <Badge variant={healthQ.data ? "success" : "destructive"} className="gap-1">
            <Server className="h-3 w-3" />
            {healthQ.data ? "Healthy" : "Offline"}
          </Badge>
        </div>
      </div>

      {/* Stats cards */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <StatCard icon={Activity} label="Running" value={running} color="text-blue-500" barColor="bg-blue-500" />
        <StatCard icon={CheckCircle2} label="Completed" value={completed} color="text-emerald-500" barColor="bg-emerald-500" />
        <StatCard icon={XCircle} label="Failed" value={failed} color="text-red-500" barColor="bg-red-500" />
        <StatCard icon={Clock} label="Queued Tasks" value={queueTotal} color="text-amber-500" barColor="bg-amber-500" />
      </div>

      {/* Status distribution bar */}
      {(running + completed + failed) > 0 && (
        <Card>
          <CardContent className="pt-6">
            <p className="text-xs font-medium text-muted-foreground mb-2">Workflow Status Distribution</p>
            <div className="flex h-3 rounded-full overflow-hidden bg-muted">
              {running > 0 && (
                <div
                  className="bg-blue-500 transition-all duration-500"
                  style={{ width: `${(running / (running + completed + failed)) * 100}%` }}
                  title={`Running: ${running}`}
                />
              )}
              {completed > 0 && (
                <div
                  className="bg-emerald-500 transition-all duration-500"
                  style={{ width: `${(completed / (running + completed + failed)) * 100}%` }}
                  title={`Completed: ${completed}`}
                />
              )}
              {failed > 0 && (
                <div
                  className="bg-red-500 transition-all duration-500"
                  style={{ width: `${(failed / (running + completed + failed)) * 100}%` }}
                  title={`Failed: ${failed}`}
                />
              )}
            </div>
            <div className="flex gap-4 mt-2 text-[10px] text-muted-foreground">
              <span className="flex items-center gap-1"><span className="w-2 h-2 rounded-full bg-blue-500 inline-block" /> Running ({running})</span>
              <span className="flex items-center gap-1"><span className="w-2 h-2 rounded-full bg-emerald-500 inline-block" /> Completed ({completed})</span>
              <span className="flex items-center gap-1"><span className="w-2 h-2 rounded-full bg-red-500 inline-block" /> Failed ({failed})</span>
            </div>
          </CardContent>
        </Card>
      )}

      <div className="grid gap-4 md:grid-cols-2">
        <Card className="cursor-pointer hover:bg-muted/30 transition-colors" onClick={() => navigate("/definitions")}>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <FileCode2 className="h-4 w-4" /> Workflow Definitions
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold">{wfDefsQ.data?.length ?? 0}</div>
            <p className="text-xs text-muted-foreground mt-1">registered definitions</p>
          </CardContent>
        </Card>
        <Card className="cursor-pointer hover:bg-muted/30 transition-colors" onClick={() => navigate("/taskdefs")}>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <ListChecks className="h-4 w-4" /> Task Definitions
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold">{taskDefsQ.data?.length ?? 0}</div>
            <p className="text-xs text-muted-foreground mt-1">registered task types</p>
          </CardContent>
        </Card>
      </div>

      {/* Recent executions */}
      {recent.length > 0 && (
        <Card>
          <CardHeader className="flex flex-row items-center justify-between">
            <CardTitle className="text-sm font-medium">Recent Executions</CardTitle>
            <Button variant="ghost" size="sm" onClick={() => navigate("/executions")} className="text-xs">
              View All <ArrowRight className="h-3 w-3 ml-1" />
            </Button>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {recent.slice(0, 8).map((wf) => (
                <div
                  key={wf.workflowId}
                  className="flex items-center justify-between rounded-lg border p-3 text-sm cursor-pointer hover:bg-muted/50 transition-colors"
                  onClick={() => navigate(`/executions/${wf.workflowId}`)}
                >
                  <div>
                    <span className="font-medium">{wf.workflowType}</span>
                    <span className="text-muted-foreground ml-2 text-xs">
                      v{wf.version}
                    </span>
                  </div>
                  <div className="flex items-center gap-3">
                    <RelativeTime value={wf.startTime} className="text-xs text-muted-foreground" />
                    <StatusBadge status={wf.status} />
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Quick start empty state */}
      {recent.length === 0 && (wfDefsQ.data?.length ?? 0) > 0 && (
        <Card>
          <CardContent className="py-8 text-center">
            <p className="text-muted-foreground text-sm mb-3">
              No recent executions. Start your first workflow!
            </p>
            <Button onClick={() => setStartOpen(true)}>
              <Plus className="h-4 w-4 mr-1" />
              Start Workflow
            </Button>
          </CardContent>
        </Card>
      )}

      <StartWorkflowDialog open={startOpen} onOpenChange={setStartOpen} />
    </div>
  );
}

function StatCard({
  icon: Icon,
  label,
  value,
  color,
  barColor,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: number;
  color: string;
  barColor?: string;
}) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-sm font-medium">{label}</CardTitle>
        <Icon className={`h-4 w-4 ${color}`} />
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold">{value}</div>
        {barColor && value > 0 && (
          <div className="mt-2 h-1 rounded-full bg-muted overflow-hidden">
            <div className={`h-full ${barColor} rounded-full animate-in fade-in-0 slide-in-from-left-1/2`} style={{ width: "100%" }} />
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function StatusBadge({ status }: { status: string }) {
  const variant =
    status === "COMPLETED"
      ? "success"
      : status === "RUNNING"
        ? "default"
        : status === "FAILED" || status === "TIMED_OUT"
          ? "destructive"
          : status === "PAUSED"
            ? "warning"
            : "secondary";
  return <Badge variant={variant as "default"}>{status}</Badge>;
}
