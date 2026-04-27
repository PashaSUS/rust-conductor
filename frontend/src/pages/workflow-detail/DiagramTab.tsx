import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { CopyButton } from "@/components/CopyButton";
import { JsonView } from "@/components/JsonView";
import { TaskStatusBadge } from "@/components/StatusBadge";
import WorkflowDiagram from "@/components/WorkflowDiagram";
import { formatTs, type TaskResult } from "@/api/conductor";
import { useThemeText } from "@/components/ThemeContext";
import { ExternalLink, X } from "lucide-react";

import type { WorkflowTask } from "@/api/conductor";

interface DiagramTabProps {
  wfDef: { tasks: WorkflowTask[] } | undefined;
  tasks: TaskResult[];
  selectedTask: TaskResult | null;
  onSelectTask: (task: TaskResult | null) => void;
  onNavigate: (path: string) => void;
}

export function DiagramTab({ wfDef, tasks, selectedTask, onSelectTask, onNavigate }: DiagramTabProps) {
  const t = useThemeText();

  return (
    <Card>
      <CardContent className="pt-6">
        <div className="flex gap-4">
          {selectedTask && (
            <div className="w-80 shrink-0 border rounded-lg bg-muted overflow-auto max-h-150">
              <div className="flex items-center justify-between p-3 border-b bg-muted">
                <h4 className="font-semibold text-sm truncate">{selectedTask.referenceTaskName}</h4>
                <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => onSelectTask(null)}>
                  <X className="h-3 w-3" />
                </Button>
              </div>
              <div className="p-3 space-y-3 text-xs">
                <div className="flex justify-between">
                  <span className="text-muted-foreground">{t.status}</span>
                  <TaskStatusBadge status={selectedTask.status} />
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">{t.type}</span>
                  <span>{selectedTask.taskType}</span>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-muted-foreground">{t.task} {"ID"}</span>
                  <span className="flex items-center gap-1"><span className="font-mono truncate max-w-32" title={selectedTask.taskId}>{selectedTask.taskId}</span><CopyButton value={selectedTask.taskId} /></span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">{t.worker}</span>
                  <span>{selectedTask.workerId ?? "\u2014"}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">{t.pollCount}</span>
                  <span>{selectedTask.pollCount}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">{t.retryCount}</span>
                  <span>{selectedTask.retryCount}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">{t.scheduled}</span>
                  <span>{formatTs(selectedTask.scheduledTime)}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">{t.started}</span>
                  <span>{formatTs(selectedTask.startTime)}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">{t.ended}</span>
                  <span>{formatTs(selectedTask.endTime)}</span>
                </div>
                {selectedTask.reasonForIncompletion && (
                  <div>
                    <p className="text-muted-foreground mb-1">{t.reason}</p>
                    <p className="text-destructive">{selectedTask.reasonForIncompletion}</p>
                  </div>
                )}
                <div>
                  <p className="text-muted-foreground font-semibold mb-1">{t.input}</p>
                  <JsonView data={selectedTask.inputData} maxHeight="10rem" />
                </div>
                <div>
                  <p className="text-muted-foreground font-semibold mb-1">{t.output}</p>
                  <JsonView data={selectedTask.outputData} maxHeight="10rem" />
                </div>
                {selectedTask.taskType === "SUB_WORKFLOW" && selectedTask.subWorkflowId && (
                  <Button variant="outline" size="sm" className="w-full" onClick={() => onNavigate(`/executions/${selectedTask.subWorkflowId}`)}>
                    <ExternalLink className="h-3 w-3 mr-1" />
                    {t.viewSubWorkflow}
                  </Button>
                )}
              </div>
            </div>
          )}
          <div className="flex-1 min-w-0">
            {wfDef ? (
              <WorkflowDiagram
                definitionTasks={wfDef.tasks}
                runtimeTasks={tasks}
                onTaskClick={(refName) => {
                  const task = tasks.find((t2) => t2.referenceTaskName === refName);
                  onSelectTask(task ?? null);
                }}
              />
            ) : (
              <p className="text-muted-foreground text-sm">{t.loadingWorkflowDef}</p>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
