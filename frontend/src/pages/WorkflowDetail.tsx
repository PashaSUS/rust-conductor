import { useState, useEffect } from "react";
import { useParams, useNavigate } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { workflowApi, metadataApi, formatTs, type TaskResult } from "@/api/conductor";
import WorkflowDiagram from "@/components/WorkflowDiagram";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Pause, Play, XCircle, RotateCcw, RefreshCcw, ChevronDown, ChevronRight, ExternalLink, X, Clock, Repeat } from "lucide-react";
import { CopyButton } from "@/components/CopyButton";
import { ExecutionTimeline } from "@/components/ExecutionTimeline";
import { JsonView } from "@/components/JsonView";
import { StartWorkflowDialog } from "@/components/StartWorkflowDialog";
import { useThemeText } from "@/components/ThemeContext";

export default function WorkflowDetail() {
  const t = useThemeText();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [expandedTasks, setExpandedTasks] = useState<Set<string>>(new Set());
  const [selectedDiagramTask, setSelectedDiagramTask] = useState<TaskResult | null>(null);
  const [replayOpen, setReplayOpen] = useState(false);

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
    refetchInterval: (query) => {
      const status = query.state.data?.status;
      return status === "RUNNING" || status === "PAUSED" ? 3000 : false;
    },
  });

  const { data: wfDef } = useQuery({
    queryKey: ["workflowDef", wf?.workflowName, wf?.workflowVersion],
    queryFn: () => metadataApi.getWorkflowDef(wf!.workflowName, wf!.workflowVersion),
    enabled: !!wf,
  });

  const invalidate = () => queryClient.invalidateQueries({ queryKey: ["workflow", id] });

  const pauseMut = useMutation({ mutationFn: () => workflowApi.pause(id!), onSuccess: () => { toast.success(t.toastWorkflowPaused); invalidate(); } });
  const resumeMut = useMutation({ mutationFn: () => workflowApi.resume(id!), onSuccess: () => { toast.success(t.toastWorkflowResumed); invalidate(); } });
  const terminateMut = useMutation({ mutationFn: () => workflowApi.terminate(id!), onSuccess: () => { toast.success(t.toastWorkflowTerminated); invalidate(); } });
  const restartMut = useMutation({ mutationFn: () => workflowApi.restart(id!), onSuccess: () => { toast.success(t.toastWorkflowRestarted); invalidate(); } });
  const retryMut = useMutation({ mutationFn: () => workflowApi.retry(id!), onSuccess: () => { toast.success(t.toastWorkflowRetried); invalidate(); } });

  if (isLoading) return <p className="text-muted-foreground">{t.loading}</p>;
  if (!wf) return <p className="text-muted-foreground">{t.workflowNotFound}</p>;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">{wf.workflowName}</h2>
          <p className="text-muted-foreground text-sm flex items-center gap-1">v{wf.workflowVersion} &middot; <span className="font-mono">{wf.workflowId}</span><CopyButton value={wf.workflowId} /></p>
        </div>
        <div className="flex items-center gap-2">
          <StatusBadge status={wf.status} />
          <DurationBadge startTime={wf.startTime} endTime={wf.endTime} status={wf.status} />
          <div className="flex gap-1 ml-2">
            {wf.status === "RUNNING" && (
              <>
                <Button variant="outline" size="sm" onClick={() => pauseMut.mutate()}><Pause className="h-3 w-3 mr-1" />{t.pause}</Button>
                <Button variant="destructive" size="sm" onClick={() => terminateMut.mutate()}><XCircle className="h-3 w-3 mr-1" />{t.terminate}</Button>
              </>
            )}
            {wf.status === "PAUSED" && (
              <Button variant="outline" size="sm" onClick={() => resumeMut.mutate()}><Play className="h-3 w-3 mr-1" />{t.resume}</Button>
            )}
            {(wf.status === "FAILED" || wf.status === "TIMED_OUT") && (
              <Button variant="outline" size="sm" onClick={() => retryMut.mutate()}><RefreshCcw className="h-3 w-3 mr-1" />{t.retry}</Button>
            )}
            {(wf.status === "FAILED" || wf.status === "TIMED_OUT" || wf.status === "TERMINATED" || wf.status === "COMPLETED") && (
              <Button variant="outline" size="sm" onClick={() => restartMut.mutate()}><RotateCcw className="h-3 w-3 mr-1" />{t.restart}</Button>
            )}
            <Button variant="outline" size="sm" onClick={() => setReplayOpen(true)}>
              <Repeat className="h-3 w-3 mr-1" />{t.replayExecution}
            </Button>
          </div>
        </div>
      </div>

      <Tabs defaultValue="tasks">
        <TabsList>
          <TabsTrigger value="tasks">{t.tasksTab} ({wf.tasks.length})</TabsTrigger>
          <TabsTrigger value="timeline">{t.timeline}</TabsTrigger>
          <TabsTrigger value="diagram">{t.diagram}</TabsTrigger>
          <TabsTrigger value="input">{t.input}</TabsTrigger>
          <TabsTrigger value="output">{t.output}</TabsTrigger>
          <TabsTrigger value="info">{t.info}</TabsTrigger>
        </TabsList>

        <TabsContent value="timeline">
          <Card>
            <CardContent className="pt-6">
              <ExecutionTimeline
                tasks={wf.tasks}
                workflowStartTime={wf.startTime}
                workflowEndTime={wf.endTime}
              />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="tasks">
          <Card>
            <CardContent className="pt-6">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-8"></TableHead>
                    <TableHead>#</TableHead>
                    <TableHead>{t.task}</TableHead>
                    <TableHead>{t.type}</TableHead>
                    <TableHead>{t.status}</TableHead>
                    <TableHead>{t.started}</TableHead>
                    <TableHead>{t.ended}</TableHead>
                    <TableHead>{t.worker}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {wf.tasks.map((tk) => (
                    <>
                      <TableRow key={tk.taskId} className="cursor-pointer hover:bg-muted/50" onClick={() => toggleTask(tk.taskId)}>
                        <TableCell className="w-8 px-2">
                          {expandedTasks.has(tk.taskId) ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
                        </TableCell>
                        <TableCell>{tk.seq}</TableCell>
                        <TableCell className="font-medium">
                          <div className="flex items-center gap-1">
                            {tk.referenceTaskName}
                            <CopyButton value={tk.taskId} />
                          </div>
                        </TableCell>
                        <TableCell className="text-muted-foreground text-xs">{tk.taskType}</TableCell>
                        <TableCell><TaskStatusBadge status={tk.status} /></TableCell>
                        <TableCell className="text-xs">{formatTs(tk.startTime)}</TableCell>
                        <TableCell className="text-xs">{formatTs(tk.endTime)}</TableCell>
                        <TableCell className="text-xs text-muted-foreground">{tk.workerId ?? "—"}</TableCell>
                      </TableRow>
                      {expandedTasks.has(tk.taskId) && (
                        <TableRow key={`${tk.taskId}-detail`}>
                          <TableCell colSpan={8} className="bg-muted/30 p-4">
                            <div className="grid grid-cols-2 gap-4">
                              <div>
                                <p className="text-xs font-semibold mb-1 text-muted-foreground">{t.input}</p>
                                <JsonView data={tk.inputData} maxHeight="16rem" />
                              </div>
                              <div>
                                <p className="text-xs font-semibold mb-1 text-muted-foreground">{t.output}</p>
                                <JsonView data={tk.outputData} maxHeight="16rem" />
                              </div>
                            </div>
                            {tk.reasonForIncompletion && (
                              <div className="mt-2">
                                <p className="text-xs font-semibold text-muted-foreground">{t.reason}</p>
                                <p className="text-xs text-destructive">{tk.reasonForIncompletion}</p>
                              </div>
                            )}
                            {tk.taskType === "SUB_WORKFLOW" && tk.subWorkflowId && (
                              <div className="mt-3">
                                <Button
                                  variant="outline"
                                  size="sm"
                                  onClick={(e) => { e.stopPropagation(); navigate(`/executions/${tk.subWorkflowId}`); }}
                                >
                                  <ExternalLink className="h-3 w-3 mr-1" />
                                  {t.viewSubWorkflow}
                                </Button>
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
              <div className="flex gap-4">
                {/* Task detail panel */}
                {selectedDiagramTask && (
                  <div className="w-80 shrink-0 border rounded-lg bg-muted/20 overflow-auto max-h-150">
                    <div className="flex items-center justify-between p-3 border-b bg-muted/40">
                      <h4 className="font-semibold text-sm truncate">{selectedDiagramTask.referenceTaskName}</h4>
                      <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => setSelectedDiagramTask(null)}>
                        <X className="h-3 w-3" />
                      </Button>
                    </div>
                    <div className="p-3 space-y-3 text-xs">
                      <div className="flex justify-between">
                        <span className="text-muted-foreground">{t.status}</span>
                        <TaskStatusBadge status={selectedDiagramTask.status} />
                      </div>
                      <div className="flex justify-between">
                        <span className="text-muted-foreground">{t.type}</span>
                        <span>{selectedDiagramTask.taskType}</span>
                      </div>
                      <div className="flex justify-between items-center">
                        <span className="text-muted-foreground">{t.task} {"ID"}</span>
                        <span className="flex items-center gap-1"><span className="font-mono truncate max-w-32" title={selectedDiagramTask.taskId}>{selectedDiagramTask.taskId}</span><CopyButton value={selectedDiagramTask.taskId} /></span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-muted-foreground">{t.worker}</span>
                        <span>{selectedDiagramTask.workerId ?? "\u2014"}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-muted-foreground">{t.pollCount}</span>
                        <span>{selectedDiagramTask.pollCount}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-muted-foreground">{t.retryCount}</span>
                        <span>{selectedDiagramTask.retryCount}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-muted-foreground">{t.scheduled}</span>
                        <span>{formatTs(selectedDiagramTask.scheduledTime)}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-muted-foreground">{t.started}</span>
                        <span>{formatTs(selectedDiagramTask.startTime)}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-muted-foreground">{t.ended}</span>
                        <span>{formatTs(selectedDiagramTask.endTime)}</span>
                      </div>
                      {selectedDiagramTask.reasonForIncompletion && (
                        <div>
                          <p className="text-muted-foreground mb-1">{t.reason}</p>
                          <p className="text-destructive">{selectedDiagramTask.reasonForIncompletion}</p>
                        </div>
                      )}
                      <div>
                        <p className="text-muted-foreground font-semibold mb-1">{t.input}</p>
                        <JsonView data={selectedDiagramTask.inputData} maxHeight="10rem" />
                      </div>
                      <div>
                        <p className="text-muted-foreground font-semibold mb-1">{t.output}</p>
                        <JsonView data={selectedDiagramTask.outputData} maxHeight="10rem" />
                      </div>
                      {selectedDiagramTask.taskType === "SUB_WORKFLOW" && selectedDiagramTask.subWorkflowId && (
                        <Button
                          variant="outline"
                          size="sm"
                          className="w-full"
                          onClick={() => navigate(`/executions/${selectedDiagramTask.subWorkflowId}`)}
                        >
                          <ExternalLink className="h-3 w-3 mr-1" />
                          {t.viewSubWorkflow}
                        </Button>
                      )}
                    </div>
                  </div>
                )}
                {/* Diagram */}
                <div className="flex-1 min-w-0">
                  {wfDef ? (
                    <WorkflowDiagram
                      definitionTasks={wfDef.tasks}
                      runtimeTasks={wf.tasks}
                      onTaskClick={(refName) => {
                        const task = wf.tasks.find((t2) => t2.referenceTaskName === refName);
                        setSelectedDiagramTask(task ?? null);
                      }}
                    />
                  ) : (
                    <p className="text-muted-foreground text-sm">{t.loadingWorkflowDef}</p>
                  )}
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="input">
          <Card>
            <CardContent className="pt-6">
              <JsonView data={wf.input} maxHeight="24rem" />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="output">
          <Card>
            <CardContent className="pt-6">
              <JsonView data={wf.output} maxHeight="24rem" />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="info">
          <Card>
            <CardContent className="pt-6 space-y-2 text-sm">
              <InfoRow label={t.workflowIdLabel} value={wf.workflowId} copyable />
              <InfoRow label={t.correlationIdLabel} value={wf.correlationId ?? "\u2014"} copyable />
              <InfoRow label={t.priority} value={String(wf.priority)} />
              <InfoRow label={t.started} value={formatTs(wf.startTime)} />
              <InfoRow label={t.ended} value={formatTs(wf.endTime)} />
              <InfoRow label={t.updated} value={formatTs(wf.updateTime)} />
              {wf.reasonForIncompletion && <InfoRow label={t.reason} value={wf.reasonForIncompletion} />}
              {wf.tags && wf.tags.length > 0 && (
                <div className="flex items-center">
                  <span className="font-medium text-muted-foreground w-36">{t.tagsLabel}</span>
                  <div className="flex gap-1 flex-wrap">
                    {wf.tags.map((tag) => (
                      <Badge key={tag} variant="secondary">{tag}</Badge>
                    ))}
                  </div>
                </div>
              )}
              {wfDef?.slaDeadlineSeconds && wfDef.slaDeadlineSeconds > 0 && (
                <InfoRow label={t.slaDeadline} value={`${wfDef.slaDeadlineSeconds}s`} />
              )}
              {wfDef?.onCompleteWebhook && (
                <InfoRow label={t.onCompleteWebhook} value={wfDef.onCompleteWebhook} />
              )}
              {wfDef?.onFailureWebhook && (
                <InfoRow label={t.onFailureWebhook} value={wfDef.onFailureWebhook} />
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      {/* Replay dialog — pre-filled with this execution's workflow & input */}
      <StartWorkflowDialog
        open={replayOpen}
        onOpenChange={setReplayOpen}
        preselectedDef={{ name: wf.workflowName, version: wf.workflowVersion } as any}
        prefilledInput={wf.input as Record<string, unknown> | undefined}
      />
    </div>
  );
}

function InfoRow({ label, value, copyable }: { label: string; value: string; copyable?: boolean }) {
  return (
    <div className="flex items-center">
      <span className="font-medium text-muted-foreground w-36">{label}</span>
      <span className="font-mono text-xs break-all">{value}</span>
      {copyable && value && value !== "—" && <CopyButton value={value} className="ml-1" />}
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const variant = status === "COMPLETED" ? "success" : status === "RUNNING" ? "default" : status === "FAILED" ? "destructive" : status === "TIMED_OUT" ? "destructive" : status === "PAUSED" ? "warning" : "secondary";
  return <Badge variant={variant as "default"} className="text-sm">{status}</Badge>;
}

function TaskStatusBadge({ status }: { status: string }) {
  const variant = status === "COMPLETED" ? "success" : status === "IN_PROGRESS" ? "default" : status === "FAILED" ? "destructive" : status === "TIMED_OUT" ? "destructive" : status === "SCHEDULED" ? "warning" : "secondary";
  return <Badge variant={variant as "default"}>{status}</Badge>;
}

function DurationBadge({ startTime, endTime, status }: { startTime: number; endTime?: number; status: string }) {
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
