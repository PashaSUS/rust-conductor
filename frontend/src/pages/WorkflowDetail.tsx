import { useState } from "react";
import { useParams } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { workflowApi, metadataApi, formatTs } from "@/api/conductor";
import WorkflowDiagram from "@/components/WorkflowDiagram";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Pause, Play, XCircle, RotateCcw, RefreshCcw, ChevronDown, ChevronRight } from "lucide-react";

export default function WorkflowDetail() {
  const { id } = useParams<{ id: string }>();
  const queryClient = useQueryClient();
  const [expandedTasks, setExpandedTasks] = useState<Set<string>>(new Set());

  const toggleTask = (taskId: string) => {
    setExpandedTasks((prev) => {
      const next = new Set(prev);
      if (next.has(taskId)) next.delete(taskId);
      else next.add(taskId);
      return next;
    });
  };

  const { data: wf, isLoading } = useQuery({
    queryKey: ["workflow", id],
    queryFn: () => workflowApi.get(id!),
    enabled: !!id,
  });

  const { data: wfDef } = useQuery({
    queryKey: ["workflowDef", wf?.workflowName, wf?.workflowVersion],
    queryFn: () => metadataApi.getWorkflowDef(wf!.workflowName, wf!.workflowVersion),
    enabled: !!wf,
  });

  const invalidate = () => queryClient.invalidateQueries({ queryKey: ["workflow", id] });

  const pauseMut = useMutation({ mutationFn: () => workflowApi.pause(id!), onSuccess: () => { toast.success("Paused"); invalidate(); } });
  const resumeMut = useMutation({ mutationFn: () => workflowApi.resume(id!), onSuccess: () => { toast.success("Resumed"); invalidate(); } });
  const terminateMut = useMutation({ mutationFn: () => workflowApi.terminate(id!), onSuccess: () => { toast.success("Terminated"); invalidate(); } });
  const restartMut = useMutation({ mutationFn: () => workflowApi.restart(id!), onSuccess: () => { toast.success("Restarted"); invalidate(); } });
  const retryMut = useMutation({ mutationFn: () => workflowApi.retry(id!), onSuccess: () => { toast.success("Retried"); invalidate(); } });

  if (isLoading) return <p className="text-muted-foreground">Loading...</p>;
  if (!wf) return <p className="text-muted-foreground">Workflow not found</p>;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">{wf.workflowName}</h2>
          <p className="text-muted-foreground text-sm">v{wf.workflowVersion} &middot; {wf.workflowId}</p>
        </div>
        <div className="flex items-center gap-2">
          <StatusBadge status={wf.status} />
          <div className="flex gap-1 ml-2">
            {wf.status === "RUNNING" && (
              <>
                <Button variant="outline" size="sm" onClick={() => pauseMut.mutate()}><Pause className="h-3 w-3 mr-1" />Pause</Button>
                <Button variant="destructive" size="sm" onClick={() => terminateMut.mutate()}><XCircle className="h-3 w-3 mr-1" />Terminate</Button>
              </>
            )}
            {wf.status === "PAUSED" && (
              <Button variant="outline" size="sm" onClick={() => resumeMut.mutate()}><Play className="h-3 w-3 mr-1" />Resume</Button>
            )}
            {wf.status === "FAILED" && (
              <Button variant="outline" size="sm" onClick={() => retryMut.mutate()}><RefreshCcw className="h-3 w-3 mr-1" />Retry</Button>
            )}
            {(wf.status === "FAILED" || wf.status === "TERMINATED" || wf.status === "COMPLETED") && (
              <Button variant="outline" size="sm" onClick={() => restartMut.mutate()}><RotateCcw className="h-3 w-3 mr-1" />Restart</Button>
            )}
          </div>
        </div>
      </div>

      <Tabs defaultValue="tasks">
        <TabsList>
          <TabsTrigger value="tasks">Tasks ({wf.tasks.length})</TabsTrigger>
          <TabsTrigger value="diagram">Diagram</TabsTrigger>
          <TabsTrigger value="input">Input</TabsTrigger>
          <TabsTrigger value="output">Output</TabsTrigger>
          <TabsTrigger value="info">Info</TabsTrigger>
        </TabsList>

        <TabsContent value="tasks">
          <Card>
            <CardContent className="pt-6">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-8"></TableHead>
                    <TableHead>#</TableHead>
                    <TableHead>Task</TableHead>
                    <TableHead>Type</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Started</TableHead>
                    <TableHead>Ended</TableHead>
                    <TableHead>Worker</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {wf.tasks.map((t) => (
                    <>
                      <TableRow key={t.taskId} className="cursor-pointer hover:bg-muted/50" onClick={() => toggleTask(t.taskId)}>
                        <TableCell className="w-8 px-2">
                          {expandedTasks.has(t.taskId) ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
                        </TableCell>
                        <TableCell>{t.seq}</TableCell>
                        <TableCell className="font-medium">{t.referenceTaskName}</TableCell>
                        <TableCell className="text-muted-foreground text-xs">{t.taskType}</TableCell>
                        <TableCell><TaskStatusBadge status={t.status} /></TableCell>
                        <TableCell className="text-xs">{formatTs(t.startTime)}</TableCell>
                        <TableCell className="text-xs">{formatTs(t.endTime)}</TableCell>
                        <TableCell className="text-xs text-muted-foreground">{t.workerId ?? "—"}</TableCell>
                      </TableRow>
                      {expandedTasks.has(t.taskId) && (
                        <TableRow key={`${t.taskId}-detail`}>
                          <TableCell colSpan={8} className="bg-muted/30 p-4">
                            <div className="grid grid-cols-2 gap-4">
                              <div>
                                <p className="text-xs font-semibold mb-1 text-muted-foreground">Input</p>
                                <pre className="text-xs rounded-lg bg-muted p-3 overflow-auto max-h-64">
                                  {JSON.stringify(t.inputData, null, 2)}
                                </pre>
                              </div>
                              <div>
                                <p className="text-xs font-semibold mb-1 text-muted-foreground">Output</p>
                                <pre className="text-xs rounded-lg bg-muted p-3 overflow-auto max-h-64">
                                  {JSON.stringify(t.outputData, null, 2)}
                                </pre>
                              </div>
                            </div>
                            {t.reasonForIncompletion && (
                              <div className="mt-2">
                                <p className="text-xs font-semibold text-muted-foreground">Reason</p>
                                <p className="text-xs text-destructive">{t.reasonForIncompletion}</p>
                              </div>
                            )}
                          </TableCell>
                        </TableRow>
                      )}
                    </>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="diagram">
          <Card>
            <CardContent className="pt-6">
              {wfDef ? (
                <WorkflowDiagram
                  definitionTasks={wfDef.tasks}
                  runtimeTasks={wf.tasks}
                />
              ) : (
                <p className="text-muted-foreground text-sm">Loading workflow definition...</p>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="input">
          <Card>
            <CardContent className="pt-6">
              <pre className="text-xs rounded-lg bg-muted p-4 overflow-auto max-h-96">
                {JSON.stringify(wf.input, null, 2)}
              </pre>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="output">
          <Card>
            <CardContent className="pt-6">
              <pre className="text-xs rounded-lg bg-muted p-4 overflow-auto max-h-96">
                {JSON.stringify(wf.output, null, 2)}
              </pre>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="info">
          <Card>
            <CardContent className="pt-6 space-y-2 text-sm">
              <InfoRow label="Workflow ID" value={wf.workflowId} />
              <InfoRow label="Correlation ID" value={wf.correlationId ?? "—"} />
              <InfoRow label="Priority" value={String(wf.priority)} />
              <InfoRow label="Started" value={formatTs(wf.startTime)} />
              <InfoRow label="Ended" value={formatTs(wf.endTime)} />
              <InfoRow label="Updated" value={formatTs(wf.updateTime)} />
              {wf.reasonForIncompletion && <InfoRow label="Reason" value={wf.reasonForIncompletion} />}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex">
      <span className="font-medium text-muted-foreground w-36">{label}</span>
      <span className="font-mono text-xs break-all">{value}</span>
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const variant = status === "COMPLETED" ? "success" : status === "RUNNING" ? "default" : status === "FAILED" ? "destructive" : status === "PAUSED" ? "warning" : "secondary";
  return <Badge variant={variant as "default"} className="text-sm">{status}</Badge>;
}

function TaskStatusBadge({ status }: { status: string }) {
  const variant = status === "COMPLETED" ? "success" : status === "IN_PROGRESS" ? "default" : status === "FAILED" ? "destructive" : status === "SCHEDULED" ? "warning" : "secondary";
  return <Badge variant={variant as "default"}>{status}</Badge>;
}
