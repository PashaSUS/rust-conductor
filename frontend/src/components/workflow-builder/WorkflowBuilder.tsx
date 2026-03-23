import { useState, useCallback } from "react";
import { useQuery } from "@tanstack/react-query";
import { metadataApi, type WorkflowDef } from "@/api/conductor";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Plus, Code2, Wand2 } from "lucide-react";
import { useThemeText } from "@/components/ThemeContext";

import {
  type TaskFormState,
  createEmptyTask,
  taskFormToWorkflowTask,
  workflowTaskToForm,
} from "./types";
import { TaskCard } from "./TaskCard";
import { WorkflowSettings } from "./WorkflowSettings";

interface WorkflowBuilderProps {
  initialDef?: WorkflowDef;
  onSubmit: (def: WorkflowDef) => void;
  isPending?: boolean;
  submitLabel?: string;
}

export default function WorkflowBuilder({
  initialDef,
  onSubmit,
  isPending,
  submitLabel,
}: WorkflowBuilderProps) {
  const t = useThemeText();
  const resolvedLabel = submitLabel ?? t.createWorkflow;
  const [mode, setMode] = useState<"visual" | "json">("visual");

  const { data: registeredTasks } = useQuery({
    queryKey: ["task-defs"],
    queryFn: metadataApi.listTaskDefs,
  });

  // Workflow metadata
  const [name, setName] = useState(initialDef?.name ?? "");
  const [version, setVersion] = useState(initialDef?.version ?? 1);
  const [description, setDescription] = useState(initialDef?.description ?? "");
  const [timeoutSeconds, setTimeoutSeconds] = useState(initialDef?.timeoutSeconds ?? 0);
  const [ownerEmail, setOwnerEmail] = useState(initialDef?.ownerEmail ?? "");
  const [failureWorkflow, setFailureWorkflow] = useState(initialDef?.failureWorkflow ?? "");
  const [inputParams, setInputParams] = useState<string[]>(
    initialDef?.inputParameters ?? []
  );
  const [onCompleteWebhook, setOnCompleteWebhook] = useState(initialDef?.onCompleteWebhook ?? "");
  const [onFailureWebhook, setOnFailureWebhook] = useState(initialDef?.onFailureWebhook ?? "");
  const [slaDeadlineSeconds, setSlaDeadlineSeconds] = useState(initialDef?.slaDeadlineSeconds ?? 0);
  const [tags, setTags] = useState<string[]>(initialDef?.tags ?? []);

  // Tasks
  const [tasks, setTasks] = useState<TaskFormState[]>(
    initialDef?.tasks.map(workflowTaskToForm) ?? []
  );
  const [expandedIdx, setExpandedIdx] = useState<number | null>(null);

  // JSON mode
  const [jsonText, setJsonText] = useState(() =>
    initialDef ? JSON.stringify(initialDef, null, 2) : ""
  );
  const [jsonError, setJsonError] = useState("");

  const workflowInputParams = inputParams;

  const handleSettingsUpdate = useCallback(
    (field: string, value: string | number) => {
      switch (field) {
        case "name": setName(value as string); break;
        case "version": setVersion(value as number); break;
        case "description": setDescription(value as string); break;
        case "timeoutSeconds": setTimeoutSeconds(value as number); break;
        case "ownerEmail": setOwnerEmail(value as string); break;
        case "failureWorkflow": setFailureWorkflow(value as string); break;
        case "onCompleteWebhook": setOnCompleteWebhook(value as string); break;
        case "onFailureWebhook": setOnFailureWebhook(value as string); break;
        case "slaDeadlineSeconds": setSlaDeadlineSeconds(value as number); break;
      }
    },
    []
  );

  const addTask = useCallback(() => {
    const newTask = createEmptyTask();
    newTask.taskReferenceName = `task_${tasks.length + 1}`;
    setTasks((prev) => [...prev, newTask]);
    setExpandedIdx(tasks.length);
  }, [tasks.length]);

  const removeTask = useCallback((idx: number) => {
    setTasks((prev) => prev.filter((_, i) => i !== idx));
    setExpandedIdx(null);
  }, []);

  const moveTask = useCallback((idx: number, direction: "up" | "down") => {
    setTasks((prev) => {
      const arr = [...prev];
      const target = direction === "up" ? idx - 1 : idx + 1;
      if (target < 0 || target >= arr.length) return arr;
      [arr[idx], arr[target]] = [arr[target], arr[idx]];
      return arr;
    });
    setExpandedIdx(direction === "up" ? idx - 1 : idx + 1);
  }, []);

  const duplicateTask = useCallback((idx: number) => {
    setTasks((prev) => {
      const copy = JSON.parse(JSON.stringify(prev[idx])) as TaskFormState;
      copy.taskReferenceName = `${copy.taskReferenceName}_copy`;
      return [...prev.slice(0, idx + 1), copy, ...prev.slice(idx + 1)];
    });
  }, []);

  const updateTask = useCallback((idx: number, updates: Partial<TaskFormState>) => {
    setTasks((prev) => prev.map((t, i) => (i === idx ? { ...t, ...updates } : t)));
  }, []);

  const buildDef = useCallback((): WorkflowDef => {
    const def: WorkflowDef = {
      name,
      version,
      tasks: tasks.map(taskFormToWorkflowTask),
    };
    if (description) def.description = description;
    if (timeoutSeconds > 0) def.timeoutSeconds = timeoutSeconds;
    if (ownerEmail) def.ownerEmail = ownerEmail;
    if (failureWorkflow) def.failureWorkflow = failureWorkflow;
    if (workflowInputParams.length > 0) def.inputParameters = workflowInputParams;
    if (onCompleteWebhook) def.onCompleteWebhook = onCompleteWebhook;
    if (onFailureWebhook) def.onFailureWebhook = onFailureWebhook;
    if (slaDeadlineSeconds > 0) def.slaDeadlineSeconds = slaDeadlineSeconds;
    if (tags.length > 0) def.tags = tags;
    return def;
  }, [name, version, description, timeoutSeconds, ownerEmail, failureWorkflow, workflowInputParams, onCompleteWebhook, onFailureWebhook, slaDeadlineSeconds, tags, tasks]);

  const handleVisualSubmit = () => {
    if (!name.trim() || tasks.length === 0) return;
    onSubmit(buildDef());
  };

  const handleJsonSubmit = () => {
    try {
      const def = JSON.parse(jsonText) as WorkflowDef;
      setJsonError("");
      onSubmit(def);
    } catch (e) {
      setJsonError(e instanceof Error ? e.message : t.toastInvalidJson);
    }
  };

  const syncToJson = () => {
    setJsonText(JSON.stringify(buildDef(), null, 2));
    setJsonError("");
    setMode("json");
  };

  const syncToVisual = () => {
    try {
      const def = JSON.parse(jsonText) as WorkflowDef;
      setName(def.name);
      setVersion(def.version);
      setDescription(def.description ?? "");
      setTimeoutSeconds(def.timeoutSeconds ?? 0);
      setOwnerEmail(def.ownerEmail ?? "");
      setFailureWorkflow(def.failureWorkflow ?? "");
      setInputParams(def.inputParameters ?? []);
      setOnCompleteWebhook(def.onCompleteWebhook ?? "");
      setOnFailureWebhook(def.onFailureWebhook ?? "");
      setSlaDeadlineSeconds(def.slaDeadlineSeconds ?? 0);
      setTags(def.tags ?? []);
      setTasks(def.tasks.map(workflowTaskToForm));
      setJsonError("");
      setMode("visual");
    } catch (e) {
      setJsonError(e instanceof Error ? e.message : t.toastInvalidJson);
    }
  };

  return (
    <div className="space-y-4">
      {/* Mode toggle */}
      <div className="flex items-center gap-2">
        <Button
          variant={mode === "visual" ? "default" : "outline"}
          size="sm"
          onClick={() => (mode === "json" ? syncToVisual() : setMode("visual"))}
        >
          <Wand2 className="h-3 w-3 mr-1" />
          {t.visualBuilder}
        </Button>
        <Button
          variant={mode === "json" ? "default" : "outline"}
          size="sm"
          onClick={() => (mode === "visual" ? syncToJson() : setMode("json"))}
        >
          <Code2 className="h-3 w-3 mr-1" />
          {t.jsonEditor}
        </Button>
      </div>

      {mode === "json" ? (
        <div className="space-y-3">
          <Textarea
            rows={24}
            value={jsonText}
            onChange={(e) => {
              setJsonText(e.target.value);
              setJsonError("");
            }}
            className="font-mono text-xs"
            placeholder='{"name": "my_workflow", "version": 1, "tasks": [...]}'
          />
          {jsonError && <p className="text-sm text-destructive">{jsonError}</p>}
          <Button onClick={handleJsonSubmit} disabled={isPending}>
            {isPending ? t.saving : resolvedLabel}
          </Button>
        </div>
      ) : (
        <div className="space-y-4">
          <WorkflowSettings
            name={name}
            version={version}
            description={description}
            timeoutSeconds={timeoutSeconds}
            ownerEmail={ownerEmail}
            failureWorkflow={failureWorkflow}
            inputParams={inputParams}
            onCompleteWebhook={onCompleteWebhook}
            onFailureWebhook={onFailureWebhook}
            slaDeadlineSeconds={slaDeadlineSeconds}
            tags={tags}
            onUpdate={handleSettingsUpdate}
            onUpdateInputParams={setInputParams}
            onUpdateTags={setTags}
          />

          {/* Tasks */}
          <Card>
            <CardHeader className="pb-3">
              <div className="flex items-center justify-between">
                <CardTitle className="text-sm font-medium">
                  {t.tasksTab} ({tasks.length})
                </CardTitle>
                <Button size="sm" onClick={addTask}>
                  <Plus className="h-3 w-3 mr-1" />
                  {t.addTask}
                </Button>
              </div>
            </CardHeader>
            <CardContent>
              {tasks.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  <p className="text-sm">{t.noTasksYet}</p>
                  <p className="text-xs mt-1">{t.tasksSequentialHint}</p>
                </div>
              ) : (
                <div className="space-y-2">
                  {tasks.map((task, idx) => (
                    <TaskCard
                      key={idx}
                      task={task}
                      index={idx}
                      total={tasks.length}
                      expanded={expandedIdx === idx}
                      onToggle={() => setExpandedIdx(expandedIdx === idx ? null : idx)}
                      onUpdate={(updates) => updateTask(idx, updates)}
                      onRemove={() => removeTask(idx)}
                      onMoveUp={() => moveTask(idx, "up")}
                      onMoveDown={() => moveTask(idx, "down")}
                      onDuplicate={() => duplicateTask(idx)}
                      registeredTasks={registeredTasks ?? []}
                      allTasks={tasks}
                      taskIndex={idx}
                      workflowInputParams={workflowInputParams}
                    />
                  ))}
                </div>
              )}
            </CardContent>
          </Card>

          {/* Validation + Submit */}
          <div className="flex items-center justify-between">
            <div className="text-sm text-muted-foreground">
              {!name.trim() && (
                <span className="text-destructive">{t.workflowNameRequired} </span>
              )}
              {tasks.length === 0 && (
                <span className="text-destructive">{t.addAtLeastOneTask} </span>
              )}
              {tasks.some((t) => !t.taskReferenceName.trim()) && (
                <span className="text-destructive">{t.allTasksNeedRefName} </span>
              )}
            </div>
            <div className="flex gap-2">
              <Button variant="outline" size="sm" onClick={syncToJson}>
                <Code2 className="h-3 w-3 mr-1" />
                {t.previewJson}
              </Button>
              <Button
                onClick={handleVisualSubmit}
                disabled={isPending || !name.trim() || tasks.length === 0}
              >
                {isPending ? t.saving : resolvedLabel}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
