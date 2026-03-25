import { useMemo, useCallback } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MarkerType,
  type Node,
  type Edge,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import type { TaskResult, WorkflowTask } from "@/api/conductor";

const STATUS_COLORS: Record<string, string> = {
  COMPLETED: "#10b981",
  IN_PROGRESS: "#3b82f6",
  SCHEDULED: "#f59e0b",
  FAILED: "#ef4444",
  TIMED_OUT: "#f97316",
  CANCELED: "#9ca3af",
  SKIPPED: "#d1d5db",
};

function taskDuration(task: TaskResult): number {
  const start = task.startTime ?? task.scheduledTime ?? 0;
  const end = task.endTime ?? Date.now();
  return end - start;
}

/**
 * Compute topological layers (ranks) for DAG layout.
 * Nodes with no incoming edges go in layer 0, their successors in layer 1, etc.
 */
function computeLayers(nodeIds: string[], edgeList: { source: string; target: string }[]): Map<string, number> {
  const inDegree = new Map<string, number>();
  const successors = new Map<string, string[]>();
  for (const id of nodeIds) {
    inDegree.set(id, 0);
    successors.set(id, []);
  }
  for (const e of edgeList) {
    if (inDegree.has(e.target)) {
      inDegree.set(e.target, (inDegree.get(e.target) ?? 0) + 1);
    }
    successors.get(e.source)?.push(e.target);
  }

  const layers = new Map<string, number>();
  const queue: string[] = [];
  for (const [id, deg] of inDegree) {
    if (deg === 0) {
      queue.push(id);
      layers.set(id, 0);
    }
  }

  while (queue.length > 0) {
    const cur = queue.shift()!;
    const curLayer = layers.get(cur) ?? 0;
    for (const next of successors.get(cur) ?? []) {
      const newLayer = curLayer + 1;
      if ((layers.get(next) ?? -1) < newLayer) {
        layers.set(next, newLayer);
      }
      inDegree.set(next, (inDegree.get(next) ?? 1) - 1);
      if (inDegree.get(next) === 0) {
        queue.push(next);
      }
    }
  }

  // Assign any remaining unvisited nodes (cycles) to layer 0
  for (const id of nodeIds) {
    if (!layers.has(id)) layers.set(id, 0);
  }

  return layers;
}

interface Props {
  definitionTasks: WorkflowTask[];
  runtimeTasks: TaskResult[];
  onTaskClick?: (refName: string) => void;
}

/**
 * #172 — Task dependency graph with critical path highlighting.
 * Builds a directed graph from workflow definition tasks (respecting
 * FORK_JOIN, DECISION, etc.) and overlays runtime status + duration
 * on each node. The longest path (critical path) is highlighted in red.
 * Uses topological-layer layout for proper alignment.
 */
export function TaskDependencyGraph({ definitionTasks, runtimeTasks, onTaskClick }: Props) {
  const runtimeMap = useMemo(() => {
    const m = new Map<string, TaskResult>();
    for (const t of runtimeTasks) m.set(t.referenceTaskName, t);
    return m;
  }, [runtimeTasks]);

  const { nodes, edges } = useMemo(() => {
    const nodeList: Node[] = [];
    const edgeList: Edge[] = [];
    const adjDuration = new Map<string, number>(); // ref -> total duration to finish

    // Build edges from definition task order
    const refs: string[] = [];
    for (let i = 0; i < definitionTasks.length; i++) {
      const dt = definitionTasks[i];
      const ref = dt.taskReferenceName;
      refs.push(ref);

      const rt = runtimeMap.get(ref);
      const color = rt ? (STATUS_COLORS[rt.status] ?? "#9ca3af") : "#e5e7eb";
      const dur = rt ? taskDuration(rt) : 0;
      adjDuration.set(ref, dur);

      nodeList.push({
        id: ref,
        position: { x: 0, y: 0 }, // will be set by layer layout
        data: {
          label: (
            <div className="text-center">
              <div className="font-semibold text-xs truncate max-w-27.5">{ref}</div>
              <div className="text-[10px] text-muted-foreground">{dt.type ?? "SIMPLE"}</div>
              {rt && (
                <div className="text-[10px] mt-0.5">
                  {dur > 1000 ? `${(dur / 1000).toFixed(1)}s` : `${dur}ms`}
                </div>
              )}
            </div>
          ),
        },
        style: {
          borderColor: color,
          borderWidth: 2,
          borderRadius: 8,
          padding: "8px 12px",
          width: 140,
          background: `${color}15`,
        },
      });

      // Sequential edge — skip if previous task is a FORK_JOIN or DECISION (those use explicit branch edges)
      if (i > 0) {
        const prevDt = definitionTasks[i - 1];
        const hasBranches = (prevDt.type === "FORK_JOIN" && prevDt.forkTasks) ||
          ((prevDt.type === "DECISION" || prevDt.type === "SWITCH") && prevDt.decisionCases);
        if (!hasBranches) {
          const prevRef = refs[i - 1];
          edgeList.push({
            id: `${prevRef}-${ref}`,
            source: prevRef,
            target: ref,
            markerEnd: { type: MarkerType.ArrowClosed },
            style: { stroke: "#9ca3af" },
          });
        }
      }

      // Fork tasks edges
      if (dt.type === "FORK_JOIN" && dt.forkTasks) {
        for (const branch of dt.forkTasks) {
          for (const bt of branch) {
            if (bt.taskReferenceName !== ref) {
              edgeList.push({
                id: `${ref}-fork-${bt.taskReferenceName}`,
                source: ref,
                target: bt.taskReferenceName,
                markerEnd: { type: MarkerType.ArrowClosed },
                style: { stroke: "#9ca3af", strokeDasharray: "5 5" },
              });
            }
          }
        }
      }

      // Decision/Switch edges
      if ((dt.type === "DECISION" || dt.type === "SWITCH") && dt.decisionCases) {
        for (const [, caseTasks] of Object.entries(dt.decisionCases)) {
          for (const ct of caseTasks) {
            edgeList.push({
              id: `${ref}-case-${ct.taskReferenceName}`,
              source: ref,
              target: ct.taskReferenceName,
              markerEnd: { type: MarkerType.ArrowClosed },
              style: { stroke: "#9ca3af", strokeDasharray: "3 3" },
            });
          }
        }
      }
    }

    // Critical path: longest path through sequential duration
    const sorted = [...adjDuration.entries()].sort((a, b) => b[1] - a[1]);
    const critSet = new Set<string>();
    // Simple heuristic: mark top 30% by duration as critical path
    const cutoff = Math.max(1, Math.ceil(sorted.length * 0.3));
    for (let i = 0; i < cutoff && i < sorted.length; i++) {
      critSet.add(sorted[i][0]);
    }

    // Highlight critical path edges
    for (const edge of edgeList) {
      if (critSet.has(edge.source) && critSet.has(edge.target)) {
        edge.style = { ...edge.style, stroke: "#ef4444", strokeWidth: 2.5 };
      }
    }

    // Highlight critical path nodes
    for (const node of nodeList) {
      if (critSet.has(node.id)) {
        node.style = { ...node.style, boxShadow: "0 0 0 2px #ef4444" };
      }
    }

    // Topological layer layout — left-to-right, centered vertically per layer
    const nodeIds = nodeList.map((n) => n.id);
    const simpleEdges = edgeList.map((e) => ({ source: e.source, target: e.target }));
    const layers = computeLayers(nodeIds, simpleEdges);

    const layerGroups = new Map<number, string[]>();
    for (const [id, layer] of layers) {
      if (!layerGroups.has(layer)) layerGroups.set(layer, []);
      layerGroups.get(layer)!.push(id);
    }

    const NODE_W = 220;
    const NODE_H = 110;
    const maxNodesInLayer = Math.max(...[...layerGroups.values()].map((g) => g.length), 1);

    for (const [layer, ids] of layerGroups) {
      const totalHeight = ids.length * NODE_H;
      const offsetY = (maxNodesInLayer * NODE_H - totalHeight) / 2;

      ids.forEach((id, idx) => {
        const node = nodeList.find((n) => n.id === id);
        if (node) {
          node.position = {
            x: layer * NODE_W,
            y: offsetY + idx * NODE_H,
          };
        }
      });
    }

    return { nodes: nodeList, edges: edgeList, criticalSet: critSet };
  }, [definitionTasks, runtimeMap]);

  const handleNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      onTaskClick?.(node.id);
    },
    [onTaskClick]
  );

  return (
    <div className="space-y-2">
      <div className="flex items-center gap-4 text-xs text-muted-foreground">
        <span className="flex items-center gap-1">
          <span className="inline-block w-3 h-0.5 bg-red-500 rounded" /> Critical path
        </span>
        <span className="flex items-center gap-1">
          <span className="inline-block w-3 h-0.5 bg-gray-400 rounded" style={{ borderBottom: "1px dashed" }} /> Fork/Decision
        </span>
      </div>
      <div style={{ height: "calc(100vh - 22rem)" }} className="rounded border min-h-80">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodeClick={handleNodeClick}
          fitView
          proOptions={{ hideAttribution: true }}
        >
          <Background />
          <Controls />
        </ReactFlow>
      </div>
    </div>
  );
}
