import { useQuery } from "@tanstack/react-query";
import { workflowApi, taskApi, healthApi, metadataApi, formatTs } from "@/api/conductor";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Activity,
  CheckCircle2,
  XCircle,
  Clock,
  FileCode2,
  ListChecks,
  Server,
} from "lucide-react";

export default function Dashboard() {
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
        <Badge variant={healthQ.data ? "success" : "destructive"} className="gap-1">
          <Server className="h-3 w-3" />
          {healthQ.data ? "Healthy" : "Offline"}
        </Badge>
      </div>

      {/* Stats cards */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <StatCard icon={Activity} label="Running" value={running} color="text-blue-500" />
        <StatCard icon={CheckCircle2} label="Completed" value={completed} color="text-emerald-500" />
        <StatCard icon={XCircle} label="Failed" value={failed} color="text-red-500" />
        <StatCard icon={Clock} label="Queued Tasks" value={queueTotal} color="text-amber-500" />
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Card>
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
        <Card>
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
          <CardHeader>
            <CardTitle className="text-sm font-medium">Recent Executions</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {recent.slice(0, 8).map((wf) => (
                <div
                  key={wf.workflowId}
                  className="flex items-center justify-between rounded-lg border p-3 text-sm"
                >
                  <div>
                    <span className="font-medium">{wf.workflowType}</span>
                    <span className="text-muted-foreground ml-2 text-xs">
                      v{wf.version}
                    </span>
                  </div>
                  <div className="flex items-center gap-3">
                    <span className="text-xs text-muted-foreground">
                      {formatTs(wf.startTime)}
                    </span>
                    <StatusBadge status={wf.status} />
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

function StatCard({
  icon: Icon,
  label,
  value,
  color,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: number;
  color: string;
}) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-sm font-medium">{label}</CardTitle>
        <Icon className={`h-4 w-4 ${color}`} />
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold">{value}</div>
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
