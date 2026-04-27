import { useState, useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { metadataApi, type TaskDef, type WorkflowDef } from "@/api/conductor";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { SearchableSelect } from "@/components/SearchableSelect";
import { JsonEditor } from "@/components/JsonEditor";
import { Plus, Trash2, Zap, Link2 } from "lucide-react";
import { toast } from "sonner";
import type { DesignerTask } from "./types";
import { TASK_TYPES, TASK_TYPE_COLORS } from "./constants";
import { DecisionFields, DoWhileFields, DynamicForkFields } from "./BranchingFields";

interface TaskEditorPanelProps {
  editingTask: DesignerTask;
  setEditingTask: (task: DesignerTask | null) => void;
  tasks: DesignerTask[];
  taskDefMap: Map<string, TaskDef>;
  taskDefOptions: { label: string; value: string }[];
  workflowDefOptions: { label: string; value: string }[];
  workflowName: string;
  onApply: (task: DesignerTask) => void;
}

export function TaskEditorPanel({
  editingTask, setEditingTask, tasks, taskDefMap, taskDefOptions,
  workflowDefOptions, workflowName, onApply,
}: TaskEditorPanelProps) {
  const queryClient = useQueryClient();

  const [rawJsonText, setRawJsonText] = useState("");
  const [newInputKey, setNewInputKey] = useState("");
  const [newOutputKey, setNewOutputKey] = useState("");

  useEffect(() => {
    setRawJsonText(JSON.stringify(editingTask.inputParameters, null, 2));
  }, [editingTask.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const selectTaskDef = (taskDefName: string) => {
    const td = taskDefMap.get(taskDefName);
    const inputKeys = td?.inputKeys ?? [];
    const outputKeys = td?.outputKeys ?? [];
    const newInputParams: Record<string, unknown> = {};
    for (const k of inputKeys) newInputParams[k] = editingTask.inputParameters[k] ?? "";
    for (const [k, v] of Object.entries(editingTask.inputParameters)) {
      if (!(k in newInputParams)) newInputParams[k] = v;
    }
    setEditingTask({
      ...editingTask, name: taskDefName,
      taskReferenceName: editingTask.taskReferenceName === editingTask.name ? taskDefName : editingTask.taskReferenceName,
      inputParameters: newInputParams, outputKeys,
    });
  };

  const addInputToTask = () => {
    if (!newInputKey.trim()) return;
    const updated = { ...editingTask, inputParameters: { ...editingTask.inputParameters, [newInputKey.trim()]: "" } };
    setEditingTask(updated);
    setRawJsonText(JSON.stringify(updated.inputParameters, null, 2));
    setNewInputKey("");
  };
  const removeInputFromTask = (key: string) => {
    const { [key]: _, ...rest } = editingTask.inputParameters;
    const updated = { ...editingTask, inputParameters: rest };
    setEditingTask(updated);
    setRawJsonText(JSON.stringify(updated.inputParameters, null, 2));
  };
  const addOutputToTask = () => {
    if (!newOutputKey.trim() || editingTask.outputKeys.includes(newOutputKey.trim())) return;
    setEditingTask({ ...editingTask, outputKeys: [...editingTask.outputKeys, newOutputKey.trim()] });
    setNewOutputKey("");
  };
  const removeOutputFromTask = (key: string) => {
    setEditingTask({ ...editingTask, outputKeys: editingTask.outputKeys.filter((k) => k !== key) });
  };

  const color = TASK_TYPE_COLORS[editingTask.type] ?? "#9ca3af";
  const stepIdx = tasks.findIndex((t) => t.id === editingTask.id);

  return (
    <div className="p-4 space-y-3">
      <div className="flex items-center justify-between">
        <h4 className="font-semibold text-sm">Edit Task</h4>
        <Button variant="ghost" size="sm" className="h-6 px-1" onClick={() => setEditingTask(null)}>×</Button>
      </div>
      <div className="flex items-center gap-2">
        <span className="w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold text-white" style={{ backgroundColor: color }}>{stepIdx + 1}</span>
        <Badge variant="outline" className="text-xs" style={{ borderColor: color }}>{editingTask.type}</Badge>
        <span className="text-[9px] text-muted-foreground ml-auto">Step {stepIdx + 1} of {tasks.length}</span>
      </div>

      {editingTask.type === "SIMPLE" && (
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">Task Definition <span className="text-[9px]">(auto-populates I/O)</span></label>
          <SearchableSelect options={taskDefOptions} value={editingTask.name} onChange={selectTaskDef} placeholder="Select registered task..." />
          {taskDefMap.has(editingTask.name) && (
            <p className="text-[9px] text-emerald-600 mt-1">Loaded {taskDefMap.get(editingTask.name)!.inputKeys?.length ?? 0} inputs, {taskDefMap.get(editingTask.name)!.outputKeys?.length ?? 0} outputs from registry</p>
          )}
        </div>
      )}

      <div><label className="text-xs text-muted-foreground mb-1 block">Name</label><Input value={editingTask.name} onChange={(e) => setEditingTask({ ...editingTask, name: e.target.value })} className="h-8 text-xs" /></div>
      <div><label className="text-xs text-muted-foreground mb-1 block">Reference Name</label><Input value={editingTask.taskReferenceName} onChange={(e) => setEditingTask({ ...editingTask, taskReferenceName: e.target.value })} className="h-8 text-xs" /></div>
      <div>
        <label className="text-xs text-muted-foreground mb-1 block">Type</label>
        <Select value={editingTask.type} onValueChange={(v) => setEditingTask({ ...editingTask, type: v })}>
          <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
          <SelectContent>{TASK_TYPES.map((t) => (<SelectItem key={t} value={t}>{t}</SelectItem>))}</SelectContent>
        </Select>
      </div>
      <div className="flex items-center gap-2">
        <input type="checkbox" checked={editingTask.optional ?? false} onChange={(e) => setEditingTask({ ...editingTask, optional: e.target.checked })} className="h-3 w-3" />
        <label className="text-xs text-muted-foreground">Optional (continue on failure)</label>
      </div>
      <div><label className="text-xs text-muted-foreground mb-1 block">Description</label><Textarea value={editingTask.description} onChange={(e) => setEditingTask({ ...editingTask, description: e.target.value })} placeholder="Describe what this task does..." className="text-xs min-h-16 resize-y" /></div>

      <HttpFields editingTask={editingTask} setEditingTask={setEditingTask} />

      {/* Input parameters */}
      <div className="border-t pt-2">
        <div className="flex items-center justify-between mb-1.5">
          <label className="text-xs font-medium text-blue-600 dark:text-blue-400">Input Parameters</label>
          <Badge variant="secondary" className="text-[10px]">{Object.keys(editingTask.inputParameters).length}</Badge>
        </div>
        <div className="space-y-1.5 mb-2">
          {Object.entries(editingTask.inputParameters).map(([key, value]) => (
            <div key={key} className="flex items-center gap-1 group">
              <span className="w-2 h-2 rounded-full bg-blue-500 shrink-0" />
              <span className="text-[10px] font-mono font-medium text-blue-600 dark:text-blue-400 shrink-0">{key}</span>
              <span className="text-[9px] text-muted-foreground truncate flex-1">{typeof value === "string" ? value : JSON.stringify(value)}</span>
              <button onClick={() => removeInputFromTask(key)} className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive p-0.5"><Trash2 className="h-2.5 w-2.5" /></button>
            </div>
          ))}
        </div>
        <div className="flex gap-1">
          <Input value={newInputKey} onChange={(e) => setNewInputKey(e.target.value)} onKeyDown={(e) => e.key === "Enter" && addInputToTask()} placeholder="Add input key..." className="h-7 text-xs flex-1" />
          <Button variant="outline" size="sm" className="h-7 px-2" onClick={addInputToTask}><Plus className="h-3 w-3" /></Button>
        </div>
      </div>

      {/* Output keys */}
      <div className="border-t pt-2">
        <div className="flex items-center justify-between mb-1.5">
          <label className="text-xs font-medium text-emerald-600 dark:text-emerald-400">Output Keys</label>
          <Badge variant="secondary" className="text-[10px]">{editingTask.outputKeys.length}</Badge>
        </div>
        <p className="text-[9px] text-muted-foreground mb-1.5">Output keys define connectable green ports on the right side of the node.</p>
        <div className="space-y-1 mb-2">
          {editingTask.outputKeys.map((key) => (
            <div key={key} className="flex items-center gap-1 group">
              <span className="w-2 h-2 rounded-full bg-emerald-500 shrink-0" />
              <span className="text-[10px] font-mono font-medium text-emerald-600 dark:text-emerald-400">{key}</span>
              <div className="flex-1" />
              <button onClick={() => removeOutputFromTask(key)} className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive p-0.5"><Trash2 className="h-2.5 w-2.5" /></button>
            </div>
          ))}
        </div>
        <div className="flex gap-1">
          <Input value={newOutputKey} onChange={(e) => setNewOutputKey(e.target.value)} onKeyDown={(e) => e.key === "Enter" && addOutputToTask()} placeholder="Add output key..." className="h-7 text-xs flex-1" />
          <Button variant="outline" size="sm" className="h-7 px-2" onClick={addOutputToTask}><Plus className="h-3 w-3" /></Button>
        </div>
      </div>

      {/* Raw JSON */}
      <div className="border-t pt-2">
        <label className="text-xs text-muted-foreground mb-1 block">Input Parameters (raw JSON)</label>
        <JsonEditor
          value={rawJsonText}
          onChange={(v) => {
            setRawJsonText(v);
            try { setEditingTask({ ...editingTask, inputParameters: JSON.parse(v) }); } catch { /* typing */ }
          }}
          rows={6}
          ariaLabel="Input parameters JSON"
        />
      </div>

      <SubWorkflowFields editingTask={editingTask} setEditingTask={setEditingTask} workflowDefOptions={workflowDefOptions} workflowName={workflowName} tasks={tasks} queryClient={queryClient} />
      <DecisionFields editingTask={editingTask} setEditingTask={setEditingTask} />
      <DoWhileFields editingTask={editingTask} setEditingTask={setEditingTask} />
      <DynamicForkFields editingTask={editingTask} setEditingTask={setEditingTask} />

      <div className="flex gap-2 pt-1">
        <Button size="sm" className="flex-1" onClick={() => onApply(editingTask)}><Zap className="h-3 w-3 mr-1" /> Apply</Button>
        <Button variant="outline" size="sm" onClick={() => setEditingTask(null)}>Cancel</Button>
      </div>
    </div>
  );
}

function HttpFields({ editingTask, setEditingTask }: { editingTask: DesignerTask; setEditingTask: (t: DesignerTask) => void }) {
  if (editingTask.type !== "HTTP") return null;
  const httpReq = (editingTask.inputParameters.http_request ?? {}) as Record<string, unknown>;
  const updateHttp = (field: string, value: unknown) => {
    setEditingTask({ ...editingTask, inputParameters: { ...editingTask.inputParameters, http_request: { ...httpReq, [field]: value } } });
  };
  return (
    <div className="space-y-2 border-t pt-2">
      <label className="text-xs font-medium" style={{ color: TASK_TYPE_COLORS.HTTP }}>HTTP Request</label>
      <div className="grid grid-cols-[1fr_auto] gap-2">
        <div><label className="text-[10px] text-muted-foreground">URL</label><Input value={(httpReq.uri as string) ?? ""} onChange={(e) => updateHttp("uri", e.target.value)} placeholder="https://api.example.com/path" className="h-7 text-xs font-mono" /></div>
        <div>
          <label className="text-[10px] text-muted-foreground">Method</label>
          <Select value={(httpReq.method as string) ?? "GET"} onValueChange={(v) => updateHttp("method", v)}>
            <SelectTrigger className="h-7 text-xs w-24"><SelectValue /></SelectTrigger>
            <SelectContent>{["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"].map((m) => (<SelectItem key={m} value={m}>{m}</SelectItem>))}</SelectContent>
          </Select>
        </div>
      </div>
      <div>
        <label className="text-[10px] text-muted-foreground">Headers (JSON)</label>
        <textarea className="w-full h-14 text-[10px] font-mono rounded-md border bg-muted p-1.5 resize-y" value={typeof httpReq.headers === "string" ? httpReq.headers : JSON.stringify(httpReq.headers ?? {}, null, 2)} onChange={(e) => { try { updateHttp("headers", JSON.parse(e.target.value)); } catch { updateHttp("headers", e.target.value); } }} placeholder='{"Content-Type": "application/json"}' />
      </div>
      {["POST", "PUT", "PATCH"].includes((httpReq.method as string) ?? "") && (
        <div>
          <label className="text-[10px] text-muted-foreground">Body (JSON)</label>
          <textarea className="w-full h-14 text-[10px] font-mono rounded-md border bg-muted p-1.5 resize-y" value={typeof httpReq.body === "string" ? httpReq.body : JSON.stringify(httpReq.body ?? {}, null, 2)} onChange={(e) => { try { updateHttp("body", JSON.parse(e.target.value)); } catch { updateHttp("body", e.target.value); } }} placeholder='{"key": "value"}' />
        </div>
      )}
      <div className="grid grid-cols-2 gap-2">
        <div><label className="text-[10px] text-muted-foreground">Connection Timeout (ms)</label><Input type="number" value={(httpReq.connectionTimeOut as number) ?? 3000} onChange={(e) => updateHttp("connectionTimeOut", Number(e.target.value))} className="h-7 text-xs" /></div>
        <div><label className="text-[10px] text-muted-foreground">Read Timeout (ms)</label><Input type="number" value={(httpReq.readTimeOut as number) ?? 3000} onChange={(e) => updateHttp("readTimeOut", Number(e.target.value))} className="h-7 text-xs" /></div>
      </div>
    </div>
  );
}

function SubWorkflowFields({ editingTask, setEditingTask, workflowDefOptions, workflowName, tasks, queryClient }: {
  editingTask: DesignerTask; setEditingTask: (t: DesignerTask) => void;
  workflowDefOptions: { label: string; value: string }[]; workflowName: string;
  tasks: DesignerTask[]; queryClient: ReturnType<typeof useQueryClient>;
}) {
  if (editingTask.type !== "SUB_WORKFLOW") return null;
  return (
    <div className="space-y-2 border-t pt-2">
      <label className="text-xs font-medium text-muted-foreground">Sub-Workflow</label>
      <SearchableSelect
        options={workflowDefOptions.map((o) => ({ ...o, value: o.label.split(" (v")[0] }))}
        value={editingTask.subWorkflowParam?.name ?? ""}
        onChange={(v) => setEditingTask({ ...editingTask, subWorkflowParam: { ...editingTask.subWorkflowParam, name: v } })}
        placeholder="Select workflow..."
      />
      <div className="border-t pt-2 mt-2">
        <p className="text-[10px] text-muted-foreground mb-2">Or create a new linked sub-workflow:</p>
        <Button variant="outline" size="sm" className="w-full text-xs gap-1.5" onClick={() => {
          const parentName = workflowName || "parent";
          const subName = `${parentName}_sub_${tasks.length + 1}`;
          const subDef: WorkflowDef = { name: subName, version: 1, description: `Sub-workflow of ${parentName}`, tasks: [], inputParameters: [] };
          metadataApi.registerWorkflowDef(subDef).then(() => {
            queryClient.invalidateQueries({ queryKey: ["workflowDefs"] });
            setEditingTask({ ...editingTask, subWorkflowParam: { name: subName, version: 1 } });
            toast.success(`Created "${subName}" — open it in a new tab to design it`);
          }).catch((err: Error) => toast.error(`Failed: ${err.message}`));
        }}>
          <Link2 className="h-3 w-3" />Create Linked Sub-Workflow
        </Button>
      </div>
    </div>
  );
}
