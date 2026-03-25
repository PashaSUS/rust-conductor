import { useState, useCallback, useRef, useMemo } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  addEdge,
  applyNodeChanges,
  applyEdgeChanges,
  MarkerType,
  Handle,
  Position,
  useReactFlow,
  ReactFlowProvider,
  type Node,
  type Edge,
  type OnNodesChange,
  type OnEdgesChange,
  type OnConnect,
  type Connection,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { metadataApi, type WorkflowDef, type WorkflowTask } from "@/api/conductor";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { SearchableSelect } from "@/components/SearchableSelect";
import { JsonView } from "@/components/JsonView";
import { Save, Plus, Trash2, Code, Eye, Play, Flag } from "lucide-react";
import { toast } from "sonner";

const TASK_TYPES = [
  "SIMPLE",
  "HTTP",
  "SUB_WORKFLOW",
  "FORK_JOIN",
  "JOIN",
  "DECISION",
  "SWITCH",
  "DO_WHILE",
  "WAIT",
  "EVENT",
  "TERMINATE",
  "DYNAMIC",
] as const;

const TASK_TYPE_COLORS: Record<string, string> = {
  SIMPLE: "#3b82f6",
  HTTP: "#8b5cf6",
  SUB_WORKFLOW: "#06b6d4",
  FORK_JOIN: "#f59e0b",
  JOIN: "#f59e0b",
  DECISION: "#ec4899",
  SWITCH: "#ec4899",
  DO_WHILE: "#10b981",
  WAIT: "#6b7280",
  EVENT: "#f97316",
  TERMINATE: "#ef4444",
  DYNAMIC: "#14b8a6",
  WORKFLOW_START: "#22c55e",
  WORKFLOW_END: "#ef4444",
};

// ── Custom node: workflow start ──
function WorkflowStartNode({ data, selected }: { data: { label: string; inputKeys: string[] }; selected: boolean }) {
  return (
    <div
      className={`rounded-lg px-4 py-3 min-w-40 ${selected ? "ring-2 ring-primary" : ""}`}
      style={{ border: "2px solid #22c55e", background: "#22c55e15" }}
    >
      <div className="flex items-center gap-2 mb-1">
        <Play className="h-3.5 w-3.5 text-green-500" />
        <span className="text-xs font-bold text-green-600 dark:text-green-400">WORKFLOW START</span>
      </div>
      {data.inputKeys.length > 0 && (
        <div className="mt-1.5 space-y-0.5">
          <span className="text-[9px] text-muted-foreground font-medium">Input parameters:</span>
          {data.inputKeys.map((key) => (
            <div key={key} className="flex items-center gap-1">
              <span className="w-1.5 h-1.5 rounded-full bg-green-500 shrink-0" />
              <span className="text-[10px] font-mono">{key}</span>
            </div>
          ))}
        </div>
      )}
      <Handle type="source" position={Position.Bottom} className="w-2! h-2! bg-green-500!" />
    </div>
  );
}

// ── Custom node: workflow end ──
function WorkflowEndNode({ data, selected }: { data: { label: string; outputKeys: string[] }; selected: boolean }) {
  return (
    <div
      className={`rounded-lg px-4 py-3 min-w-40 ${selected ? "ring-2 ring-primary" : ""}`}
      style={{ border: "2px solid #ef4444", background: "#ef444415" }}
    >
      <Handle type="target" position={Position.Top} className="w-2! h-2! bg-red-500!" />
      <div className="flex items-center gap-2 mb-1">
        <Flag className="h-3.5 w-3.5 text-red-500" />
        <span className="text-xs font-bold text-red-600 dark:text-red-400">WORKFLOW END</span>
      </div>
      {data.outputKeys.length > 0 && (
        <div className="mt-1.5 space-y-0.5">
          <span className="text-[9px] text-muted-foreground font-medium">Output parameters:</span>
          {data.outputKeys.map((key) => (
            <div key={key} className="flex items-center gap-1">
              <span className="w-1.5 h-1.5 rounded-full bg-red-500 shrink-0" />
              <span className="text-[10px] font-mono">{key}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ── Custom node for the designer task ──
function DesignerTaskNode({ data, selected }: { data: { label: string; taskType: string; refName: string; inputKeys: string[]; outputKeys: string[]; onEdit: () => void; onDelete: () => void }; selected: boolean }) {
  const color = TASK_TYPE_COLORS[data.taskType] ?? "#9ca3af";

  return (
    <div
      className={`rounded-lg px-3 py-2 min-w-44 ${selected ? "ring-2 ring-primary" : ""}`}
      style={{ border: `2px solid ${color}`, background: `${color}10` }}
    >
      <Handle type="target" position={Position.Top} className="w-2! h-2! bg-border!" />
      <div className="flex items-center justify-between gap-2">
        <div className="min-w-0">
          <div className="text-xs font-semibold truncate max-w-32">{data.label}</div>
          <Badge variant="outline" className="text-[9px] px-1 py-0 mt-0.5" style={{ borderColor: color, color }}>
            {data.taskType}
          </Badge>
        </div>
        <div className="flex flex-col gap-0.5 shrink-0">
          <button onClick={data.onEdit} className="text-muted-foreground hover:text-foreground p-0.5">
            <Code className="h-3 w-3" />
          </button>
          <button onClick={data.onDelete} className="text-muted-foreground hover:text-destructive p-0.5">
            <Trash2 className="h-3 w-3" />
          </button>
        </div>
      </div>
      {/* Input/Output parameter hints */}
      {(data.inputKeys.length > 0 || data.outputKeys.length > 0) && (
        <div className="mt-1.5 border-t border-border pt-1 space-y-0.5">
          {data.inputKeys.length > 0 && (
            <div className="flex items-center gap-1">
              <span className="text-[8px] text-muted-foreground w-4">IN:</span>
              <span className="text-[9px] font-mono text-muted-foreground truncate">{data.inputKeys.join(", ")}</span>
            </div>
          )}
          {data.outputKeys.length > 0 && (
            <div className="flex items-center gap-1">
              <span className="text-[8px] text-muted-foreground w-4">OUT:</span>
              <span className="text-[9px] font-mono text-muted-foreground truncate">{data.outputKeys.join(", ")}</span>
            </div>
          )}
        </div>
      )}
      <Handle type="source" position={Position.Bottom} className="w-2! h-2! bg-border!" />
    </div>
  );
}

const nodeTypes = {
  designerTask: DesignerTaskNode,
  workflowStart: WorkflowStartNode,
  workflowEnd: WorkflowEndNode,
};

interface DesignerTask {
  id: string;
  name: string;
  taskReferenceName: string;
  type: string;
  inputParameters: Record<string, unknown>;
  subWorkflowParam?: { name: string; version?: number };
  optional?: boolean;
}

let nextId = 1;

/**
 * #180 — Drag-and-drop workflow designer using @xyflow/react.
 * Allows visual construction of workflow definitions with task palette,
 * connection snapping, and export to JSON.
 */
export default function WorkflowDesigner() {
  return (
    <ReactFlowProvider>
      <WorkflowDesignerInner />
    </ReactFlowProvider>
  );
}

function WorkflowDesignerInner() {
  const queryClient = useQueryClient();
  const { screenToFlowPosition } = useReactFlow();
  const [workflowName, setWorkflowName] = useState("my_workflow");
  const [workflowVersion, setWorkflowVersion] = useState(1);
  const [workflowDesc, setWorkflowDesc] = useState("");
  const [workflowInputKeys, setWorkflowInputKeys] = useState<string[]>([]);
  const [workflowOutputKeys, setWorkflowOutputKeys] = useState<string[]>([]);
  const [tasks, setTasks] = useState<DesignerTask[]>([]);
  const [nodes, setNodes] = useState<Node[]>([]);
  const [edges, setEdges] = useState<Edge[]>([]);
  const [editingTask, setEditingTask] = useState<DesignerTask | null>(null);
  const [showPreview, setShowPreview] = useState(false);
  const [hasStart, setHasStart] = useState(false);
  const [hasEnd, setHasEnd] = useState(false);

  const reactFlowRef = useRef<HTMLDivElement>(null);

  // Fetch registered task definitions for SIMPLE task selection
  const { data: registeredTaskDefs } = useQuery({
    queryKey: ["task-defs"],
    queryFn: metadataApi.listTaskDefs,
  });

  const taskDefOptions = useMemo(() => {
    if (!registeredTaskDefs) return [];
    return registeredTaskDefs.map((td) => ({
      label: td.name,
      value: td.name,
    }));
  }, [registeredTaskDefs]);

  const onNodesChange: OnNodesChange = useCallback(
    (changes) => setNodes((nds) => applyNodeChanges(changes, nds)),
    []
  );

  const onEdgesChange: OnEdgesChange = useCallback(
    (changes) => setEdges((eds) => applyEdgeChanges(changes, eds)),
    []
  );

  const onConnect: OnConnect = useCallback(
    (conn: Connection) =>
      setEdges((eds) =>
        addEdge(
          { ...conn, markerEnd: { type: MarkerType.ArrowClosed }, animated: true, style: { stroke: "#9ca3af" } },
          eds
        )
      ),
    []
  );

  const addStartNode = useCallback(() => {
    if (hasStart) return;
    const id = "__workflow_start__";
    const newNode: Node = {
      id,
      type: "workflowStart",
      position: { x: 200, y: 20 },
      data: { label: "Start", inputKeys: workflowInputKeys },
      deletable: false,
    };
    setNodes((prev) => [newNode, ...prev]);
    setHasStart(true);
  }, [hasStart, workflowInputKeys]);

  const addEndNode = useCallback(() => {
    if (hasEnd) return;
    const id = "__workflow_end__";
    const yPos = 20 + (nodes.length) * 120;
    const newNode: Node = {
      id,
      type: "workflowEnd",
      position: { x: 200, y: yPos },
      data: { label: "End", outputKeys: workflowOutputKeys },
      deletable: false,
    };
    setNodes((prev) => [...prev, newNode]);
    setHasEnd(true);
  }, [hasEnd, nodes.length, workflowOutputKeys]);

  // Update start/end node data when keys change
  const updateStartEndNodes = useCallback(() => {
    setNodes((prev) =>
      prev.map((n) => {
        if (n.id === "__workflow_start__") {
          return { ...n, data: { ...n.data, inputKeys: workflowInputKeys } };
        }
        if (n.id === "__workflow_end__") {
          return { ...n, data: { ...n.data, outputKeys: workflowOutputKeys } };
        }
        return n;
      })
    );
  }, [workflowInputKeys, workflowOutputKeys]);

  const addTask = useCallback((type: string, position?: { x: number; y: number }) => {
    const id = `task_${nextId++}`;
    const refName = `${type.toLowerCase()}_${id}`;
    const task: DesignerTask = {
      id,
      name: refName,
      taskReferenceName: refName,
      type,
      inputParameters: {},
    };
    setTasks((prev) => [...prev, task]);

    // Count non-special nodes for vertical stacking
    const taskNodeCount = nodes.filter((n) => n.type === "designerTask").length;
    const pos = position ?? { x: 200, y: 80 + taskNodeCount * 120 };
    const newNode: Node = {
      id,
      type: "designerTask",
      position: pos,
      data: {
        label: refName,
        taskType: type,
        refName,
        inputKeys: Object.keys(task.inputParameters),
        outputKeys: [] as string[],
        onEdit: () => setEditingTask(task),
        onDelete: () => removeTask(id),
      },
    };
    setNodes((prev) => [...prev, newNode]);
  }, [nodes]);

  const removeTask = useCallback((id: string) => {
    setTasks((prev) => prev.filter((t) => t.id !== id));
    setNodes((prev) => prev.filter((n) => n.id !== id));
    setEdges((prev) => prev.filter((e) => e.source !== id && e.target !== id));
    if (editingTask?.id === id) setEditingTask(null);
  }, [editingTask]);

  const updateTask = useCallback((updated: DesignerTask) => {
    setTasks((prev) => prev.map((t) => (t.id === updated.id ? updated : t)));
    setNodes((prev) =>
      prev.map((n) =>
        n.id === updated.id
          ? {
              ...n,
              data: {
                ...n.data,
                label: updated.taskReferenceName,
                taskType: updated.type,
                refName: updated.taskReferenceName,
                inputKeys: Object.keys(updated.inputParameters),
                outputKeys: [] as string[],
                onEdit: () => setEditingTask(updated),
                onDelete: () => removeTask(updated.id),
              },
            }
          : n
      )
    );
    setEditingTask(null);
  }, [removeTask]);

  // Build the ordered task list from edges (topological sort by connections)
  const orderedTasks = useMemo((): WorkflowTask[] => {
    const taskMap = new Map(tasks.map((t) => [t.id, t]));
    const adj = new Map<string, string[]>();
    const inDeg = new Map<string, number>();
    for (const t of tasks) {
      adj.set(t.id, []);
      inDeg.set(t.id, 0);
    }
    for (const e of edges) {
      adj.get(e.source)?.push(e.target);
      inDeg.set(e.target, (inDeg.get(e.target) ?? 0) + 1);
    }

    // Kahn's algorithm
    const queue = tasks.filter((t) => (inDeg.get(t.id) ?? 0) === 0).map((t) => t.id);
    const ordered: string[] = [];
    while (queue.length > 0) {
      const cur = queue.shift()!;
      ordered.push(cur);
      for (const next of adj.get(cur) ?? []) {
        const d = (inDeg.get(next) ?? 1) - 1;
        inDeg.set(next, d);
        if (d === 0) queue.push(next);
      }
    }

    // Add any remaining (disconnected) tasks
    for (const t of tasks) {
      if (!ordered.includes(t.id)) ordered.push(t.id);
    }

    return ordered
      .map((id) => taskMap.get(id))
      .filter(Boolean)
      .map((t) => {
        const wt: WorkflowTask = {
          name: t!.name,
          taskReferenceName: t!.taskReferenceName,
          type: t!.type,
          inputParameters: t!.inputParameters,
        };
        if (t!.optional) wt.optional = true;
        if (t!.subWorkflowParam) wt.subWorkflowParam = t!.subWorkflowParam;
        return wt;
      });
  }, [tasks, edges]);

  const workflowDef: WorkflowDef = {
    name: workflowName,
    version: workflowVersion,
    description: workflowDesc,
    tasks: orderedTasks,
  };

  const saveMut = useMutation({
    mutationFn: () => metadataApi.registerWorkflowDef(workflowDef),
    onSuccess: () => {
      toast.success("Workflow saved successfully");
      queryClient.invalidateQueries({ queryKey: ["workflowDefs"] });
    },
    onError: (err: Error) => toast.error(`Save failed: ${err.message}`),
  });

  // Drop handler using screenToFlowPosition for accurate placement
  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();
      const type = event.dataTransfer.getData("application/task-type");
      if (!type) return;

      const position = screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });
      addTask(type, position);
    },
    [addTask, screenToFlowPosition]
  );

  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
  }, []);

  // Input keys string editor helpers
  const [inputKeyDraft, setInputKeyDraft] = useState("");
  const [outputKeyDraft, setOutputKeyDraft] = useState("");

  const addInputKey = () => {
    const key = inputKeyDraft.trim();
    if (key && !workflowInputKeys.includes(key)) {
      setWorkflowInputKeys((prev) => [...prev, key]);
      setInputKeyDraft("");
      setTimeout(updateStartEndNodes, 0);
    }
  };
  const removeInputKey = (k: string) => {
    setWorkflowInputKeys((prev) => prev.filter((x) => x !== k));
    setTimeout(updateStartEndNodes, 0);
  };
  const addOutputKey = () => {
    const key = outputKeyDraft.trim();
    if (key && !workflowOutputKeys.includes(key)) {
      setWorkflowOutputKeys((prev) => [...prev, key]);
      setOutputKeyDraft("");
      setTimeout(updateStartEndNodes, 0);
    }
  };
  const removeOutputKey = (k: string) => {
    setWorkflowOutputKeys((prev) => prev.filter((x) => x !== k));
    setTimeout(updateStartEndNodes, 0);
  };

  return (
    <div className="space-y-4 max-w-7xl">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold tracking-tight">Workflow Designer</h2>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={() => setShowPreview(!showPreview)}>
            <Eye className="h-3 w-3 mr-1" />
            {showPreview ? "Hide" : "Preview"} JSON
          </Button>
          <Button size="sm" onClick={() => saveMut.mutate()} disabled={saveMut.isPending || tasks.length === 0}>
            <Save className="h-3 w-3 mr-1" />
            Save Workflow
          </Button>
        </div>
      </div>

      {/* Workflow metadata */}
      <div className="grid grid-cols-[1fr_6rem_1fr] gap-3 max-w-4xl">
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">Name</label>
          <Input value={workflowName} onChange={(e) => setWorkflowName(e.target.value)} />
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">Version</label>
          <Input type="number" min={1} value={workflowVersion} onChange={(e) => setWorkflowVersion(Number(e.target.value))} />
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">Description</label>
          <Input value={workflowDesc} onChange={(e) => setWorkflowDesc(e.target.value)} placeholder="Optional description" />
        </div>
      </div>

      {/* Workflow input/output keys */}
      <div className="grid grid-cols-2 gap-4 max-w-4xl">
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">Input Parameters</label>
          <div className="flex gap-1 mb-1">
            <Input
              value={inputKeyDraft}
              onChange={(e) => setInputKeyDraft(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addInputKey()}
              placeholder="Add parameter name"
              className="h-8 text-xs"
            />
            <Button variant="outline" size="sm" className="h-8 px-2" onClick={addInputKey}>
              <Plus className="h-3 w-3" />
            </Button>
          </div>
          <div className="flex flex-wrap gap-1">
            {workflowInputKeys.map((k) => (
              <Badge key={k} variant="secondary" className="text-xs gap-1">
                {k}
                <button onClick={() => removeInputKey(k)} className="hover:text-destructive">×</button>
              </Badge>
            ))}
          </div>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">Output Parameters</label>
          <div className="flex gap-1 mb-1">
            <Input
              value={outputKeyDraft}
              onChange={(e) => setOutputKeyDraft(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addOutputKey()}
              placeholder="Add parameter name"
              className="h-8 text-xs"
            />
            <Button variant="outline" size="sm" className="h-8 px-2" onClick={addOutputKey}>
              <Plus className="h-3 w-3" />
            </Button>
          </div>
          <div className="flex flex-wrap gap-1">
            {workflowOutputKeys.map((k) => (
              <Badge key={k} variant="secondary" className="text-xs gap-1">
                {k}
                <button onClick={() => removeOutputKey(k)} className="hover:text-destructive">×</button>
              </Badge>
            ))}
          </div>
        </div>
      </div>

      <div className="flex gap-4">
        {/* Task palette */}
        <div className="w-44 shrink-0 space-y-1">
          <p className="text-xs font-semibold text-muted-foreground mb-2">Drag to canvas:</p>
          {TASK_TYPES.map((type) => {
            const color = TASK_TYPE_COLORS[type] ?? "#9ca3af";
            return (
              <div
                key={type}
                draggable
                onDragStart={(e) => {
                  e.dataTransfer.setData("application/task-type", type);
                  e.dataTransfer.effectAllowed = "move";
                }}
                className="flex items-center gap-2 px-2 py-1.5 rounded border cursor-grab hover:shadow-sm transition-shadow"
                style={{ borderColor: color }}
              >
                <span className="w-2 h-2 rounded-full" style={{ backgroundColor: color }} />
                <span className="text-xs font-medium">{type}</span>
              </div>
            );
          })}
          <div className="pt-2 space-y-1">
            <Button variant="outline" size="sm" className="w-full" onClick={() => addTask("SIMPLE")}>
              <Plus className="h-3 w-3 mr-1" />
              Add Task
            </Button>
            <Button
              variant="outline"
              size="sm"
              className="w-full text-green-600"
              onClick={addStartNode}
              disabled={hasStart}
            >
              <Play className="h-3 w-3 mr-1" />
              {hasStart ? "Start Added" : "Add Start"}
            </Button>
            <Button
              variant="outline"
              size="sm"
              className="w-full text-red-600"
              onClick={addEndNode}
              disabled={hasEnd}
            >
              <Flag className="h-3 w-3 mr-1" />
              {hasEnd ? "End Added" : "Add End"}
            </Button>
          </div>
        </div>

        {/* Canvas */}
        <div
          ref={reactFlowRef}
          className="flex-1 rounded border"
          style={{ height: 550 }}
          onDrop={onDrop}
          onDragOver={onDragOver}
        >
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            nodeTypes={nodeTypes}
            fitView
            proOptions={{ hideAttribution: true }}
            snapToGrid
            snapGrid={[20, 20]}
          >
            <Background gap={20} />
            <Controls />
            <MiniMap />
          </ReactFlow>
        </div>

        {/* Task editor panel */}
        {editingTask && (
          <Card className="w-72 shrink-0">
            <CardContent className="pt-4 space-y-3">
              <h4 className="font-semibold text-sm">Edit Task</h4>

              {/* SIMPLE task: show registered task def dropdown */}
              {editingTask.type === "SIMPLE" && (
                <div>
                  <label className="text-xs text-muted-foreground">Task Definition</label>
                  <SearchableSelect
                    options={taskDefOptions}
                    value={editingTask.name}
                    onChange={(v) =>
                      setEditingTask({ ...editingTask, name: v, taskReferenceName: v })
                    }
                    placeholder="Select task definition..."
                  />
                </div>
              )}

              <div>
                <label className="text-xs text-muted-foreground">Name</label>
                <Input
                  value={editingTask.name}
                  onChange={(e) => setEditingTask({ ...editingTask, name: e.target.value })}
                  className="h-8 text-xs"
                />
              </div>
              <div>
                <label className="text-xs text-muted-foreground">Reference Name</label>
                <Input
                  value={editingTask.taskReferenceName}
                  onChange={(e) => setEditingTask({ ...editingTask, taskReferenceName: e.target.value })}
                  className="h-8 text-xs"
                />
              </div>
              <div>
                <label className="text-xs text-muted-foreground">Type</label>
                <Select
                  value={editingTask.type}
                  onValueChange={(v) => setEditingTask({ ...editingTask, type: v })}
                >
                  <SelectTrigger className="h-8 text-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {TASK_TYPES.map((t) => (
                      <SelectItem key={t} value={t}>{t}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div>
                <label className="text-xs text-muted-foreground">Input Parameters (JSON)</label>
                <textarea
                  className="w-full h-24 text-xs font-mono rounded border bg-muted p-2 resize-none"
                  value={JSON.stringify(editingTask.inputParameters, null, 2)}
                  onChange={(e) => {
                    try {
                      const parsed = JSON.parse(e.target.value);
                      setEditingTask({ ...editingTask, inputParameters: parsed });
                    } catch {
                      // Invalid JSON — let user keep typing
                    }
                  }}
                />
              </div>
              {editingTask.type === "SUB_WORKFLOW" && (
                <div>
                  <label className="text-xs text-muted-foreground">Sub-workflow Name</label>
                  <Input
                    value={editingTask.subWorkflowParam?.name ?? ""}
                    onChange={(e) =>
                      setEditingTask({
                        ...editingTask,
                        subWorkflowParam: { ...editingTask.subWorkflowParam, name: e.target.value },
                      })
                    }
                    className="h-8 text-xs"
                  />
                </div>
              )}
              <div className="flex gap-2">
                <Button size="sm" className="flex-1" onClick={() => updateTask(editingTask)}>
                  Apply
                </Button>
                <Button variant="outline" size="sm" onClick={() => setEditingTask(null)}>
                  Cancel
                </Button>
              </div>
            </CardContent>
          </Card>
        )}
      </div>

      {/* JSON preview */}
      {showPreview && (
        <Card>
          <CardContent className="pt-4">
            <h3 className="font-semibold text-sm mb-2">Workflow Definition Preview</h3>
            <JsonView data={workflowDef} maxHeight="24rem" />
          </CardContent>
        </Card>
      )}
    </div>
  );
}
