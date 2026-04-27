import { useCallback, useMemo } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  type Node,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import type { WorkflowTask, TaskResult } from "@/api/conductor";
import { useThemeText } from "@/components/ThemeContext";
import { nodeTypes, buildGraph } from "./workflow-diagram-helpers";

// ── Main Component ──

interface WorkflowDiagramProps {
  definitionTasks: WorkflowTask[];
  runtimeTasks: TaskResult[];
  onTaskClick?: (refName: string) => void;
}

export default function WorkflowDiagram({ definitionTasks, runtimeTasks, onTaskClick }: WorkflowDiagramProps) {
  const t = useThemeText();
  const { nodes, edges } = useMemo(
    () => buildGraph(definitionTasks, runtimeTasks, t),
    [definitionTasks, runtimeTasks, t],
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
