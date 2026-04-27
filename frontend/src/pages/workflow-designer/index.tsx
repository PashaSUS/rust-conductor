import { useCallback, useMemo } from "react";
import {
  ReactFlow, Background, Controls, MiniMap,
  ReactFlowProvider, useReactFlow,
  type Node, type Edge,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { SearchableSelect } from "@/components/SearchableSelect";
import { JsonView } from "@/components/JsonView";
import {
  Save, Eye, Settings2, AlertTriangle, ArrowDown, GripHorizontal,
  Code, Copy, ChevronUp, ChevronDown, LayoutGrid, Trash2,
} from "lucide-react";
import type { ContextMenuItem } from "./types";
import { TASK_TYPE_GROUPS, TASK_TYPE_COLORS, DATA_EDGE_STYLE } from "./constants";
import { WorkflowStartNode, WorkflowEndNode, DesignerTaskNode, DesignerActionsContext } from "./nodes";
import { ContextMenu } from "./ContextMenu";
import { TaskEditorPanel } from "./TaskEditorPanel";
import { WorkflowSettingsPanel } from "./WorkflowSettingsPanel";
import { TaskPalette } from "./TaskPalette";
import { useWorkflowState } from "./useWorkflowState";
import { useWorkflowActions } from "./useWorkflowActions";

export default function WorkflowDesigner() {
  return <ReactFlowProvider><WorkflowDesignerInner /></ReactFlowProvider>;
}

function WorkflowDesignerInner() {
  const { screenToFlowPosition } = useReactFlow();
  const state = useWorkflowState();
  const actions = useWorkflowActions(state);

  const {
    workflowName, setWorkflowName, workflowVersion, setWorkflowVersion,
    workflowDesc, setWorkflowDesc, workflowInputKeys,
    tasks, editingTask, setEditingTask, nodes, edges,
    showPreview, setShowPreview, showSettings, setShowSettings,
    isDragOver, setIsDragOver, ctxMenu, setCtxMenu,
    reactFlowRef, tasksRef,
    rightPanelWidth, onResizeStart,
    workflowDefs, workflowDefOptions, taskDefMap, taskDefOptions,
    workflowOutputParams, hasStart,
    onNodesChange, onEdgesChange,
    inputKeyDraft, setInputKeyDraft, addInputKey, removeInputKey,
    workflowDef, validationErrors, saveMut,
  } = state;

  const { onConnect, addTask, removeTask, duplicateTask, moveTask, updateTask, importWorkflow, autoLayout, clearAll } = actions;

  // Node type registry — useMemo avoids Fast Refresh warning
  const nodeTypes = useMemo(() => ({
    designerTask: DesignerTaskNode,
    workflowStart: WorkflowStartNode,
    workflowEnd: WorkflowEndNode,
  }), []);

  // Context value for node edit/delete actions
  const designerActions = useMemo(() => ({
    editTask: (id: string) => {
      const task = tasksRef.current.find((t) => t.id === id);
      if (task) { setShowSettings(false); setEditingTask(task); }
    },
    deleteTask: removeTask,
  }), [removeTask, setEditingTask, setShowSettings, tasksRef]);

  // DnD handlers
  const onDrop = useCallback((event: React.DragEvent) => {
    event.preventDefault(); setIsDragOver(false);
    const type = event.dataTransfer.getData("application/task-type");
    if (!type) return;
    addTask(type, screenToFlowPosition({ x: event.clientX, y: event.clientY }));
  }, [addTask, screenToFlowPosition, setIsDragOver]);
  const onDragOver = useCallback((event: React.DragEvent) => { event.preventDefault(); event.dataTransfer.dropEffect = "move"; setIsDragOver(true); }, [setIsDragOver]);
  const onDragLeave = useCallback(() => setIsDragOver(false), [setIsDragOver]);

  // Node click
  const onNodeClick = useCallback((_event: React.MouseEvent, node: Node) => {
    if (node.id === "__start__" || node.id === "__end__") { setEditingTask(null); setShowSettings(true); return; }
    const task = tasksRef.current.find((t) => t.id === node.id);
    if (task) { setShowSettings(false); setEditingTask(task); }
  }, [tasksRef, setEditingTask, setShowSettings]);

  // Context menus
  const onCanvasContextMenu = useCallback((event: React.MouseEvent | MouseEvent) => {
    event.preventDefault();
    const flowPos = screenToFlowPosition({ x: event.clientX, y: event.clientY });
    const addItems: ContextMenuItem[] = TASK_TYPE_GROUPS.flatMap((group) => [
      { label: group.label, onClick: () => {}, disabled: true, separator: false },
      ...group.types.map((t) => ({ label: t.label, icon: <span className="w-2.5 h-2.5 rounded-sm" style={{ backgroundColor: TASK_TYPE_COLORS[t.value] }} />, onClick: () => addTask(t.value, flowPos) })),
      { label: "", onClick: () => {}, separator: true },
    ]);
    addItems.pop();
    setCtxMenu({ x: event.clientX, y: event.clientY, items: [...addItems, { label: "", onClick: () => {}, separator: true }, { label: "Auto Layout", icon: <LayoutGrid className="h-3.5 w-3.5" />, onClick: autoLayout }, { label: "Clear All", icon: <Trash2 className="h-3.5 w-3.5" />, onClick: clearAll, danger: true }] });
  }, [addTask, autoLayout, clearAll, screenToFlowPosition, setCtxMenu]);

  const onNodeContextMenu = useCallback((event: React.MouseEvent, node: Node) => {
    event.preventDefault();
    if (node.id === "__start__" || node.id === "__end__") {
      setCtxMenu({ x: event.clientX, y: event.clientY, items: [{ label: "Edit Settings", icon: <Settings2 className="h-3.5 w-3.5" />, onClick: () => setShowSettings(true) }] });
      return;
    }
    const currentTasks = tasksRef.current;
    const task = currentTasks.find((t) => t.id === node.id);
    if (!task) return;
    const idx = currentTasks.indexOf(task);
    setCtxMenu({ x: event.clientX, y: event.clientY, items: [
      { label: "Edit Task", icon: <Code className="h-3.5 w-3.5" />, onClick: () => setEditingTask(task) },
      { label: "Duplicate", icon: <Copy className="h-3.5 w-3.5" />, onClick: () => duplicateTask(task.id) },
      { label: "", onClick: () => {}, separator: true },
      { label: "Move Up (run earlier)", icon: <ChevronUp className="h-3.5 w-3.5" />, onClick: () => moveTask(task.id, "up"), disabled: idx === 0 },
      { label: "Move Down (run later)", icon: <ChevronDown className="h-3.5 w-3.5" />, onClick: () => moveTask(task.id, "down"), disabled: idx === currentTasks.length - 1 },
      { label: "", onClick: () => {}, separator: true },
      { label: "Delete Task", icon: <Trash2 className="h-3.5 w-3.5" />, onClick: () => removeTask(task.id), danger: true },
    ] });
  }, [duplicateTask, moveTask, removeTask, tasksRef, setCtxMenu, setEditingTask, setShowSettings]);

  const onEdgeContextMenu = useCallback((event: React.MouseEvent, edge: Edge) => {
    event.preventDefault();
    setCtxMenu({ x: event.clientX, y: event.clientY, items: [{ label: `Delete connection${edge.label ? ` (${edge.label})` : ""}`, icon: <Trash2 className="h-3.5 w-3.5" />, onClick: () => state.setEdges((eds) => eds.filter((e) => e.id !== edge.id)), danger: true }] });
  }, [setCtxMenu, state]);

  return (
    <div className="flex flex-col h-[calc(100vh-4rem)]">
      {ctxMenu && <ContextMenu x={ctxMenu.x} y={ctxMenu.y} items={ctxMenu.items} onClose={() => setCtxMenu(null)} />}

      {/* Top toolbar */}
      <div className="flex items-center justify-between px-4 py-2 border-b bg-background shrink-0">
        <div className="flex items-center gap-3">
          <h2 className="text-lg font-bold tracking-tight">Workflow Designer</h2>
          <Badge variant="outline" className="text-xs">{tasks.length} task{tasks.length !== 1 ? "s" : ""}</Badge>
          {validationErrors.length > 0 && <Badge variant="destructive" className="text-xs gap-1"><AlertTriangle className="h-3 w-3" />{validationErrors.length}</Badge>}
        </div>
        <div className="flex items-center gap-2">
          <SearchableSelect options={workflowDefOptions} value="" onChange={(val) => { const [name, ver] = val.split("::"); const def = workflowDefs?.find((d) => d.name === name && d.version === Number(ver)); if (def) importWorkflow(def); }} placeholder="Import workflow..." />
          <div className="w-px h-5 bg-border" />
          <Button variant="outline" size="sm" onClick={autoLayout} disabled={tasks.length === 0}><LayoutGrid className="h-3.5 w-3.5 mr-1" />Layout</Button>
          <Button variant="outline" size="sm" className="text-destructive hover:text-destructive" onClick={clearAll} disabled={tasks.length === 0}><Trash2 className="h-3.5 w-3.5 mr-1" />Clear</Button>
          <div className="w-px h-5 bg-border" />
          <Button variant={showSettings ? "secondary" : "outline"} size="sm" onClick={() => { setShowSettings(!showSettings); if (!showSettings) setEditingTask(null); }}><Settings2 className="h-3.5 w-3.5 mr-1" />Settings</Button>
          <Button variant="outline" size="sm" onClick={() => setShowPreview(!showPreview)}><Eye className="h-3.5 w-3.5 mr-1" />{showPreview ? "Hide" : "JSON"}</Button>
          <Button size="sm" onClick={() => saveMut.mutate()} disabled={saveMut.isPending || validationErrors.length > 0}><Save className="h-3.5 w-3.5 mr-1" />Save</Button>
        </div>
      </div>

      {/* Main: palette + canvas + editor */}
      <div className="flex flex-1 min-h-0">
        <TaskPalette tasks={tasks} editingTask={editingTask} setEditingTask={setEditingTask} setShowSettings={setShowSettings} moveTask={moveTask} />

        {/* Canvas */}
        <div className={`flex-1 min-w-0 transition-all ${isDragOver ? "ring-2 ring-inset ring-primary/40 bg-primary/5" : ""}`} ref={reactFlowRef} onDrop={onDrop} onDragOver={onDragOver} onDragLeave={onDragLeave}>
          {tasks.length === 0 && !hasStart ? (
            <div className="h-full flex items-center justify-center">
              <div className="text-center max-w-sm">
                <div className="w-16 h-16 rounded-2xl bg-muted/50 flex items-center justify-center mx-auto mb-4"><ArrowDown className="h-8 w-8 text-muted-foreground/50" /></div>
                <h3 className="text-sm font-semibold text-muted-foreground mb-2">Build your workflow</h3>
                <div className="text-xs text-muted-foreground space-y-1 text-left bg-muted/30 rounded-lg p-3">
                  <p><b>Drag</b> a task from the palette, or <b>right-click</b> here to add one</p>
                  <p><b>Gray handles</b> (top/bottom): connect flow order between tasks</p>
                  <p><b>Colored handles</b> (left/right): wire data between inputs &amp; outputs</p>
                  <p>Draw flow: Start → Task → Task → End. Draw data: output → input</p>
                </div>
              </div>
            </div>
          ) : (
            <DesignerActionsContext.Provider value={designerActions}>
              <ReactFlow nodes={nodes} edges={edges} onNodesChange={onNodesChange} onEdgesChange={onEdgesChange} onConnect={onConnect} onNodeClick={onNodeClick} onPaneContextMenu={onCanvasContextMenu} onNodeContextMenu={onNodeContextMenu} onEdgeContextMenu={onEdgeContextMenu} nodeTypes={nodeTypes} fitView proOptions={{ hideAttribution: true }} snapToGrid snapGrid={[20, 20]} defaultEdgeOptions={DATA_EDGE_STYLE} connectionLineStyle={{ stroke: "#3b82f6", strokeWidth: 1.5 }}>
                <Background gap={20} /><Controls /><MiniMap nodeStrokeWidth={3} style={{ height: 100, width: 140 }} />
              </ReactFlow>
            </DesignerActionsContext.Provider>
          )}
        </div>

        {/* Right panel */}
        {(editingTask || showPreview || showSettings) && (
          <div className="relative flex shrink-0" style={{ width: rightPanelWidth }}>
            <div className="w-1.5 cursor-col-resize hover:bg-primary/20 active:bg-primary/30 transition-colors flex items-center justify-center group" onMouseDown={onResizeStart}>
              <GripHorizontal className="h-4 w-4 text-muted-foreground/30 group-hover:text-muted-foreground/60 rotate-90" />
            </div>
            <div className="flex-1 border-l bg-background overflow-y-auto">
              {showSettings && !editingTask && (
                <WorkflowSettingsPanel workflowName={workflowName} setWorkflowName={setWorkflowName} workflowVersion={workflowVersion} setWorkflowVersion={setWorkflowVersion} workflowDesc={workflowDesc} setWorkflowDesc={setWorkflowDesc} workflowInputKeys={workflowInputKeys} inputKeyDraft={inputKeyDraft} setInputKeyDraft={setInputKeyDraft} addInputKey={addInputKey} removeInputKey={removeInputKey} workflowOutputParams={workflowOutputParams} validationErrors={validationErrors} onClose={() => setShowSettings(false)} />
              )}
              {editingTask && (
                <TaskEditorPanel editingTask={editingTask} setEditingTask={setEditingTask} tasks={tasks} taskDefMap={taskDefMap} taskDefOptions={taskDefOptions} workflowDefOptions={workflowDefOptions} workflowName={workflowName} onApply={updateTask} />
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
