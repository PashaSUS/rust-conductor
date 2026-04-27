import { Badge } from "@/components/ui/badge";
import { GripVertical, ChevronUp, ChevronDown, GitBranch, GitFork, Repeat } from "lucide-react";
import type { DesignerTask } from "./types";
import { TASK_TYPE_GROUPS, TASK_TYPE_COLORS } from "./constants";

interface TaskPaletteProps {
  tasks: DesignerTask[];
  editingTask: DesignerTask | null;
  setEditingTask: (task: DesignerTask | null) => void;
  setShowSettings: (v: boolean) => void;
  moveTask: (id: string, direction: "up" | "down") => void;
}

const BRANCH_ICON: Record<string, React.ReactNode> = {
  DECISION: <GitBranch className="h-2.5 w-2.5" />,
  SWITCH: <GitBranch className="h-2.5 w-2.5" />,
  FORK_JOIN: <GitFork className="h-2.5 w-2.5" />,
  DYNAMIC_FORK_JOIN: <GitFork className="h-2.5 w-2.5" />,
  DO_WHILE: <Repeat className="h-2.5 w-2.5" />,
};

export function TaskPalette({ tasks, editingTask, setEditingTask, setShowSettings, moveTask }: TaskPaletteProps) {
  return (
    <div className="w-56 border-r bg-muted/20 shrink-0 flex flex-col">
      <div className="px-3 py-2.5 border-b shrink-0">
        <p className="text-xs font-semibold">Task Palette</p>
        <p className="text-[10px] text-muted-foreground mt-0.5">Drag onto canvas or right-click canvas</p>
      </div>
      <div className={`overflow-y-auto ${tasks.length > 0 ? "max-h-[40%] border-b" : "flex-1"}`}>
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

      {tasks.length > 0 && (
        <div className="flex-1 min-h-0 flex flex-col">
          <div className="px-3 py-2.5 border-b shrink-0 flex items-center justify-between">
            <div>
              <p className="text-xs font-semibold">Execution Order</p>
              <p className="text-[10px] text-muted-foreground mt-0.5">Reorder with arrows</p>
            </div>
            <Badge variant="secondary" className="text-[10px]">{tasks.length}</Badge>
          </div>
          <div className="overflow-y-auto flex-1 min-h-0">
            {tasks.map((task, idx) => {
              const color = TASK_TYPE_COLORS[task.type] ?? "#9ca3af";
              return (
                <div
                  key={task.id}
                  className={`flex items-center gap-1.5 px-2 py-1.5 hover:bg-muted/50 transition-colors cursor-pointer ${editingTask?.id === task.id ? "bg-primary/10" : ""}`}
                  onClick={() => { setShowSettings(false); setEditingTask(task); }}
                >
                  <span className="w-5 h-5 rounded-full flex items-center justify-center text-[9px] font-bold text-white shrink-0" style={{ backgroundColor: color }}>{idx + 1}</span>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-1">
                      <p className="text-[10px] font-medium truncate">{task.taskReferenceName}</p>
                      {BRANCH_ICON[task.type] && <span style={{ color }}>{BRANCH_ICON[task.type]}</span>}
                    </div>
                    <p className="text-[8px] text-muted-foreground uppercase">{task.type}</p>
                  </div>
                  <div className="flex flex-col gap-0.5 shrink-0">
                    <button onClick={(e) => { e.stopPropagation(); moveTask(task.id, "up"); }} disabled={idx === 0} className="p-0.5 rounded hover:bg-muted disabled:opacity-20 text-muted-foreground hover:text-foreground disabled:cursor-not-allowed"><ChevronUp className="h-3 w-3" /></button>
                    <button onClick={(e) => { e.stopPropagation(); moveTask(task.id, "down"); }} disabled={idx === tasks.length - 1} className="p-0.5 rounded hover:bg-muted disabled:opacity-20 text-muted-foreground hover:text-foreground disabled:cursor-not-allowed"><ChevronDown className="h-3 w-3" /></button>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

    </div>
  );
}
