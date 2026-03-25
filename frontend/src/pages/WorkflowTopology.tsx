import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { workflowApi, taskApi } from "@/api/conductor";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  type Node,
  type Edge,
  MarkerType,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

const STATUS_COLORS: Record<string, string> = {
  RUNNING: "#3b82f6",
  COMPLETED: "#10b981",
  FAILED: "#ef4444",
  TIMED_OUT: "#f97316",
  PAUSED: "#f59e0b",
  TERMINATED: "#9ca3af",
};

/**
 * #174 — Real-time topology map.
 * Shows a live graph of all running workflows and their current queue depths.
 * Nodes represent workflow types, sized by execution count. Edges show
 * sub-workflow relationships. Auto-refreshes every 5 seconds.
 */
export default function WorkflowTopology() {
  const { data: stats } = useQuery({
    queryKey: ["workflowStats"],
    queryFn: () => workflowApi.stats(),
    refetchInterval: 5000,
  });

  const { data: queueSizes } = useQuery({
    queryKey: ["queueSizes"],
    queryFn: () => taskApi.queueSizes(),
    refetchInterval: 5000,
  });

  const { data: running } = useQuery({
    queryKey: ["runningWorkflows"],
    queryFn: () => workflowApi.search({ status: "RUNNING", size: 100 }),
    refetchInterval: 5000,
  });

  const { nodes, edges } = useMemo(() => {
    const nodeList: Node[] = [];
    const edgeList: Edge[] = [];

    // Workflow type nodes from running workflows
    const typeCount = new Map<string, { count: number; statuses: Map<string, number> }>();
    if (running?.results) {
      for (const wf of running.results) {
        const existing = typeCount.get(wf.workflowType) ?? { count: 0, statuses: new Map() };
        existing.count++;
        existing.statuses.set(wf.status, (existing.statuses.get(wf.status) ?? 0) + 1);
        typeCount.set(wf.workflowType, existing);
      }
    }

    const cols = Math.max(Math.ceil(Math.sqrt(typeCount.size + (queueSizes ? Object.keys(queueSizes).length : 0))), 3);
    let idx = 0;

    // Workflow type nodes
    for (const [name, info] of typeCount) {
      const size = Math.min(40 + info.count * 8, 120);
      nodeList.push({
        id: `wf-${name}`,
        position: { x: (idx % cols) * 240, y: Math.floor(idx / cols) * 180 },
        data: {
          label: (
            <div className="text-center">
              <div className="font-semibold text-xs">{name}</div>
              <div className="text-[10px] mt-1">
                {Array.from(info.statuses.entries()).map(([st, cnt]) => (
                  <span key={st} className="mr-1">
                    <span className="inline-block w-1.5 h-1.5 rounded-full mr-0.5" style={{ backgroundColor: STATUS_COLORS[st] ?? "#9ca3af" }} />
                    {cnt}
                  </span>
                ))}
              </div>
            </div>
          ),
        },
        style: {
          width: size + 60,
          height: size,
          borderRadius: "50%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          border: `2px solid ${STATUS_COLORS.RUNNING}`,
          background: `${STATUS_COLORS.RUNNING}10`,
        },
      });
      idx++;
    }

    // Queue nodes
    if (queueSizes) {
      for (const [queue, size] of Object.entries(queueSizes)) {
        const nodeSize = Math.min(30 + size * 2, 80);
        nodeList.push({
          id: `q-${queue}`,
          position: { x: (idx % cols) * 240, y: Math.floor(idx / cols) * 180 },
          data: {
            label: (
              <div className="text-center">
                <div className="text-[10px] text-muted-foreground">Queue</div>
                <div className="font-semibold text-xs">{queue}</div>
                <div className="text-sm font-bold mt-0.5">{size}</div>
              </div>
            ),
          },
          style: {
            width: nodeSize + 60,
            borderRadius: 8,
            border: `2px solid ${size > 10 ? "#f59e0b" : "#10b981"}`,
            background: size > 10 ? "#f59e0b10" : "#10b98110",
          },
        });

        // Connect workflow types to their queues (heuristic: name match)
        for (const [wfName] of typeCount) {
          if (queue.toLowerCase().includes(wfName.toLowerCase()) || wfName.toLowerCase().includes(queue.toLowerCase())) {
            edgeList.push({
              id: `wf-${wfName}-q-${queue}`,
              source: `wf-${wfName}`,
              target: `q-${queue}`,
              markerEnd: { type: MarkerType.ArrowClosed },
              animated: true,
              style: { stroke: "#9ca3af" },
            });
          }
        }

        idx++;
      }
    }

    return { nodes: nodeList, edges: edgeList };
  }, [running, queueSizes]);

  const totalRunning = running?.totalHits ?? 0;
  const totalQueued = queueSizes ? Object.values(queueSizes).reduce((a, b) => a + b, 0) : 0;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold tracking-tight">Workflow Topology</h2>
        <div className="flex gap-2">
          <Badge variant="outline">{totalRunning} running</Badge>
          <Badge variant="secondary">{totalQueued} queued</Badge>
        </div>
      </div>

      {/* Stats overview */}
      {stats && Object.keys(stats).length > 0 && (
        <div className="flex gap-2 flex-wrap">
          {Object.entries(stats).map(([k, v]) => (
            <Card key={k} className="px-3 py-2">
              <p className="text-[10px] text-muted-foreground">{k}</p>
              <p className="text-lg font-bold">{v}</p>
            </Card>
          ))}
        </div>
      )}

      {/* Topology graph */}
      <div style={{ height: 550 }} className="rounded border">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          fitView
          proOptions={{ hideAttribution: true }}
        >
          <Background />
          <Controls />
          <MiniMap />
        </ReactFlow>
      </div>

      {/* Legend */}
      <Card className="mt-4">
        <CardContent className="pt-4 pb-4">
          <h3 className="text-sm font-semibold mb-3">Legend</h3>
          <div className="flex flex-wrap gap-x-8 gap-y-3 text-xs">
            <div>
              <p className="font-medium text-muted-foreground mb-1.5">Node Shapes</p>
              <div className="flex items-center gap-4">
                <div className="flex items-center gap-1.5">
                  <div className="w-6 h-5 rounded-full border-2" style={{ borderColor: STATUS_COLORS.RUNNING, background: `${STATUS_COLORS.RUNNING}10` }} />
                  <span>Workflow Type</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <div className="w-6 h-5 rounded border-2" style={{ borderColor: "#10b981", background: "#10b98110" }} />
                  <span>Task Queue</span>
                </div>
              </div>
            </div>
            <div>
              <p className="font-medium text-muted-foreground mb-1.5">Status Colors</p>
              <div className="flex flex-wrap items-center gap-3">
                {Object.entries(STATUS_COLORS).map(([status, color]) => (
                  <div key={status} className="flex items-center gap-1.5">
                    <span className="inline-block w-2.5 h-2.5 rounded-full" style={{ backgroundColor: color }} />
                    <span>{status}</span>
                  </div>
                ))}
              </div>
            </div>
            <div>
              <p className="font-medium text-muted-foreground mb-1.5">Size & Edges</p>
              <div className="space-y-1 text-muted-foreground">
                <p>Larger nodes = more active executions</p>
                <p>Animated edges = workflow-to-queue connections</p>
                <p>Queue border turns amber when depth &gt; 10</p>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
