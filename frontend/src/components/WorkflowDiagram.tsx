import { useCallback, useMemo } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  type Node,
  type Edge,
  Position,
  MarkerType,
  Handle,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import type { WorkflowTask, TaskResult } from "@/api/conductor";

// ── Status colors ──

const STATUS_COLORS: Record<string, { bg: string; border: string; text: string }> = {
  COMPLETED:   { bg: "#ecfdf5", border: "#10b981", text: "#065f46" },
  IN_PROGRESS: { bg: "#eff6ff", border: "#3b82f6", text: "#1e40af" },
  SCHEDULED:   { bg: "#fffbeb", border: "#f59e0b", text: "#92400e" },
  FAILED:      { bg: "#fef2f2", border: "#ef4444", text: "#991b1b" },
  FAILED_WITH_TERMINAL_ERROR: { bg: "#fef2f2", border: "#dc2626", text: "#7f1d1d" },
  TIMED_OUT:   { bg: "#fef2f2", border: "#f97316", text: "#9a3412" },
  CANCELED:    { bg: "#f5f5f5", border: "#737373", text: "#404040" },
  SKIPPED:     { bg: "#f5f5f5", border: "#a3a3a3", text: "#525252" },
};

const DEFAULT_COLOR = { bg: "#f9fafb", border: "#d1d5db", text: "#374151" };

// ── Task type icons ──

function taskTypeLabel(type?: string): string {
  switch (type?.toUpperCase()) {
    case "FORK_JOIN": return "⑂ Fork";
    case "JOIN":      return "⊕ Join";
    case "DECISION":
    case "SWITCH":    return "◇ Switch";
    case "SUB_WORKFLOW": return "↳ SubWF";
    case "DO_WHILE":  return "↻ Loop";
    case "EVENT":     return "⚡ Event";
    case "HTTP":      return "🌐 HTTP";
    case "WAIT":      return "⏳ Wait";
    case "TERMINATE": return "⏹ End";
    default:          return "▪ Task";
  }
}

// ── Custom node ──

function TaskNode({ data }: { data: { label: string; taskType: string; status?: string; refName: string } }) {
  const colors = data.status ? (STATUS_COLORS[data.status] ?? DEFAULT_COLOR) : DEFAULT_COLOR;

  return (
    <div
      style={{
        background: colors.bg,
        border: `2px solid ${colors.border}`,
        borderRadius: 8,
        padding: "8px 14px",
        minWidth: 160,
        fontFamily: "system-ui, sans-serif",
        boxShadow: "0 1px 3px rgba(0,0,0,0.1)",
        cursor: data.refName ? "pointer" : "default",
      }}
    >
      <Handle type="target" position={Position.Top} style={{ background: colors.border }} />
      <div style={{ fontSize: 10, color: "#6b7280", marginBottom: 2 }}>
        {taskTypeLabel(data.taskType)}
      </div>
      <div style={{ fontSize: 13, fontWeight: 600, color: colors.text }}>
        {data.label}
      </div>
      <div style={{ fontSize: 10, color: "#9ca3af", marginTop: 1 }}>{data.refName}</div>
      {data.status && (
        <div
          style={{
            fontSize: 9,
            fontWeight: 700,
            color: colors.text,
            background: colors.border + "22",
            borderRadius: 4,
            padding: "1px 6px",
            marginTop: 4,
            display: "inline-block",
          }}
        >
          {data.status}
        </div>
      )}
      <Handle type="source" position={Position.Bottom} style={{ background: colors.border }} />
    </div>
  );
}

const nodeTypes = { task: TaskNode };

// ── Graph layout constants ──

const X_SPACING = 220;
const Y_SPACING = 100;

// ── Convert workflow tasks → nodes & edges ──

interface LayoutResult {
  nodes: Node[];
  edges: Edge[];
}

function buildGraph(
  definitionTasks: WorkflowTask[],
  runtimeTasks: TaskResult[],
): LayoutResult {
  const nodes: Node[] = [];
  const edges: Edge[] = [];

  // Map refName → runtime status
  const statusMap = new Map<string, string>();
  for (const rt of runtimeTasks) {
    statusMap.set(rt.referenceTaskName, rt.status);
  }

  let nodeIdCounter = 0;
  const nextId = () => `n${nodeIdCounter++}`;

  // Start node — x will be centred after layout
  const startId = nextId();
  nodes.push({
    id: startId,
    type: "task",
    position: { x: 0, y: 0 },
    data: { label: "Start", taskType: "START", refName: "", status: undefined },
  });

  // Recursively lay out tasks, returning the IDs of terminal nodes
  function layoutTasks(
    tasks: WorkflowTask[],
    parentIds: string[],
    startX: number,
    startY: number,
  ): { terminalIds: string[]; maxX: number; maxY: number } {
    let currentIds = parentIds;
    let y = startY;
    let maxX = startX;

    for (const task of tasks) {
      const type = (task.type ?? "SIMPLE").toUpperCase();

      if (type === "FORK_JOIN" && task.forkTasks?.length) {
        // Fork node
        const forkId = nextId();
        nodes.push({
          id: forkId,
          type: "task",
          position: { x: startX, y },
          data: {
            label: task.name,
            taskType: type,
            refName: task.taskReferenceName,
            status: statusMap.get(task.taskReferenceName),
          },
        });
        for (const pid of currentIds) {
          edges.push({
            id: `e-${pid}-${forkId}`,
            source: pid,
            target: forkId,
            markerEnd: { type: MarkerType.ArrowClosed },
            style: { stroke: "#94a3b8" },
          });
        }

        // Layout each fork branch side by side
        const branchTerminals: string[] = [];
        let branchX = startX - ((task.forkTasks.length - 1) * X_SPACING) / 2;
        let branchMaxY = y + Y_SPACING;

        for (const branch of task.forkTasks) {
          const result = layoutTasks(branch, [forkId], branchX, y + Y_SPACING);
          branchTerminals.push(...result.terminalIds);
          branchMaxY = Math.max(branchMaxY, result.maxY);
          maxX = Math.max(maxX, result.maxX);
          branchX += X_SPACING;
        }

        currentIds = branchTerminals;
        y = branchMaxY + Y_SPACING;
      } else if ((type === "DECISION" || type === "SWITCH") && task.decisionCases) {
        // Decision node
        const decId = nextId();
        nodes.push({
          id: decId,
          type: "task",
          position: { x: startX, y },
          data: {
            label: task.name,
            taskType: type,
            refName: task.taskReferenceName,
            status: statusMap.get(task.taskReferenceName),
          },
        });
        for (const pid of currentIds) {
          edges.push({
            id: `e-${pid}-${decId}`,
            source: pid,
            target: decId,
            markerEnd: { type: MarkerType.ArrowClosed },
            style: { stroke: "#94a3b8" },
          });
        }

        const cases = Object.entries(task.decisionCases);
        if (task.defaultCase?.length) {
          cases.push(["default", task.defaultCase]);
        }

        const branchTerminals: string[] = [];
        let branchX = startX - ((cases.length - 1) * X_SPACING) / 2;
        let branchMaxY = y + Y_SPACING;

        for (const [, caseTasks] of cases) {
          const result = layoutTasks(caseTasks, [decId], branchX, y + Y_SPACING);
          branchTerminals.push(...result.terminalIds);
          branchMaxY = Math.max(branchMaxY, result.maxY);
          maxX = Math.max(maxX, result.maxX);
          branchX += X_SPACING;
        }

        currentIds = branchTerminals;
        y = branchMaxY + Y_SPACING;
      } else if (type === "DO_WHILE" && task.loopOver?.length) {
        // Loop node
        const loopId = nextId();
        nodes.push({
          id: loopId,
          type: "task",
          position: { x: startX, y },
          data: {
            label: task.name,
            taskType: type,
            refName: task.taskReferenceName,
            status: statusMap.get(task.taskReferenceName),
          },
        });
        for (const pid of currentIds) {
          edges.push({
            id: `e-${pid}-${loopId}`,
            source: pid,
            target: loopId,
            markerEnd: { type: MarkerType.ArrowClosed },
            style: { stroke: "#94a3b8" },
          });
        }

        const inner = layoutTasks(task.loopOver, [loopId], startX, y + Y_SPACING);
        // Loop-back edge
        for (const tid of inner.terminalIds) {
          edges.push({
            id: `e-loop-${tid}-${loopId}`,
            source: tid,
            target: loopId,
            markerEnd: { type: MarkerType.ArrowClosed },
            style: { stroke: "#a78bfa", strokeDasharray: "5,5" },
            animated: true,
          });
        }

        currentIds = [loopId];
        y = inner.maxY + Y_SPACING;
        maxX = Math.max(maxX, inner.maxX);
      } else {
        // Simple / other task types — linear
        const tid = nextId();
        nodes.push({
          id: tid,
          type: "task",
          position: { x: startX, y },
          data: {
            label: task.name,
            taskType: type,
            refName: task.taskReferenceName,
            status: statusMap.get(task.taskReferenceName),
          },
        });
        for (const pid of currentIds) {
          edges.push({
            id: `e-${pid}-${tid}`,
            source: pid,
            target: tid,
            markerEnd: { type: MarkerType.ArrowClosed },
            style: { stroke: "#94a3b8" },
          });
        }
        currentIds = [tid];
        y += Y_SPACING;
      }
    }

    return { terminalIds: currentIds, maxX: Math.max(maxX, startX), maxY: y };
  }

  const { terminalIds, maxY } = layoutTasks(definitionTasks, [startId], 0, Y_SPACING);

  // End node — x will be centred after layout
  const endId = nextId();
  nodes.push({
    id: endId,
    type: "task",
    position: { x: 0, y: maxY },
    data: { label: "End", taskType: "END", refName: "", status: undefined },
  });

  // ── Centre Start & End on the midpoint of all other nodes ──
  const contentNodes = nodes.filter((n) => n.id !== startId && n.id !== endId);
  if (contentNodes.length > 0) {
    const minX = Math.min(...contentNodes.map((n) => n.position.x));
    const maxX = Math.max(...contentNodes.map((n) => n.position.x));
    const centerX = (minX + maxX) / 2;
    const startNode = nodes.find((n) => n.id === startId)!;
    const endNode = nodes.find((n) => n.id === endId)!;
    startNode.position.x = centerX;
    endNode.position.x = centerX;
  }
  for (const tid of terminalIds) {
    edges.push({
      id: `e-${tid}-${endId}`,
      source: tid,
      target: endId,
      markerEnd: { type: MarkerType.ArrowClosed },
      style: { stroke: "#94a3b8" },
    });
  }

  return { nodes, edges };
}

// ── Main Component ──

interface WorkflowDiagramProps {
  definitionTasks: WorkflowTask[];
  runtimeTasks: TaskResult[];
  onTaskClick?: (refName: string) => void;
}

export default function WorkflowDiagram({ definitionTasks, runtimeTasks, onTaskClick }: WorkflowDiagramProps) {
  const { nodes, edges } = useMemo(
    () => buildGraph(definitionTasks, runtimeTasks),
    [definitionTasks, runtimeTasks],
  );

  const onInit = useCallback((instance: { fitView: () => void }) => {
    setTimeout(() => instance.fitView(), 50);
  }, []);

  const handleNodeClick = useCallback((_event: React.MouseEvent, node: Node) => {
    const refName = node.data?.refName as string;
    if (refName && onTaskClick) onTaskClick(refName);
  }, [onTaskClick]);

  return (
    <div style={{ width: "100%", height: 600, borderRadius: 8, overflow: "hidden", border: "1px solid #e5e7eb" }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onInit={onInit}
        onNodeClick={handleNodeClick}
        fitView
        minZoom={0.2}
        maxZoom={2}
        proOptions={{ hideAttribution: true }}
      >
        <Background gap={16} size={1} color="#f1f5f9" />
        <Controls position="bottom-right" />
        <MiniMap
          nodeStrokeWidth={3}
          pannable
          zoomable
          style={{ border: "1px solid #e5e7eb", borderRadius: 4 }}
        />
      </ReactFlow>
    </div>
  );
}
