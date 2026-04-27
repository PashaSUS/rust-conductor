import { useState, useCallback, useRef, useMemo, useEffect } from "react";
import {
  applyNodeChanges, applyEdgeChanges,
  type Node, type Edge, type OnNodesChange, type OnEdgesChange,
} from "@xyflow/react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { metadataApi, type TaskDef, type WorkflowDef, type WorkflowTask } from "@/api/conductor";
import { toast } from "sonner";
import type { DesignerTask, ContextMenuItem } from "./types";
import { deriveTaskOrder, getBranchMeta } from "./useWorkflowActions";

export function useWorkflowState() {
  const queryClient = useQueryClient();

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
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number; items: ContextMenuItem[] } | null>(null);

  // Refs
  const reactFlowRef = useRef<HTMLDivElement>(null);
  const nextIdRef = useRef(1);
  const tasksRef = useRef(tasks);
  tasksRef.current = tasks;

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
      setRightPanelWidth(Math.max(260, Math.min(600, resizeStartW.current + (resizeStartX.current - ev.clientX))));
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

  // Queries
  const { data: registeredTaskDefs } = useQuery({ queryKey: ["task-defs"], queryFn: metadataApi.listTaskDefs });
  const { data: workflowDefs } = useQuery({ queryKey: ["workflowDefs"], queryFn: metadataApi.listWorkflowDefs });

  const taskDefMap = useMemo(() => {
    const m = new Map<string, TaskDef>();
    if (registeredTaskDefs) for (const td of registeredTaskDefs) m.set(td.name, td);
    return m;
  }, [registeredTaskDefs]);

  const taskDefOptions = useMemo(() =>
    registeredTaskDefs?.map((td) => ({ label: td.name, value: td.name })) ?? []
  , [registeredTaskDefs]);

  const workflowDefOptions = useMemo(() =>
    workflowDefs?.map((wd) => ({ label: `${wd.name} (v${wd.version})`, value: `${wd.name}::${wd.version}` })) ?? []
  , [workflowDefs]);

  // Derived: workflow output parameters from END-connected data edges
  const workflowOutputParams = useMemo(() => {
    const out: Record<string, string> = {};
    for (const e of edges) {
      if (e.target !== "__end__" || e.targetHandle !== "data-in") continue;
      const task = tasks.find((t) => t.id === e.source);
      if (!task) continue;
      const key = e.sourceHandle?.replace("out-", "") ?? "result";
      out[`${task.taskReferenceName}_${key}`] = `\${${task.taskReferenceName}.output.${key}}`;
    }
    return out;
  }, [edges, tasks]);

  const hasStart = useMemo(() => nodes.some((n) => n.id === "__start__"), [nodes]);

  /* ── Unified node data sync ──
   * Single effect that keeps ALL node data in sync with tasks, edges,
   * input keys, and editing state. Replaces all scattered update logic.
   */
  useEffect(() => {
    const ordered = deriveTaskOrder(tasks, edges);
    const stepMap = new Map(ordered.map((t, i) => [t.id, i + 1]));
    const endLabels = Object.keys(workflowOutputParams);

    setNodes((prev) => prev.map((n) => {
      if (n.id === "__start__") {
        return { ...n, data: { label: "Start", inputKeys: workflowInputKeys } };
      }
      if (n.id === "__end__") {
        return { ...n, data: { label: "End", connectedOutputs: endLabels } };
      }
      if (n.type === "designerTask") {
        // Live-preview: use editingTask data for the node being edited
        const task = (editingTask?.id === n.id ? editingTask : null) ?? tasks.find((t) => t.id === n.id);
        if (!task) return n;
        return { ...n, data: {
          label: task.taskReferenceName, taskType: task.type, refName: task.taskReferenceName,
          stepNumber: stepMap.get(task.id) ?? 0,
          inputKeys: Object.keys(task.inputParameters), outputKeys: task.outputKeys,
          branchMeta: getBranchMeta(task),
        }};
      }
      return n;
    }));
  }, [tasks, edges, workflowInputKeys, editingTask, workflowOutputParams, setNodes]);

  // Change handlers
  const onNodesChange: OnNodesChange = useCallback(
    (changes) => setNodes((nds) => applyNodeChanges(changes, nds)), []
  );
  const onEdgesChange: OnEdgesChange = useCallback(
    (changes) => setEdges((eds) => applyEdgeChanges(changes, eds)), []
  );

  // Input key management
  const [inputKeyDraft, setInputKeyDraft] = useState("");
  const addInputKey = useCallback(() => {
    const k = inputKeyDraft.trim();
    if (k && !workflowInputKeys.includes(k)) {
      setWorkflowInputKeys((prev) => [...prev, k]);
      setInputKeyDraft("");
    }
  }, [inputKeyDraft, workflowInputKeys]);
  const removeInputKey = useCallback((k: string) => {
    setWorkflowInputKeys((prev) => prev.filter((x) => x !== k));
  }, []);

  // Build workflow definition — task order derived from flow edges
  const orderedTasks = useMemo((): WorkflowTask[] => {
    return deriveTaskOrder(tasks, edges).map((t) => {
      const wt: WorkflowTask = { name: t.name, taskReferenceName: t.taskReferenceName, type: t.type, inputParameters: t.inputParameters };
      if (t.description) wt.description = t.description;
      if (t.optional) wt.optional = true;
      if (t.subWorkflowParam) wt.subWorkflowParam = t.subWorkflowParam;
      if (t.type === "DYNAMIC_FORK_JOIN") wt.dynamicForkJoinTasksParam = (t.inputParameters?.dynamicForkJoinTasksParam as string) || "dynamicTasks";
      if ((t.type === "DECISION" || t.type === "SWITCH") && t.caseExpression) {
        wt.caseExpression = t.caseExpression;
        if (t.caseValueParam) wt.caseValueParam = t.caseValueParam;
        if (t.caseNames?.length) wt.decisionCases = Object.fromEntries(t.caseNames.map((n) => [n, []]));
      }
      if (t.type === "DO_WHILE" && t.loopCondition) wt.loopCondition = t.loopCondition;
      return wt;
    });
  }, [tasks, edges]);

  const workflowDef: WorkflowDef = useMemo(() => ({
    name: workflowName, version: workflowVersion, description: workflowDesc,
    tasks: orderedTasks,
    inputParameters: workflowInputKeys.length > 0 ? workflowInputKeys : undefined,
    outputParameters: Object.keys(workflowOutputParams).length > 0 ? workflowOutputParams : undefined,
  }), [workflowName, workflowVersion, workflowDesc, orderedTasks, workflowInputKeys, workflowOutputParams]);

  // Validation
  const validationErrors = useMemo(() => {
    const errors: string[] = [];
    if (!workflowName.trim()) errors.push("Workflow name is required");
    if (tasks.length === 0) errors.push("Add at least one task");
    const refs = tasks.map((t) => t.taskReferenceName);
    const dupes = refs.filter((r, i) => refs.indexOf(r) !== i);
    if (dupes.length > 0) errors.push(`Duplicate refs: ${[...new Set(dupes)].join(", ")}`);
    for (const t of tasks) { if (!t.taskReferenceName.trim()) errors.push(`Task "${t.name || t.id}" needs a reference name`); }
    return errors;
  }, [tasks, workflowName]);

  const saveMut = useMutation({
    mutationFn: () => metadataApi.registerWorkflowDef(workflowDef),
    onSuccess: () => { toast.success("Workflow saved"); queryClient.invalidateQueries({ queryKey: ["workflowDefs"] }); },
    onError: (err: Error) => toast.error(`Save failed: ${err.message}`),
  });

  return {
    workflowName, setWorkflowName, workflowVersion, setWorkflowVersion,
    workflowDesc, setWorkflowDesc, workflowInputKeys, setWorkflowInputKeys,
    tasks, setTasks, nodes, setNodes, edges, setEdges,
    editingTask, setEditingTask,
    showPreview, setShowPreview, showSettings, setShowSettings,
    isDragOver, setIsDragOver, ctxMenu, setCtxMenu,
    reactFlowRef, nextIdRef, tasksRef,
    rightPanelWidth, onResizeStart,
    workflowDefs, taskDefMap, taskDefOptions, workflowDefOptions,
    workflowOutputParams, hasStart,
    onNodesChange, onEdgesChange,
    inputKeyDraft, setInputKeyDraft, addInputKey, removeInputKey,
    orderedTasks, workflowDef, validationErrors, saveMut,
  };
}

export type WorkflowState = ReturnType<typeof useWorkflowState>;
