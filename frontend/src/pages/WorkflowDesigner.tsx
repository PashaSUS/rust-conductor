import { useState, useCallback, useRef, useMemo, useEffect } from "react";
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
import { metadataApi, type WorkflowDef, type WorkflowTask, type TaskDef } from "@/api/conductor";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { SearchableSelect } from "@/components/SearchableSelect";
import { JsonView } from "@/components/JsonView";
import {
  Save, Plus, Trash2, Code, Eye, Play, Flag,
  Settings2, Zap, LayoutGrid, AlertTriangle,
  ArrowDown, GripVertical, ChevronUp, ChevronDown, Copy, GripHorizontal,
} from "lucide-react";
import { toast } from "sonner";

// ── Task type configuration ──

const TASK_TYPE_GROUPS = [
  {
    label: "Worker Tasks",
    types: [
      { value: "SIMPLE", label: "Simple", desc: "Worker-polled task" },
      { value: "HTTP", label: "HTTP", desc: "HTTP API call" },
      { value: "DYNAMIC", label: "Dynamic", desc: "Dynamic task resolution" },
    ],
  },
  {
    label: "Flow Control",
    types: [
      { value: "FORK_JOIN", label: "Fork/Join", desc: "Static parallel" },
      { value: "DYNAMIC_FORK_JOIN", label: "Dynamic Fork", desc: "Dynamic parallel" },
      { value: "JOIN", label: "Join", desc: "Wait for branches" },
      { value: "DECISION", label: "Decision", desc: "Conditional branch" },
      { value: "SWITCH", label: "Switch", desc: "Multi-branch switch" },
      { value: "DO_WHILE", label: "Do While", desc: "Loop until condition" },
    ],
  },
  {
    label: "Utilities",
    types: [
      { value: "SUB_WORKFLOW", label: "Sub-Workflow", desc: "Run another workflow" },
      { value: "WAIT", label: "Wait", desc: "Wait for signal" },
      { value: "EVENT", label: "Event", desc: "Publish event" },
      { value: "TERMINATE", label: "Terminate", desc: "End workflow" },
    ],
  },
] as const;

const TASK_TYPES = TASK_TYPE_GROUPS.flatMap((g) => g.types.map((t) => t.value));

const TASK_TYPE_COLORS: Record<string, string> = {
  SIMPLE: "#3b82f6",
  HTTP: "#8b5cf6",
  SUB_WORKFLOW: "#06b6d4",
  FORK_JOIN: "#f59e0b",
  DYNAMIC_FORK_JOIN: "#d97706",
  JOIN: "#eab308",
  DECISION: "#ec4899",
  SWITCH: "#ec4899",
  DO_WHILE: "#10b981",
  WAIT: "#6b7280",
  EVENT: "#f97316",
  TERMINATE: "#ef4444",
  DYNAMIC: "#14b8a6",
};

const DATA_EDGE_STYLE = {
  markerEnd: { type: MarkerType.ArrowClosed as const, width: 12, height: 12 },
  animated: false,
  style: { stroke: "#3b82f6", strokeWidth: 1.5, strokeDasharray: "6 3" },
};

// ── Context menu component ──
interface ContextMenuItem {
  label: string;
  icon?: React.ReactNode;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
  separator?: boolean;
}

function ContextMenu({ x, y, items, onClose }: { x: number; y: number; items: ContextMenuItem[]; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as HTMLElement)) onClose();
    };
    const keyHandler = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    document.addEventListener("mousedown", handler);
    document.addEventListener("keydown", keyHandler);
    return () => { document.removeEventListener("mousedown", handler); document.removeEventListener("keydown", keyHandler); };
  }, [onClose]);

  return (
    <div
      ref={ref}
      style={{ position: "fixed", top: y, left: x, zIndex: 9999 }}
      className="min-w-45 rounded-lg border bg-popover text-popover-foreground shadow-lg py-1 animate-in fade-in-0 zoom-in-95"
    >
      {items.map((item, i) =>
        item.separator ? (
          <div key={i} className="h-px bg-border my-1" />
        ) : (
          <button
            key={i}
            onClick={() => { item.onClick(); onClose(); }}
            disabled={item.disabled}
            className={`w-full flex items-center gap-2 px-3 py-1.5 text-xs text-left transition-colors
              ${item.danger ? "text-destructive hover:bg-destructive/10" : "hover:bg-accent"}
              ${item.disabled ? "opacity-40 cursor-not-allowed" : "cursor-pointer"}`}
          >
            {item.icon && <span className="w-4 h-4 flex items-center justify-center shrink-0">{item.icon}</span>}
            {item.label}
          </button>
        )
      )}
    </div>
  );
}

// ── Custom node: workflow start (workflow inputs = data sources) ──
function WorkflowStartNode({ data, selected }: { data: { label: string; inputKeys: string[] }; selected: boolean }) {
  return (
    <div
      className={`rounded-xl px-5 py-3 min-w-48 shadow-md ${selected ? "ring-2 ring-primary" : ""}`}
      style={{ border: "2px solid #22c55e", background: "var(--color-card, #fff)" }}
    >
      <div className="flex items-center gap-2 mb-1">
        <div className="w-7 h-7 rounded-full flex items-center justify-center bg-green-500/15">
          <Play className="h-3.5 w-3.5 text-green-600" />
        </div>
        <span className="text-xs font-bold text-green-600 dark:text-green-400">START</span>
        <span className="text-[9px] text-muted-foreground ml-auto">workflow inputs</span>
      </div>
      {data.inputKeys.length > 0 && (
        <div className="mt-2 space-y-1.5">
          {data.inputKeys.map((key) => (
            <div key={key} className="flex items-center justify-end gap-1.5 relative">
              <span className="text-[10px] font-mono text-emerald-600 dark:text-emerald-400">{key}</span>
              <Handle
                type="source"
                position={Position.Right}
                id={`out-${key}`}
                style={{ top: "auto", right: -8, position: "absolute" }}
                className="w-2.5! h-2.5! bg-emerald-500! border! border-background! rounded-full!"
              />
            </div>
          ))}
        </div>
      )}
      {data.inputKeys.length === 0 && (
        <p className="text-[9px] text-muted-foreground mt-1">Add input parameters in Settings to create output ports</p>
      )}
    </div>
  );
}

// ── Custom node: workflow end (collects task outputs) ──
function WorkflowEndNode({ data, selected }: { data: { label: string; connectedOutputs: string[] }; selected: boolean }) {
  return (
    <div
      className={`rounded-xl px-5 py-3 min-w-48 shadow-md ${selected ? "ring-2 ring-primary" : ""}`}
      style={{ border: "2px solid #ef4444", background: "var(--color-card, #fff)" }}
    >
      <Handle
        type="target"
        position={Position.Left}
        id="data-in"
        className="w-3! h-3! bg-blue-500! border-2! border-background! rounded-full!"
      />
      <div className="flex items-center gap-2 mb-1">
        <div className="w-7 h-7 rounded-full flex items-center justify-center bg-red-500/15">
          <Flag className="h-3.5 w-3.5 text-red-600" />
        </div>
        <span className="text-xs font-bold text-red-600 dark:text-red-400">END</span>
        <span className="text-[9px] text-muted-foreground ml-auto">workflow output</span>
      </div>
      {data.connectedOutputs.length > 0 ? (
        <div className="mt-2 space-y-1">
          {data.connectedOutputs.map((entry) => (
            <div key={entry} className="flex items-center gap-1.5">
              <span className="w-1.5 h-1.5 rounded-full bg-red-500 shrink-0" />
              <span className="text-[9px] font-mono text-muted-foreground truncate">{entry}</span>
            </div>
          ))}
        </div>
      ) : (
        <p className="text-[9px] text-muted-foreground mt-1">Connect task outputs here to define workflow output</p>
      )}
    </div>
  );
}

// ── Custom task node with step number + named I/O handles ──
function DesignerTaskNode({ data, selected }: {
  data: {
    label: string; taskType: string; refName: string;
    stepNumber: number;
    inputKeys: string[]; outputKeys: string[];
    onEdit: () => void; onDelete: () => void;
  };
  selected: boolean;
}) {
  const color = TASK_TYPE_COLORS[data.taskType] ?? "#9ca3af";
  const inputKeys: string[] = data.inputKeys || [];
  const outputKeys: string[] = data.outputKeys || [];
  return (
    <div
      className={`rounded-xl min-w-56 shadow-md transition-shadow hover:shadow-lg ${selected ? "ring-2 ring-primary" : ""}`}
      style={{ border: `2px solid ${color}`, background: "var(--color-card, #fff)" }}
    >
      {/* Header bar with step number */}
      <div className="px-3 py-1.5 flex items-center gap-2 border-b border-border/30" style={{ background: `${color}15`, borderRadius: "10px 10px 0 0" }}>
        <span
          className="w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold text-white shrink-0"
          style={{ backgroundColor: color }}
        >
          {data.stepNumber}
        </span>
        <span className="text-[10px] font-bold tracking-wide uppercase" style={{ color }}>{data.taskType.replace(/_/g, " ")}</span>
        <div className="ml-auto flex gap-1">
          <button onClick={data.onEdit} className="text-muted-foreground hover:text-foreground p-0.5 rounded hover:bg-muted" title="Edit task">
            <Code className="h-3 w-3" />
          </button>
          <button onClick={data.onDelete} className="text-muted-foreground hover:text-destructive p-0.5 rounded hover:bg-muted" title="Delete task">
            <Trash2 className="h-3 w-3" />
          </button>
        </div>
      </div>

      {/* Body: I/O ports */}
      <div className="px-3 py-2">
        <div className="text-xs font-semibold truncate max-w-48 mb-1.5">{data.label}</div>

        <div className="flex gap-4">
          {/* Left column: inputs */}
          <div className="flex-1 min-w-0 space-y-1">
            {inputKeys.length > 0 && (
              <>
                <div className="text-[7px] font-bold text-blue-500 uppercase tracking-wider">Inputs</div>
                {inputKeys.slice(0, 8).map((k) => (
                  <div key={k} className="flex items-center gap-1 relative">
                    <Handle
                      type="target"
                      position={Position.Left}
                      id={`in-${k}`}
                      style={{ top: "auto", left: -8, position: "absolute" }}
                      className="w-2! h-2! bg-blue-500! border! border-background! rounded-full!"
                    />
                    <span className="text-[9px] font-mono text-blue-600 dark:text-blue-400 truncate ml-0.5">{k}</span>
                  </div>
                ))}
                {inputKeys.length > 8 && (
                  <span className="text-[8px] text-muted-foreground">+{inputKeys.length - 8} more</span>
                )}
              </>
            )}
          </div>

          {/* Right column: outputs */}
          <div className="flex-1 min-w-0 space-y-1">
            {outputKeys.length > 0 && (
              <>
                <div className="text-[7px] font-bold text-emerald-500 uppercase tracking-wider text-right">Outputs</div>
                {outputKeys.slice(0, 8).map((k) => (
                  <div key={k} className="flex items-center justify-end gap-1 relative">
                    <span className="text-[9px] font-mono text-emerald-600 dark:text-emerald-400 truncate mr-0.5">{k}</span>
                    <Handle
                      type="source"
                      position={Position.Right}
                      id={`out-${k}`}
                      style={{ top: "auto", right: -8, position: "absolute" }}
                      className="w-2! h-2! bg-emerald-500! border! border-background! rounded-full!"
                    />
                  </div>
                ))}
                {outputKeys.length > 8 && (
                  <span className="text-[8px] text-muted-foreground float-right">+{outputKeys.length - 8} more</span>
                )}
              </>
            )}
            {outputKeys.length === 0 && (
              <div className="text-right">
                <div className="text-[7px] font-bold text-emerald-500 uppercase tracking-wider">Output</div>
                <div className="flex items-center justify-end gap-1 relative">
                  <span className="text-[9px] font-mono text-emerald-600 dark:text-emerald-400">result</span>
                  <Handle
                    type="source"
                    position={Position.Right}
                    id="out-result"
                    style={{ top: "auto", right: -8, position: "absolute" }}
                    className="w-2! h-2! bg-emerald-500! border! border-background! rounded-full!"
                  />
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
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
  outputKeys: string[];
  subWorkflowParam?: { name: string; version?: number };
  optional?: boolean;
}

export default function WorkflowDesigner() {
  return (
    <ReactFlowProvider>
      <WorkflowDesignerInner />
    </ReactFlowProvider>
  );
}

function WorkflowDesignerInner() {
  const queryClient = useQueryClient();
  const { screenToFlowPosition, fitView } = useReactFlow();

  // Stable ID counter (survives re-renders, resets on remount)
  const nextIdRef = useRef(1);

  // Workflow metadata
  const [workflowName, setWorkflowName] = useState("my_workflow");
  const [workflowVersion, setWorkflowVersion] = useState(1);
  const [workflowDesc, setWorkflowDesc] = useState("");
  const [workflowInputKeys, setWorkflowInputKeys] = useState<string[]>([]);

  // Canvas state
  const [tasks, setTasks] = useState<DesignerTask[]>([]);
  const [nodes, setNodes] = useState<Node[]>([]);
  const [edges, setEdges] = useState<Edge[]>([]);
  const [editingTask, setEditingTask] = useState<DesignerTask | null>(null);

  // UI toggles
  const [showPreview, setShowPreview] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [isDragOver, setIsDragOver] = useState(false);

  // Context menu state
  const [ctxMenu, setCtxMenu] = useState<{
    x: number; y: number;
    items: ContextMenuItem[];
  } | null>(null);

  const reactFlowRef = useRef<HTMLDivElement>(null);

  // Stable callback refs — used inside node data so closures never go stale
  const tasksRef = useRef(tasks);
  tasksRef.current = tasks;
  const setEditingTaskRef = useRef(setEditingTask);
  setEditingTaskRef.current = setEditingTask;
  const removeTaskRef = useRef<(id: string) => void>(() => {});

  // Resizable right panel
  const [rightPanelWidth, setRightPanelWidth] = useState(320);
  const resizingRef = useRef(false);
  const resizeStartX = useRef(0);
  const resizeStartW = useRef(320);

  const onResizeStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    resizingRef.current = true;
    resizeStartX.current = e.clientX;
    resizeStartW.current = rightPanelWidth;
    const onMove = (ev: MouseEvent) => {
      if (!resizingRef.current) return;
      const delta = resizeStartX.current - ev.clientX;
      setRightPanelWidth(Math.max(260, Math.min(600, resizeStartW.current + delta)));
    };
    const onUp = () => {
      resizingRef.current = false;
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  }, [rightPanelWidth]);

  // Fetch data
  const { data: registeredTaskDefs } = useQuery({
    queryKey: ["task-defs"],
    queryFn: metadataApi.listTaskDefs,
  });
  const { data: workflowDefs } = useQuery({
    queryKey: ["workflowDefs"],
    queryFn: metadataApi.listWorkflowDefs,
  });

  // Build lookup for task def I/O keys
  const taskDefMap = useMemo(() => {
    const m = new Map<string, TaskDef>();
    if (registeredTaskDefs) {
      for (const td of registeredTaskDefs) m.set(td.name, td);
    }
    return m;
  }, [registeredTaskDefs]);

  const taskDefOptions = useMemo(() => {
    if (!registeredTaskDefs) return [];
    return registeredTaskDefs.map((td) => ({ label: td.name, value: td.name }));
  }, [registeredTaskDefs]);

  const workflowDefOptions = useMemo(() => {
    if (!workflowDefs) return [];
    return workflowDefs.map((wd) => ({ label: `${wd.name} (v${wd.version})`, value: `${wd.name}::${wd.version}` }));
  }, [workflowDefs]);

  // Derive workflow output params from tasks connected to END
  const workflowOutputParams = useMemo(() => {
    const endDataEdges = edges.filter(
      (e) => e.target === "__end__" && e.targetHandle === "data-in"
    );
    const out: Record<string, string> = {};
    for (const edge of endDataEdges) {
      const task = tasks.find((t) => t.id === edge.source);
      if (!task) continue;
      const outputKey = edge.sourceHandle?.replace("out-", "") ?? "result";
      const key = `${task.taskReferenceName}_${outputKey}`;
      out[key] = `\${${task.taskReferenceName}.output.${outputKey}}`;
    }
    return out;
  }, [edges, tasks]);

  // Update the end node's displayed outputs whenever connections change
  useEffect(() => {
    const labels = Object.keys(workflowOutputParams);
    setNodes((prev) =>
      prev.map((n) =>
        n.id === "__end__"
          ? { ...n, data: { ...n.data, connectedOutputs: labels } }
          : n
      )
    );
  }, [workflowOutputParams]);

  // ── Helpers ──
  const hasStart = useMemo(() => nodes.some((n) => n.id === "__start__"), [nodes]);

  // ── Handlers ──

  const onNodesChange: OnNodesChange = useCallback(
    (changes) => setNodes((nds) => applyNodeChanges(changes, nds)),
    []
  );
  const onEdgesChange: OnEdgesChange = useCallback(
    (changes) => setEdges((eds) => applyEdgeChanges(changes, eds)),
    []
  );

  // Connection handler: data connections only
  const onConnect: OnConnect = useCallback(
    (conn: Connection) => {
      const sourceHandle = conn.sourceHandle ?? "";
      const targetHandle = conn.targetHandle ?? "";

      const isOutputHandle = sourceHandle.startsWith("out-");
      const isInputHandle = targetHandle.startsWith("in-");
      const isEndDataIn = targetHandle === "data-in";

      if (!isOutputHandle || !(isInputHandle || isEndDataIn)) return;

      const outputKey = sourceHandle.replace("out-", "");

      if (isInputHandle) {
        const inputKey = targetHandle.replace("in-", "");
        const sourceTaskId = conn.source;
        const targetTaskId = conn.target;

        setTasks((prev) => {
          if (sourceTaskId === "__start__") {
            const expression = `\${workflow.input.${outputKey}}`;
            return prev.map((t) =>
              t.id === targetTaskId
                ? { ...t, inputParameters: { ...t.inputParameters, [inputKey]: expression } }
                : t
            );
          }
          const sourceTask = prev.find((t) => t.id === sourceTaskId);
          if (!sourceTask) return prev;
          const expression = `\${${sourceTask.taskReferenceName}.output.${outputKey}}`;
          return prev.map((t) =>
            t.id === targetTaskId
              ? { ...t, inputParameters: { ...t.inputParameters, [inputKey]: expression } }
              : t
          );
        });
      }

      const edgeLabel = isInputHandle
        ? `${outputKey} → ${targetHandle.replace("in-", "")}`
        : outputKey;

      setEdges((eds) => addEdge({
        ...conn,
        ...DATA_EDGE_STYLE,
        label: edgeLabel,
        labelStyle: { fontSize: 8, fill: "#3b82f6" },
        labelBgStyle: { fill: "var(--color-card, #fff)", fillOpacity: 0.9 },
        labelBgPadding: [4, 2] as [number, number],
      }, eds));
    },
    []
  );

  // ── Ensure start/end ──
  const ensureStartEnd = useCallback((taskCount: number) => {
    setNodes((prev) => {
      let result = [...prev];
      if (!result.some((n) => n.id === "__start__")) {
        result = [{
          id: "__start__",
          type: "workflowStart",
          position: { x: 250, y: 0 },
          data: { label: "Start", inputKeys: workflowInputKeys },
          deletable: false,
        }, ...result];
      }
      if (!result.some((n) => n.id === "__end__")) {
        result = [...result, {
          id: "__end__",
          type: "workflowEnd",
          position: { x: 250, y: 140 + taskCount * 140 },
          data: { label: "End", connectedOutputs: [] as string[] },
          deletable: false,
        }];
      }
      return result;
    });
  }, [workflowInputKeys]);

  // ── Helper to build node data ──
  const makeNodeData = useCallback((task: DesignerTask, stepNumber: number) => ({
    label: task.taskReferenceName,
    taskType: task.type,
    refName: task.taskReferenceName,
    stepNumber,
    inputKeys: Object.keys(task.inputParameters),
    outputKeys: task.outputKeys,
    onEdit: () => {
      // Use ref to always get the freshest task from the array
      const fresh = tasksRef.current.find((t) => t.id === task.id);
      setEditingTaskRef.current(fresh ?? task);
    },
    onDelete: () => removeTaskRef.current(task.id),
  }), []);

  // ── Refresh step numbers on all task nodes ──
  const refreshStepNumbers = useCallback((taskList: DesignerTask[]) => {
    setNodes((prev) => {
      const indexMap = new Map(taskList.map((t, i) => [t.id, i + 1]));
      return prev.map((n) => {
        if (n.type !== "designerTask") return n;
        const task = taskList.find((t) => t.id === n.id);
        const step = indexMap.get(n.id) ?? 0;
        if (!task) return n;
        return {
          ...n,
          data: {
            ...n.data,
            stepNumber: step,
            label: task.taskReferenceName,
            inputKeys: Object.keys(task.inputParameters),
            outputKeys: task.outputKeys,
            onEdit: () => {
              const fresh = tasksRef.current.find((t) => t.id === task.id);
              setEditingTaskRef.current(fresh ?? task);
            },
            onDelete: () => removeTaskRef.current(task.id),
          },
        };
      });
    });
  }, []);

  // ── Add task ──
  const addTask = useCallback((type: string, position?: { x: number; y: number }) => {
    const id = `task_${nextIdRef.current++}`;
    const refName = `${type.toLowerCase()}_${id}`;

    const task: DesignerTask = {
      id, name: refName, taskReferenceName: refName, type,
      inputParameters: {},
      outputKeys: [],
    };

    setTasks((prev) => {
      const newTasks = [...prev, task];
      ensureStartEnd(newTasks.length);
      refreshStepNumbers(newTasks);
      return newTasks;
    });

    const taskNodes = nodes.filter((n) => n.type === "designerTask");
    const lastTaskNode = taskNodes.length > 0
      ? taskNodes.reduce((a, b) => ((a.position?.y ?? 0) > (b.position?.y ?? 0) ? a : b))
      : null;

    const pos = position ?? {
      x: 250,
      y: lastTaskNode ? (lastTaskNode.position?.y ?? 0) + 140 : 120,
    };
    const stepNum = taskNodes.length + 1;

    setNodes((prev) => {
      const updated = prev.map((n) =>
        n.id === "__end__" ? { ...n, position: { x: 250, y: pos.y + 140 } } : n
      );
      return [...updated, {
        id,
        type: "designerTask",
        position: pos,
        data: makeNodeData(task, stepNum),
      }];
    });
  }, [nodes, ensureStartEnd, makeNodeData, refreshStepNumbers]);

  const removeTask = useCallback((id: string) => {
    setTasks((prev) => {
      const newTasks = prev.filter((t) => t.id !== id);
      refreshStepNumbers(newTasks);
      return newTasks;
    });
    setNodes((prev) => prev.filter((n) => n.id !== id));
    setEdges((prev) => prev.filter((e) => e.source !== id && e.target !== id));
    setEditingTask((cur) => cur?.id === id ? null : cur);
  }, [refreshStepNumbers]);
  // Keep the ref in sync so stale node-data closures always reach the latest removeTask
  removeTaskRef.current = removeTask;

  const duplicateTask = useCallback((id: string) => {
    const task = tasksRef.current.find((t) => t.id === id);
    if (!task) return;
    const newId = `task_${nextIdRef.current++}`;
    const newTask: DesignerTask = {
      ...task,
      id: newId,
      taskReferenceName: `${task.taskReferenceName}_copy`,
    };
    setTasks((prev) => {
      const idx = prev.findIndex((t) => t.id === id);
      const newTasks = [...prev];
      newTasks.splice(idx + 1, 0, newTask);
      refreshStepNumbers(newTasks);
      return newTasks;
    });
    const sourceNode = nodes.find((n) => n.id === id) ?? { position: { x: 300, y: 200 } };
    const pos = { x: sourceNode.position.x + 40, y: sourceNode.position.y + 40 };
    setNodes((prev) => [...prev, {
      id: newId,
      type: "designerTask",
      position: pos,
      data: makeNodeData(newTask, tasks.length + 1),
    }]);
    toast.success("Task duplicated");
  }, [tasks, nodes, makeNodeData, refreshStepNumbers]);

  const moveTask = useCallback((id: string, direction: "up" | "down") => {
    setTasks((prev) => {
      const idx = prev.findIndex((t) => t.id === id);
      if (idx < 0) return prev;
      const newIdx = direction === "up" ? idx - 1 : idx + 1;
      if (newIdx < 0 || newIdx >= prev.length) return prev;
      const newTasks = [...prev];
      [newTasks[idx], newTasks[newIdx]] = [newTasks[newIdx], newTasks[idx]];
      refreshStepNumbers(newTasks);
      return newTasks;
    });
  }, [refreshStepNumbers]);

  const updateTask = useCallback((updated: DesignerTask) => {
    setTasks((prev) => {
      const newTasks = prev.map((t) => (t.id === updated.id ? updated : t));
      refreshStepNumbers(newTasks);
      return newTasks;
    });
    setEditingTask(null);
  }, [refreshStepNumbers]);

  // ── When user selects a task def in SIMPLE mode, populate I/O from registry ──
  const selectTaskDef = useCallback((taskDefName: string) => {
    if (!editingTask) return;
    const td = taskDefMap.get(taskDefName);
    const inputKeys = td?.inputKeys ?? [];
    const outputKeys = td?.outputKeys ?? [];
    const newInputParams: Record<string, unknown> = {};
    for (const k of inputKeys) {
      newInputParams[k] = editingTask.inputParameters[k] ?? "";
    }
    for (const [k, v] of Object.entries(editingTask.inputParameters)) {
      if (!(k in newInputParams)) newInputParams[k] = v;
    }
    setEditingTask({
      ...editingTask,
      name: taskDefName,
      taskReferenceName: editingTask.taskReferenceName === editingTask.name ? taskDefName : editingTask.taskReferenceName,
      inputParameters: newInputParams,
      outputKeys,
    });
  }, [editingTask, taskDefMap]);

  // ── Import workflow ──
  const importWorkflow = useCallback((def: WorkflowDef) => {
    setWorkflowName(def.name);
    setWorkflowVersion(def.version);
    setWorkflowDesc(def.description || "");
    setWorkflowInputKeys(def.inputParameters || []);

    const newTasks: DesignerTask[] = [];
    const newNodes: Node[] = [];
    nextIdRef.current = 1;

    newNodes.push({
      id: "__start__",
      type: "workflowStart",
      position: { x: 250, y: 0 },
      data: { label: "Start", inputKeys: def.inputParameters || [] },
      deletable: false,
    });

    def.tasks.forEach((wt, idx) => {
      const id = `task_${nextIdRef.current++}`;
      const td = taskDefMap.get(wt.name);
      const dt: DesignerTask = {
        id,
        name: wt.name,
        taskReferenceName: wt.taskReferenceName,
        type: wt.type || "SIMPLE",
        inputParameters: (wt.inputParameters as Record<string, unknown>) || {},
        outputKeys: td?.outputKeys ?? [],
        subWorkflowParam: wt.subWorkflowParam,
        optional: wt.optional,
      };
      newTasks.push(dt);
      newNodes.push({
        id,
        type: "designerTask",
        position: { x: 250, y: 120 + idx * 140 },
        data: {
          label: wt.taskReferenceName,
          taskType: wt.type || "SIMPLE",
          refName: wt.taskReferenceName,
          stepNumber: idx + 1,
          inputKeys: Object.keys(wt.inputParameters || {}),
          outputKeys: td?.outputKeys ?? [],
          onEdit: () => {
            const fresh = tasksRef.current.find((t) => t.id === id);
            setEditingTaskRef.current(fresh ?? dt);
          },
          onDelete: () => removeTaskRef.current(id),
        },
      });
    });

    const endY = 120 + def.tasks.length * 140;
    newNodes.push({
      id: "__end__",
      type: "workflowEnd",
      position: { x: 250, y: endY },
      data: { label: "End", connectedOutputs: Object.keys(def.outputParameters || {}) },
      deletable: false,
    });

    setTasks(newTasks);
    setNodes(newNodes);
    setEdges([]);
    setEditingTask(null);
    setTimeout(() => fitView({ padding: 0.2 }), 100);
    toast.success(`Loaded "${def.name}" v${def.version}`);
  }, [fitView, taskDefMap]);

  // ── Auto-layout ──
  const autoLayout = useCallback(() => {
    setNodes((prev) => {
      const start = prev.find((n) => n.id === "__start__");
      const end = prev.find((n) => n.id === "__end__");
      const taskNodes = prev.filter((n) => n.type === "designerTask");
      const taskOrder = tasks.map((t) => t.id);
      taskNodes.sort((a, b) => taskOrder.indexOf(a.id) - taskOrder.indexOf(b.id));
      const result: Node[] = [];
      let y = 0;
      if (start) { result.push({ ...start, position: { x: 250, y } }); y += 120; }
      for (const n of taskNodes) { result.push({ ...n, position: { x: 250, y } }); y += 140; }
      if (end) { result.push({ ...end, position: { x: 250, y } }); }
      return result;
    });
    setTimeout(() => fitView({ padding: 0.2 }), 50);
  }, [fitView, tasks]);

  const clearAll = useCallback(() => {
    setTasks([]); setNodes([]); setEdges([]);
    setEditingTask(null); nextIdRef.current = 1;
  }, []);

  // ── Build ordered tasks: simple array order ──
  const orderedTasks = useMemo((): WorkflowTask[] => {
    return tasks.map((t) => {
      const wt: WorkflowTask = {
        name: t.name,
        taskReferenceName: t.taskReferenceName,
        type: t.type,
        inputParameters: t.inputParameters,
      };
      if (t.optional) wt.optional = true;
      if (t.subWorkflowParam) wt.subWorkflowParam = t.subWorkflowParam;
      if (t.type === "DYNAMIC_FORK_JOIN") {
        wt.dynamicForkJoinTasksParam = (t.inputParameters?.dynamicForkJoinTasksParam as string) || "dynamicTasks";
      }
      return wt;
    });
  }, [tasks]);

  const workflowDef: WorkflowDef = {
    name: workflowName,
    version: workflowVersion,
    description: workflowDesc,
    tasks: orderedTasks,
    inputParameters: workflowInputKeys.length > 0 ? workflowInputKeys : undefined,
    outputParameters: Object.keys(workflowOutputParams).length > 0 ? workflowOutputParams : undefined,
  };

  // ── Validation ──
  const validationErrors = useMemo(() => {
    const errors: string[] = [];
    if (!workflowName.trim()) errors.push("Workflow name is required");
    if (tasks.length === 0) errors.push("Add at least one task");
    const refs = tasks.map((t) => t.taskReferenceName);
    const dupes = refs.filter((r, i) => refs.indexOf(r) !== i);
    if (dupes.length > 0) errors.push(`Duplicate refs: ${[...new Set(dupes)].join(", ")}`);
    for (const t of tasks) {
      if (!t.taskReferenceName.trim()) errors.push(`Task "${t.name || t.id}" needs a reference name`);
    }
    return errors;
  }, [tasks, workflowName]);

  const saveMut = useMutation({
    mutationFn: () => metadataApi.registerWorkflowDef(workflowDef),
    onSuccess: () => {
      toast.success("Workflow saved");
      queryClient.invalidateQueries({ queryKey: ["workflowDefs"] });
    },
    onError: (err: Error) => toast.error(`Save failed: ${err.message}`),
  });

  // ── Drop handler ──
  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();
      setIsDragOver(false);
      const type = event.dataTransfer.getData("application/task-type");
      if (!type) return;
      addTask(type, screenToFlowPosition({ x: event.clientX, y: event.clientY }));
    },
    [addTask, screenToFlowPosition]
  );
  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
    setIsDragOver(true);
  }, []);
  const onDragLeave = useCallback(() => setIsDragOver(false), []);

  // ── Node click: auto-open editor ──
  const onNodeClick = useCallback(
    (_event: React.MouseEvent, node: Node) => {
      if (node.id === "__start__" || node.id === "__end__") {
        setEditingTask(null);
        setShowSettings(true);
        return;
      }
      const task = tasksRef.current.find((t) => t.id === node.id);
      if (task) {
        setShowSettings(false);
        setEditingTask(task);
      }
    },
    []
  );

  // ── Context menu handlers ──
  const onCanvasContextMenu = useCallback(
    (event: React.MouseEvent | MouseEvent) => {
      event.preventDefault();
      const flowPos = screenToFlowPosition({ x: event.clientX, y: event.clientY });
      const addItems: ContextMenuItem[] = TASK_TYPE_GROUPS.flatMap((group) => [
        { label: group.label, onClick: () => {}, disabled: true, separator: false },
        ...group.types.map((t) => ({
          label: t.label,
          icon: <span className="w-2.5 h-2.5 rounded-sm" style={{ backgroundColor: TASK_TYPE_COLORS[t.value] }} />,
          onClick: () => addTask(t.value, flowPos),
        })),
        { label: "", onClick: () => {}, separator: true },
      ]);
      addItems.pop();
      setCtxMenu({
        x: event.clientX,
        y: event.clientY,
        items: [
          ...addItems,
          { label: "", onClick: () => {}, separator: true },
          { label: "Auto Layout", icon: <LayoutGrid className="h-3.5 w-3.5" />, onClick: autoLayout },
          { label: "Clear All", icon: <Trash2 className="h-3.5 w-3.5" />, onClick: clearAll, danger: true },
        ],
      });
    },
    [addTask, autoLayout, clearAll, screenToFlowPosition]
  );

  const onNodeContextMenu = useCallback(
    (event: React.MouseEvent, node: Node) => {
      event.preventDefault();
      if (node.id === "__start__" || node.id === "__end__") {
        setCtxMenu({
          x: event.clientX, y: event.clientY,
          items: [
            { label: "Edit Settings", icon: <Settings2 className="h-3.5 w-3.5" />, onClick: () => setShowSettings(true) },
          ],
        });
        return;
      }
      const currentTasks = tasksRef.current;
      const task = currentTasks.find((t) => t.id === node.id);
      if (!task) return;
      const idx = currentTasks.indexOf(task);
      setCtxMenu({
        x: event.clientX, y: event.clientY,
        items: [
          { label: "Edit Task", icon: <Code className="h-3.5 w-3.5" />, onClick: () => setEditingTask(task) },
          { label: "Duplicate", icon: <Copy className="h-3.5 w-3.5" />, onClick: () => duplicateTask(task.id) },
          { label: "", onClick: () => {}, separator: true },
          { label: "Move Up (run earlier)", icon: <ChevronUp className="h-3.5 w-3.5" />, onClick: () => moveTask(task.id, "up"), disabled: idx === 0 },
          { label: "Move Down (run later)", icon: <ChevronDown className="h-3.5 w-3.5" />, onClick: () => moveTask(task.id, "down"), disabled: idx === currentTasks.length - 1 },
          { label: "", onClick: () => {}, separator: true },
          { label: "Delete Task", icon: <Trash2 className="h-3.5 w-3.5" />, onClick: () => removeTask(task.id), danger: true },
        ],
      });
    },
    [duplicateTask, moveTask, removeTask]
  );

  const onEdgeContextMenu = useCallback(
    (event: React.MouseEvent, edge: Edge) => {
      event.preventDefault();
      setCtxMenu({
        x: event.clientX, y: event.clientY,
        items: [
          {
            label: `Delete connection${edge.label ? ` (${edge.label})` : ""}`,
            icon: <Trash2 className="h-3.5 w-3.5" />,
            onClick: () => setEdges((eds) => eds.filter((e) => e.id !== edge.id)),
            danger: true,
          },
        ],
      });
    },
    []
  );

  // ── Key editors ──
  const [inputKeyDraft, setInputKeyDraft] = useState("");

  const addInputKey = () => {
    const k = inputKeyDraft.trim();
    if (k && !workflowInputKeys.includes(k)) {
      const newKeys = [...workflowInputKeys, k];
      setWorkflowInputKeys(newKeys); setInputKeyDraft("");
      setNodes((prev) => prev.map((n) => n.id === "__start__" ? { ...n, data: { ...n.data, inputKeys: newKeys } } : n));
    }
  };
  const removeInputKey = (k: string) => {
    const newKeys = workflowInputKeys.filter((x) => x !== k);
    setWorkflowInputKeys(newKeys);
    setNodes((prev) => prev.map((n) => n.id === "__start__" ? { ...n, data: { ...n.data, inputKeys: newKeys } } : n));
  };

  // ── Raw JSON text for editor ──
  const [rawJsonText, setRawJsonText] = useState("");
  useEffect(() => {
    if (editingTask) setRawJsonText(JSON.stringify(editingTask.inputParameters, null, 2));
  }, [editingTask?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Add/remove input/output params on editing task ──
  const [newInputKey, setNewInputKey] = useState("");
  const [newOutputKey, setNewOutputKey] = useState("");

  const addInputToTask = () => {
    if (!editingTask || !newInputKey.trim()) return;
    const updated = { ...editingTask, inputParameters: { ...editingTask.inputParameters, [newInputKey.trim()]: "" } };
    setEditingTask(updated);
    setRawJsonText(JSON.stringify(updated.inputParameters, null, 2));
    setNewInputKey("");
  };
  const removeInputFromTask = (key: string) => {
    if (!editingTask) return;
    const { [key]: _, ...rest } = editingTask.inputParameters;
    const updated = { ...editingTask, inputParameters: rest };
    setEditingTask(updated);
    setRawJsonText(JSON.stringify(updated.inputParameters, null, 2));
  };
  const addOutputToTask = () => {
    if (!editingTask || !newOutputKey.trim()) return;
    if (!editingTask.outputKeys.includes(newOutputKey.trim())) {
      setEditingTask({ ...editingTask, outputKeys: [...editingTask.outputKeys, newOutputKey.trim()] });
    }
    setNewOutputKey("");
  };
  const removeOutputFromTask = (key: string) => {
    if (!editingTask) return;
    setEditingTask({ ...editingTask, outputKeys: editingTask.outputKeys.filter((k) => k !== key) });
  };

  // ───────────────── RENDER ─────────────────

  return (
    <div className="flex flex-col h-[calc(100vh-4rem)]">
      {/* Context menu portal */}
      {ctxMenu && (
        <ContextMenu x={ctxMenu.x} y={ctxMenu.y} items={ctxMenu.items} onClose={() => setCtxMenu(null)} />
      )}

      {/* ── Top toolbar ── */}
      <div className="flex items-center justify-between px-4 py-2 border-b bg-background shrink-0">
        <div className="flex items-center gap-3">
          <h2 className="text-lg font-bold tracking-tight">Workflow Designer</h2>
          <Badge variant="outline" className="text-xs">{tasks.length} task{tasks.length !== 1 ? "s" : ""}</Badge>
          {validationErrors.length > 0 && (
            <Badge variant="destructive" className="text-xs gap-1">
              <AlertTriangle className="h-3 w-3" />
              {validationErrors.length}
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-2">
          <SearchableSelect
            options={workflowDefOptions}
            value=""
            onChange={(val) => {
              const [name, ver] = val.split("::");
              const def = workflowDefs?.find((d) => d.name === name && d.version === Number(ver));
              if (def) importWorkflow(def);
            }}
            placeholder="Import workflow..."
          />
          <Button variant={showSettings ? "secondary" : "outline"} size="sm" onClick={() => { setShowSettings(!showSettings); if (!showSettings) setEditingTask(null); }}>
            <Settings2 className="h-3.5 w-3.5 mr-1" />Settings
          </Button>
          <Button variant="outline" size="sm" onClick={() => setShowPreview(!showPreview)}>
            <Eye className="h-3.5 w-3.5 mr-1" />{showPreview ? "Hide" : "JSON"}
          </Button>
          <Button size="sm" onClick={() => saveMut.mutate()} disabled={saveMut.isPending || validationErrors.length > 0}>
            <Save className="h-3.5 w-3.5 mr-1" />Save
          </Button>
        </div>
      </div>

      {/* ── Main: palette + canvas + editor ── */}
      <div className="flex flex-1 min-h-0">
        {/* Left sidebar: palette + execution order */}
        <div className="w-56 border-r bg-muted/20 overflow-y-auto shrink-0 flex flex-col">
          {/* Task palette */}
          <div className="px-3 py-2.5 border-b">
            <p className="text-xs font-semibold">Task Palette</p>
            <p className="text-[10px] text-muted-foreground mt-0.5">Drag onto canvas or right-click canvas</p>
          </div>
          <div className="flex-none">
            {TASK_TYPE_GROUPS.map((group) => (
              <div key={group.label} className="px-2 py-2">
                <p className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider px-1 mb-1.5">{group.label}</p>
                {group.types.map((t) => {
                  const color = TASK_TYPE_COLORS[t.value] ?? "#9ca3af";
                  return (
                    <div
                      key={t.value}
                      draggable
                      onDragStart={(e) => { e.dataTransfer.setData("application/task-type", t.value); e.dataTransfer.effectAllowed = "move"; }}
                      className="flex items-center gap-2.5 px-2.5 py-2 rounded-lg cursor-grab active:cursor-grabbing hover:bg-muted/80 transition-all mb-0.5 border border-transparent hover:border-border/50 hover:shadow-sm"
                    >
                      <GripVertical className="h-3 w-3 text-muted-foreground/50 shrink-0" />
                      <span className="w-3 h-3 rounded-sm shrink-0" style={{ backgroundColor: color }} />
                      <div className="min-w-0">
                        <div className="text-xs font-medium leading-tight">{t.label}</div>
                        <div className="text-[9px] text-muted-foreground leading-tight">{t.desc}</div>
                      </div>
                    </div>
                  );
                })}
              </div>
            ))}
          </div>

          {/* Execution Order list */}
          {tasks.length > 0 && (
            <div className="border-t flex-1 min-h-0">
              <div className="px-3 py-2.5 border-b">
                <p className="text-xs font-semibold">Execution Order</p>
                <p className="text-[10px] text-muted-foreground mt-0.5">Tasks run top-to-bottom. Reorder with arrows.</p>
              </div>
              <div className="overflow-y-auto">
                {tasks.map((task, idx) => {
                  const color = TASK_TYPE_COLORS[task.type] ?? "#9ca3af";
                  return (
                    <div
                      key={task.id}
                      className={`flex items-center gap-1.5 px-2 py-1.5 hover:bg-muted/50 transition-colors cursor-pointer ${editingTask?.id === task.id ? "bg-primary/10" : ""}`}
                      onClick={() => { setShowSettings(false); setEditingTask(task); }}
                    >
                      <span
                        className="w-5 h-5 rounded-full flex items-center justify-center text-[9px] font-bold text-white shrink-0"
                        style={{ backgroundColor: color }}
                      >
                        {idx + 1}
                      </span>
                      <div className="flex-1 min-w-0">
                        <p className="text-[10px] font-medium truncate">{task.taskReferenceName}</p>
                        <p className="text-[8px] text-muted-foreground uppercase">{task.type}</p>
                      </div>
                      <div className="flex flex-col gap-0.5 shrink-0">
                        <button
                          onClick={(e) => { e.stopPropagation(); moveTask(task.id, "up"); }}
                          disabled={idx === 0}
                          className="p-0.5 rounded hover:bg-muted disabled:opacity-20 text-muted-foreground hover:text-foreground disabled:cursor-not-allowed"
                        >
                          <ChevronUp className="h-3 w-3" />
                        </button>
                        <button
                          onClick={(e) => { e.stopPropagation(); moveTask(task.id, "down"); }}
                          disabled={idx === tasks.length - 1}
                          className="p-0.5 rounded hover:bg-muted disabled:opacity-20 text-muted-foreground hover:text-foreground disabled:cursor-not-allowed"
                        >
                          <ChevronDown className="h-3 w-3" />
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* Quick actions */}
          <div className="px-3 py-2.5 border-t space-y-1.5 shrink-0">
            <Button variant="outline" size="sm" className="w-full justify-start text-xs h-8" onClick={autoLayout}>
              <LayoutGrid className="h-3 w-3 mr-1.5" /> Auto Layout
            </Button>
            <Button variant="outline" size="sm" className="w-full justify-start text-xs h-8 text-destructive hover:text-destructive" onClick={clearAll}>
              <Trash2 className="h-3 w-3 mr-1.5" /> Clear All
            </Button>
          </div>

          {/* Connection guide */}
          <div className="px-3 py-2.5 border-t shrink-0">
            <p className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider mb-1.5">How it works</p>
            <div className="space-y-2 text-[9px] text-muted-foreground">
              <p><b>Click</b> a node to edit it. <b>Right-click</b> for more options.</p>
              <p><b>Execution order</b> = list above. Reorder with arrows.</p>
              <div className="flex items-start gap-1.5">
                <div className="w-2 h-2 rounded-full bg-emerald-500 border border-background mt-0.5 shrink-0" />
                <span><b>Data out:</b> Drag from green ports</span>
              </div>
              <div className="flex items-start gap-1.5">
                <div className="w-2 h-2 rounded-full bg-blue-500 border border-background mt-0.5 shrink-0" />
                <span><b>Data in:</b> Drop onto blue ports</span>
              </div>
            </div>
          </div>
        </div>

        {/* Center: ReactFlow canvas */}
        <div
          className={`flex-1 min-w-0 transition-all ${isDragOver ? "ring-2 ring-inset ring-primary/40 bg-primary/5" : ""}`}
          ref={reactFlowRef}
          onDrop={onDrop}
          onDragOver={onDragOver}
          onDragLeave={onDragLeave}
        >
          {tasks.length === 0 && !hasStart ? (
            <div className="h-full flex items-center justify-center">
              <div className="text-center max-w-sm">
                <div className="w-16 h-16 rounded-2xl bg-muted/50 flex items-center justify-center mx-auto mb-4">
                  <ArrowDown className="h-8 w-8 text-muted-foreground/50" />
                </div>
                <h3 className="text-sm font-semibold text-muted-foreground mb-2">Build your workflow</h3>
                <div className="text-xs text-muted-foreground space-y-1 text-left bg-muted/30 rounded-lg p-3">
                  <p><b>Drag</b> a task from the palette, or <b>right-click</b> here to add one</p>
                  <p>Tasks execute in the order shown in the sidebar list</p>
                  <p>Connect green output ports → blue input ports to wire data</p>
                  <p>Connect task outputs to END to define workflow outputs</p>
                </div>
              </div>
            </div>
          ) : (
            <ReactFlow
              nodes={nodes}
              edges={edges}
              onNodesChange={onNodesChange}
              onEdgesChange={onEdgesChange}
              onConnect={onConnect}
              onNodeClick={onNodeClick}
              onPaneContextMenu={onCanvasContextMenu}
              onNodeContextMenu={onNodeContextMenu}
              onEdgeContextMenu={onEdgeContextMenu}
              nodeTypes={nodeTypes}
              fitView
              proOptions={{ hideAttribution: true }}
              snapToGrid
              snapGrid={[20, 20]}
              defaultEdgeOptions={DATA_EDGE_STYLE}
              connectionLineStyle={{ stroke: "#3b82f6", strokeWidth: 1.5 }}
            >
              <Background gap={20} />
              <Controls />
              <MiniMap nodeStrokeWidth={3} style={{ height: 100, width: 140 }} />
            </ReactFlow>
          )}
        </div>

        {/* Right: Task editor / JSON preview / Workflow settings (resizable) */}
        {(editingTask || showPreview || showSettings) && (
          <div className="relative flex shrink-0" style={{ width: rightPanelWidth }}>
            {/* Resize handle */}
            <div
              className="w-1.5 cursor-col-resize hover:bg-primary/20 active:bg-primary/30 transition-colors flex items-center justify-center group"
              onMouseDown={onResizeStart}
            >
              <GripHorizontal className="h-4 w-4 text-muted-foreground/30 group-hover:text-muted-foreground/60 rotate-90" />
            </div>
          <div className="flex-1 border-l bg-background overflow-y-auto">
            {/* Workflow Settings panel */}
            {showSettings && !editingTask && (
              <div className="p-4 space-y-3">
                <div className="flex items-center justify-between">
                  <h4 className="font-semibold text-sm flex items-center gap-1.5"><Settings2 className="h-4 w-4" /> Workflow Settings</h4>
                  <Button variant="ghost" size="sm" className="h-6 px-1" onClick={() => setShowSettings(false)}>×</Button>
                </div>
                <div>
                  <label className="text-xs text-muted-foreground mb-1 block">Name</label>
                  <Input value={workflowName} onChange={(e) => setWorkflowName(e.target.value)} className="h-8 text-xs" />
                </div>
                <div className="grid grid-cols-2 gap-2">
                  <div>
                    <label className="text-xs text-muted-foreground mb-1 block">Version</label>
                    <Input type="number" min={1} value={workflowVersion} onChange={(e) => setWorkflowVersion(Number(e.target.value))} className="h-8 text-xs" />
                  </div>
                  <div className="col-span-1" />
                </div>
                <div>
                  <label className="text-xs text-muted-foreground mb-1 block">Description</label>
                  <Input value={workflowDesc} onChange={(e) => setWorkflowDesc(e.target.value)} placeholder="Optional" className="h-8 text-xs" />
                </div>
                <div className="border-t pt-2">
                  <label className="text-xs font-medium mb-1.5 block">Input Parameters</label>
                  <div className="flex gap-1 mb-2">
                    <Input value={inputKeyDraft} onChange={(e) => setInputKeyDraft(e.target.value)} onKeyDown={(e) => e.key === "Enter" && addInputKey()} placeholder="Add input key..." className="h-7 text-xs flex-1" />
                    <Button variant="outline" size="sm" className="h-7 px-2" onClick={addInputKey}><Plus className="h-3 w-3" /></Button>
                  </div>
                  <div className="flex flex-wrap gap-1">
                    {workflowInputKeys.map((k) => (
                      <Badge key={k} variant="secondary" className="text-[10px] gap-0.5 h-5">{k}<button onClick={() => removeInputKey(k)} className="hover:text-destructive ml-1">×</button></Badge>
                    ))}
                    {workflowInputKeys.length === 0 && <p className="text-[10px] text-muted-foreground">No input parameters defined</p>}
                  </div>
                </div>
                <div className="border-t pt-2">
                  <label className="text-xs font-medium mb-1.5 block">Output Parameters</label>
                  <div className="bg-muted/50 rounded p-2">
                    <p className="text-[10px] text-muted-foreground">
                      {Object.keys(workflowOutputParams).length > 0
                        ? Object.keys(workflowOutputParams).join(", ")
                        : "Connect task outputs to END node to define workflow outputs"}
                    </p>
                  </div>
                </div>
                {validationErrors.length > 0 && (
                  <div className="bg-destructive/10 rounded p-2 space-y-1 border-t pt-2">
                    <p className="text-xs font-medium text-destructive mb-1">Validation Errors</p>
                    {validationErrors.map((e, i) => (
                      <p key={i} className="text-[10px] text-destructive flex items-center gap-1">
                        <AlertTriangle className="h-2.5 w-2.5 shrink-0" /> {e}
                      </p>
                    ))}
                  </div>
                )}
              </div>
            )}

            {editingTask && (
              <div className="p-4 space-y-3">
                <div className="flex items-center justify-between">
                  <h4 className="font-semibold text-sm">Edit Task</h4>
                  <Button variant="ghost" size="sm" className="h-6 px-1" onClick={() => setEditingTask(null)}>×</Button>
                </div>

                <div className="flex items-center gap-2">
                  <span
                    className="w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold text-white"
                    style={{ backgroundColor: TASK_TYPE_COLORS[editingTask.type] ?? "#9ca3af" }}
                  >
                    {tasks.findIndex((t) => t.id === editingTask.id) + 1}
                  </span>
                  <Badge variant="outline" className="text-xs" style={{ borderColor: TASK_TYPE_COLORS[editingTask.type] }}>{editingTask.type}</Badge>
                  <span className="text-[9px] text-muted-foreground ml-auto">Step {tasks.findIndex((t) => t.id === editingTask.id) + 1} of {tasks.length}</span>
                </div>

                {editingTask.type === "SIMPLE" && (
                  <div>
                    <label className="text-xs text-muted-foreground mb-1 block">Task Definition <span className="text-[9px]">(auto-populates I/O)</span></label>
                    <SearchableSelect
                      options={taskDefOptions}
                      value={editingTask.name}
                      onChange={selectTaskDef}
                      placeholder="Select registered task..."
                    />
                    {taskDefMap.has(editingTask.name) && (
                      <p className="text-[9px] text-emerald-600 mt-1">
                        Loaded {taskDefMap.get(editingTask.name)!.inputKeys?.length ?? 0} inputs, {taskDefMap.get(editingTask.name)!.outputKeys?.length ?? 0} outputs from registry
                      </p>
                    )}
                  </div>
                )}

                <div>
                  <label className="text-xs text-muted-foreground mb-1 block">Name</label>
                  <Input value={editingTask.name} onChange={(e) => setEditingTask({ ...editingTask, name: e.target.value })} className="h-8 text-xs" />
                </div>
                <div>
                  <label className="text-xs text-muted-foreground mb-1 block">Reference Name</label>
                  <Input value={editingTask.taskReferenceName} onChange={(e) => setEditingTask({ ...editingTask, taskReferenceName: e.target.value })} className="h-8 text-xs" />
                </div>
                <div>
                  <label className="text-xs text-muted-foreground mb-1 block">Type</label>
                  <Select value={editingTask.type} onValueChange={(v) => setEditingTask({ ...editingTask, type: v })}>
                    <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
                    <SelectContent>
                      {TASK_TYPES.map((t) => (<SelectItem key={t} value={t}>{t}</SelectItem>))}
                    </SelectContent>
                  </Select>
                </div>

                <div className="flex items-center gap-2">
                  <input type="checkbox" checked={editingTask.optional ?? false} onChange={(e) => setEditingTask({ ...editingTask, optional: e.target.checked })} className="h-3 w-3" />
                  <label className="text-xs text-muted-foreground">Optional (continue on failure)</label>
                </div>

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
                        <button onClick={() => removeInputFromTask(key)} className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive p-0.5">
                          <Trash2 className="h-2.5 w-2.5" />
                        </button>
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
                  <p className="text-[9px] text-muted-foreground mb-1.5">
                    Output keys define connectable green ports on the right side of the node.
                  </p>
                  <div className="space-y-1 mb-2">
                    {editingTask.outputKeys.map((key) => (
                      <div key={key} className="flex items-center gap-1 group">
                        <span className="w-2 h-2 rounded-full bg-emerald-500 shrink-0" />
                        <span className="text-[10px] font-mono font-medium text-emerald-600 dark:text-emerald-400">{key}</span>
                        <div className="flex-1" />
                        <button onClick={() => removeOutputFromTask(key)} className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive p-0.5">
                          <Trash2 className="h-2.5 w-2.5" />
                        </button>
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
                  <textarea
                    className="w-full h-20 text-xs font-mono rounded-md border bg-muted p-2 resize-y"
                    value={rawJsonText}
                    onChange={(e) => {
                      setRawJsonText(e.target.value);
                      try { setEditingTask({ ...editingTask, inputParameters: JSON.parse(e.target.value) }); } catch { /* typing */ }
                    }}
                  />
                </div>

                {/* SUB_WORKFLOW */}
                {editingTask.type === "SUB_WORKFLOW" && (
                  <div className="space-y-2 border-t pt-2">
                    <label className="text-xs font-medium text-muted-foreground">Sub-Workflow</label>
                    <SearchableSelect
                      options={workflowDefOptions.map((o) => ({ ...o, value: o.label.split(" (v")[0] }))}
                      value={editingTask.subWorkflowParam?.name ?? ""}
                      onChange={(v) => setEditingTask({ ...editingTask, subWorkflowParam: { ...editingTask.subWorkflowParam, name: v } })}
                      placeholder="Select workflow..."
                    />
                  </div>
                )}

                {/* DYNAMIC_FORK_JOIN */}
                {editingTask.type === "DYNAMIC_FORK_JOIN" && (
                  <div className="space-y-2 border-t pt-2">
                    <label className="text-xs font-medium text-muted-foreground">Dynamic Fork Config</label>
                    <p className="text-[9px] text-muted-foreground">
                      Set the input parameter containing the array of tasks to fork.
                    </p>
                    <div>
                      <label className="text-[10px] text-muted-foreground">Tasks Param Name</label>
                      <Input
                        value={(editingTask.inputParameters?.dynamicForkJoinTasksParam as string) ?? "dynamicTasks"}
                        onChange={(e) => setEditingTask({ ...editingTask, inputParameters: { ...editingTask.inputParameters, dynamicForkJoinTasksParam: e.target.value } })}
                        className="h-8 text-xs"
                        placeholder="dynamicTasks"
                      />
                    </div>
                  </div>
                )}

                <div className="flex gap-2 pt-1">
                  <Button size="sm" className="flex-1" onClick={() => updateTask(editingTask)}>
                    <Zap className="h-3 w-3 mr-1" /> Apply
                  </Button>
                  <Button variant="outline" size="sm" onClick={() => setEditingTask(null)}>Cancel</Button>
                </div>
              </div>
            )}

            {showPreview && (
              <div className={editingTask ? "p-4 border-t" : "p-4"}>
                <h4 className="font-semibold text-sm mb-2">Workflow JSON</h4>
                <JsonView data={workflowDef} maxHeight="24rem" />
              </div>
            )}
          </div>
          </div>
        )}
      </div>
    </div>
  );
}
