import { useCallback } from "react";
import { addEdge, useReactFlow, type Node, type Edge, type OnConnect, type Connection } from "@xyflow/react";
import type { WorkflowDef } from "@/api/conductor";
import { toast } from "sonner";
import type { DesignerTask } from "./types";
import { DATA_EDGE_STYLE, FLOW_EDGE_STYLE, TASK_TYPE_DEFAULT_INPUTS } from "./constants";
import type { WorkflowState } from "./useWorkflowState";

/** Extract branching metadata for node display */
export function getBranchMeta(task: DesignerTask) {
  if (task.type === "DECISION" || task.type === "SWITCH") {
    return { caseCount: task.caseNames?.length ?? 0, caseNames: task.caseNames ?? [], expression: task.caseExpression };
  }
  if (task.type === "DO_WHILE") return { condition: task.loopCondition };
  if (task.type === "FORK_JOIN" || task.type === "DYNAMIC_FORK_JOIN") return {};
  return undefined;
}

/** BFS task ordering following flow edges from __start__. Disconnected tasks appended at end. */
export function deriveTaskOrder(tasks: DesignerTask[], edges: Edge[]): DesignerTask[] {
  const flowEdges = edges.filter((e) =>
    (e.sourceHandle === "flow-out" || e.sourceHandle?.startsWith("case-")) && e.targetHandle === "flow-in"
  );
  const adj = new Map<string, string[]>();
  for (const e of flowEdges) {
    const list = adj.get(e.source) || [];
    list.push(e.target);
    adj.set(e.source, list);
  }
  const visited = new Set<string>();
  const ordered: string[] = [];
  const queue = ["__start__"];
  while (queue.length > 0) {
    const current = queue.shift()!;
    for (const next of adj.get(current) || []) {
      if (next === "__end__" || visited.has(next)) continue;
      visited.add(next);
      ordered.push(next);
      queue.push(next);
    }
  }
  const taskMap = new Map(tasks.map((t) => [t.id, t]));
  const result = ordered.filter((id) => taskMap.has(id)).map((id) => taskMap.get(id)!);
  for (const t of tasks) {
    if (!visited.has(t.id)) result.push(t);
  }
  return result;
}

export function useWorkflowActions(state: WorkflowState) {
  const { fitView } = useReactFlow();
  const {
    setTasks, setNodes, setEdges, setEditingTask,
    nodes, tasks, taskDefMap, tasksRef, nextIdRef,
    setWorkflowName, setWorkflowVersion, setWorkflowDesc, setWorkflowInputKeys,
  } = state;

  /* ── Connection handler (flow + data) ── */
  const onConnect: OnConnect = useCallback((conn: Connection) => {
    const src = conn.sourceHandle ?? "";
    const tgt = conn.targetHandle ?? "";
    const isFlowSrc = src === "flow-out" || src.startsWith("case-");
    const isFlowTgt = tgt === "flow-in";

    if (isFlowSrc && isFlowTgt) {
      const label = src.startsWith("case-") ? src.replace("case-", "") : undefined;
      setEdges((eds) => addEdge({
        ...conn, ...FLOW_EDGE_STYLE,
        ...(label ? {
          label,
          labelStyle: { fontSize: 9, fill: "#6b7280", fontWeight: 600 },
          labelBgStyle: { fill: "var(--color-card, #fff)", fillOpacity: 0.9 },
          labelBgPadding: [4, 2] as [number, number],
        } : {}),
      }, eds));
      return;
    }

    const isOut = src.startsWith("out-");
    const isIn = tgt.startsWith("in-");
    const isEndIn = tgt === "data-in";
    if (!isOut || !(isIn || isEndIn)) return;

    const outKey = src.replace("out-", "");
    if (isIn) {
      const inKey = tgt.replace("in-", "");
      setTasks((prev) => {
        if (conn.source === "__start__") {
          return prev.map((t) => t.id === conn.target
            ? { ...t, inputParameters: { ...t.inputParameters, [inKey]: `\${workflow.input.${outKey}}` } } : t);
        }
        const srcTask = prev.find((t) => t.id === conn.source);
        if (!srcTask) return prev;
        return prev.map((t) => t.id === conn.target
          ? { ...t, inputParameters: { ...t.inputParameters, [inKey]: `\${${srcTask.taskReferenceName}.output.${outKey}}` } } : t);
      });
    }

    const edgeLabel = isIn ? `${outKey} → ${tgt.replace("in-", "")}` : outKey;
    setEdges((eds) => addEdge({
      ...conn, ...DATA_EDGE_STYLE,
      label: edgeLabel,
      labelStyle: { fontSize: 8, fill: "#3b82f6" },
      labelBgStyle: { fill: "var(--color-card, #fff)", fillOpacity: 0.9 },
      labelBgPadding: [4, 2] as [number, number],
    }, eds));
  }, [setTasks, setEdges]);

  /* ── Add task (+ create Start/End if first task) ── */
  const addTask = useCallback((type: string, position?: { x: number; y: number }) => {
    const id = `task_${nextIdRef.current++}`;
    const refName = `${type.toLowerCase()}_${id}`;
    const defaultInputs = TASK_TYPE_DEFAULT_INPUTS[type] ?? {};
    const task: DesignerTask = {
      id, name: refName, taskReferenceName: refName, type,
      description: "", inputParameters: { ...defaultInputs }, outputKeys: [],
    };
    setTasks((prev) => [...prev, task]);

    setNodes((prev) => {
      const result = [...prev];
      if (!result.some((n) => n.id === "__start__")) {
        result.push({ id: "__start__", type: "workflowStart", position: { x: 250, y: 0 },
          data: { label: "Start", inputKeys: [] as string[] }, deletable: false });
      }
      const existingTasks = result.filter((n) => n.type === "designerTask");
      const pos = position ?? { x: 250, y: 160 + existingTasks.length * 160 };
      result.push({ id, type: "designerTask", position: pos, data: {
        label: refName, taskType: type, refName, stepNumber: 0,
        inputKeys: Object.keys(defaultInputs), outputKeys: [] as string[],
        branchMeta: getBranchMeta(task),
      }});
      if (!result.some((n) => n.id === "__end__")) {
        result.push({ id: "__end__", type: "workflowEnd", position: { x: 250, y: pos.y + 200 },
          data: { label: "End", connectedOutputs: [] as string[] }, deletable: false });
      }
      return result;
    });
  }, [setTasks, setNodes, nextIdRef]);

  /* ── Remove task ── */
  const removeTask = useCallback((id: string) => {
    setTasks((prev) => prev.filter((t) => t.id !== id));
    setNodes((prev) => prev.filter((n) => n.id !== id));
    setEdges((prev) => prev.filter((e) => e.source !== id && e.target !== id));
    setEditingTask((cur) => cur?.id === id ? null : cur);
  }, [setTasks, setNodes, setEdges, setEditingTask]);

  /* ── Duplicate task ── */
  const duplicateTask = useCallback((id: string) => {
    const task = tasksRef.current.find((t) => t.id === id);
    if (!task) return;
    const newId = `task_${nextIdRef.current++}`;
    const dup: DesignerTask = { ...task, id: newId, taskReferenceName: `${task.taskReferenceName}_copy` };
    setTasks((prev) => { const idx = prev.findIndex((t) => t.id === id); const next = [...prev]; next.splice(idx + 1, 0, dup); return next; });
    const srcNode = nodes.find((n) => n.id === id);
    const pos = srcNode ? { x: srcNode.position.x + 40, y: srcNode.position.y + 40 } : { x: 300, y: 200 };
    setNodes((prev) => [...prev, { id: newId, type: "designerTask", position: pos, data: {
      label: dup.taskReferenceName, taskType: dup.type, refName: dup.taskReferenceName, stepNumber: 0,
      inputKeys: Object.keys(dup.inputParameters), outputKeys: dup.outputKeys, branchMeta: getBranchMeta(dup),
    }}]);
    toast.success("Task duplicated");
  }, [nodes, tasksRef, nextIdRef, setTasks, setNodes]);

  /* ── Move task (sidebar reorder) ── */
  const moveTask = useCallback((id: string, direction: "up" | "down") => {
    setTasks((prev) => {
      const idx = prev.findIndex((t) => t.id === id);
      if (idx < 0) return prev;
      const newIdx = direction === "up" ? idx - 1 : idx + 1;
      if (newIdx < 0 || newIdx >= prev.length) return prev;
      const next = [...prev];
      [next[idx], next[newIdx]] = [next[newIdx], next[idx]];
      return next;
    });
  }, [setTasks]);

  /* ── Update task (apply from editor) ── */
  const updateTask = useCallback((updated: DesignerTask) => {
    setTasks((prev) => prev.map((t) => t.id === updated.id ? updated : t));
    setEditingTask(null);
  }, [setTasks, setEditingTask]);

  /* ── Import workflow ── */
  const importWorkflow = useCallback((def: WorkflowDef) => {
    setWorkflowName(def.name);
    setWorkflowVersion(def.version);
    setWorkflowDesc(def.description || "");
    setWorkflowInputKeys(def.inputParameters || []);
    nextIdRef.current = 1;

    const newTasks: DesignerTask[] = [];
    const newNodes: Node[] = [];
    const newEdges: Edge[] = [];

    newNodes.push({ id: "__start__", type: "workflowStart", position: { x: 250, y: 0 },
      data: { label: "Start", inputKeys: def.inputParameters || [] }, deletable: false });

    def.tasks.forEach((wt, idx) => {
      const id = `task_${nextIdRef.current++}`;
      const td = taskDefMap.get(wt.name);
      const dt: DesignerTask = {
        id, name: wt.name, taskReferenceName: wt.taskReferenceName,
        type: wt.type || "SIMPLE", description: wt.description || "",
        inputParameters: (wt.inputParameters as Record<string, unknown>) || {},
        inputParameterDefinitions: wt.inputParameterDefinitions,
        outputKeys: td?.outputKeys ?? [], subWorkflowParam: wt.subWorkflowParam,
        optional: wt.optional, caseExpression: wt.caseExpression,
        caseValueParam: wt.caseValueParam,
        caseNames: wt.decisionCases ? Object.keys(wt.decisionCases) : undefined,
        loopCondition: wt.loopCondition,
      };
      newTasks.push(dt);
      newNodes.push({ id, type: "designerTask", position: { x: 250, y: 120 + idx * 160 }, data: {
        label: dt.taskReferenceName, taskType: dt.type, refName: dt.taskReferenceName,
        stepNumber: idx + 1, inputKeys: Object.keys(dt.inputParameters),
        outputKeys: dt.outputKeys, branchMeta: getBranchMeta(dt),
      }});
    });

    newNodes.push({ id: "__end__", type: "workflowEnd",
      position: { x: 250, y: 120 + def.tasks.length * 160 },
      data: { label: "End", connectedOutputs: Object.keys(def.outputParameters || {}) }, deletable: false });

    // Auto-generate linear flow chain for imported workflows
    const chain = ["__start__", ...newTasks.map((t) => t.id), "__end__"];
    for (let i = 0; i < chain.length - 1; i++) {
      newEdges.push({ id: `flow-${chain[i]}-${chain[i + 1]}`,
        source: chain[i], target: chain[i + 1],
        sourceHandle: "flow-out", targetHandle: "flow-in", ...FLOW_EDGE_STYLE });
    }

    setTasks(newTasks); setNodes(newNodes); setEdges(newEdges); setEditingTask(null);
    setTimeout(() => fitView({ padding: 0.2 }), 100);
    toast.success(`Loaded "${def.name}" v${def.version}`);
  }, [fitView, taskDefMap, nextIdRef, setTasks, setNodes, setEdges, setEditingTask,
      setWorkflowName, setWorkflowVersion, setWorkflowDesc, setWorkflowInputKeys]);

  /* ── Auto layout ── */
  const autoLayout = useCallback(() => {
    setNodes((prev) => {
      const start = prev.find((n) => n.id === "__start__");
      const end = prev.find((n) => n.id === "__end__");
      const taskNodes = prev.filter((n) => n.type === "designerTask");
      const taskIds = tasks.map((t) => t.id);
      taskNodes.sort((a, b) => taskIds.indexOf(a.id) - taskIds.indexOf(b.id));
      const result: Node[] = [];
      let y = 0;
      if (start) { result.push({ ...start, position: { x: 250, y } }); y += 120; }
      for (const n of taskNodes) { result.push({ ...n, position: { x: 250, y } }); y += 160; }
      if (end) { result.push({ ...end, position: { x: 250, y } }); }
      return result;
    });
    setTimeout(() => fitView({ padding: 0.2 }), 50);
  }, [fitView, tasks, setNodes]);

  /* ── Clear all ── */
  const clearAll = useCallback(() => {
    setTasks([]); setNodes([]); setEdges([]); setEditingTask(null);
    nextIdRef.current = 1;
  }, [setTasks, setNodes, setEdges, setEditingTask, nextIdRef]);

  return { onConnect, addTask, removeTask, duplicateTask, moveTask, updateTask, importWorkflow, autoLayout, clearAll };
}
