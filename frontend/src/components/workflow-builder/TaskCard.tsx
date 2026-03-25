import type { TaskDef } from "@/api/conductor";
import type { TaskFormState } from "./types";
import { TASK_TYPES, getInputSources } from "./types";
import { TaskInputFields } from "./TaskInputFields";
import { SubWorkflowFields, ForkJoinFields, DecisionFields, DoWhileFields } from "./TaskTypeFields";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Trash2,
  GripVertical,
  ChevronDown,
  ChevronRight,
  Copy,
  ArrowUp,
  ArrowDown,
} from "lucide-react";
import { useThemeText } from "@/components/ThemeContext";

interface TaskCardProps {
  task: TaskFormState;
  index: number;
  total: number;
  expanded: boolean;
  onToggle: () => void;
  onUpdate: (updates: Partial<TaskFormState>) => void;
  onRemove: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
  onDuplicate: () => void;
  registeredTasks: TaskDef[];
  allTasks: TaskFormState[];
  taskIndex: number;
  workflowInputParams: string[];
}

export function TaskCard({
  task,
  index,
  total,
  expanded,
  onToggle,
  onUpdate,
  onRemove,
  onMoveUp,
  onMoveDown,
  onDuplicate,
  registeredTasks,
  allTasks,
  taskIndex,
  workflowInputParams,
}: TaskCardProps) {
  const selectedDef = registeredTasks.find((d) => d.name === task.name);
  const lockedKeys = selectedDef?.inputKeys ?? [];
  const hasRegisteredTask = !!selectedDef;

  const sources = expanded
    ? getInputSources(allTasks, taskIndex, registeredTasks, workflowInputParams)
    : [];

  const handleSelectRegisteredTask = (taskName: string) => {
    const def = registeredTasks.find((d) => d.name === taskName);
    const fields: Record<string, string> = {};
    if (def?.inputKeys) {
      for (const key of def.inputKeys) fields[key] = task.inputFields[key] ?? "";
    }
    onUpdate({
      name: taskName,
      taskReferenceName: task.taskReferenceName || taskName,
      inputFields: fields,
      inputMode: (def?.inputKeys?.length ?? 0) > 0 ? "fields" : task.inputMode,
    });
  };

  return (
    <div className="border rounded-lg">
      <TaskCardHeader
        task={task}
        index={index}
        total={total}
        expanded={expanded}
        onToggle={onToggle}
        onMoveUp={onMoveUp}
        onMoveDown={onMoveDown}
        onDuplicate={onDuplicate}
        onRemove={onRemove}
      />

      {expanded && (
        <div className="px-4 pb-4 space-y-4 border-t pt-4">
          {/* Core fields: type, ref name, task name */}
          <TaskCoreFields
            task={task}
            onUpdate={onUpdate}
            registeredTasks={registeredTasks}
            onSelectRegisteredTask={handleSelectRegisteredTask}
          />

          {/* Description, delay, optional */}
          <TaskMetaFields task={task} onUpdate={onUpdate} />

          {/* Input parameters */}
          <TaskInputFields
            task={task}
            onUpdate={onUpdate}
            lockedKeys={lockedKeys}
            sources={sources}
            hasRegisteredTask={hasRegisteredTask}
          />

          {/* Type-specific fields */}
          {task.type === "SUB_WORKFLOW" && <SubWorkflowFields task={task} onUpdate={onUpdate} />}
          {task.type === "FORK_JOIN" && <ForkJoinFields task={task} onUpdate={onUpdate} />}
          {task.type === "DECISION" && <DecisionFields task={task} onUpdate={onUpdate} />}
          {task.type === "DO_WHILE" && <DoWhileFields task={task} onUpdate={onUpdate} />}
        </div>
      )}
    </div>
  );
}

/* ─── Sub-components ─── */

function TaskCardHeader({
  task,
  index,
  total,
  expanded,
  onToggle,
  onMoveUp,
  onMoveDown,
  onDuplicate,
  onRemove,
}: {
  task: TaskFormState;
  index: number;
  total: number;
  expanded: boolean;
  onToggle: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
  onDuplicate: () => void;
  onRemove: () => void;
}) {
  const t = useThemeText();
  return (
    <div
      className="flex items-center gap-2 px-3 py-2 cursor-pointer hover:bg-muted transition-colors"
      onClick={onToggle}
    >
      <GripVertical className="h-4 w-4 text-muted-foreground" />
      <Badge variant="secondary" className="text-[10px] w-6 justify-center">
        {index + 1}
      </Badge>
      {expanded ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
      <span className="font-medium text-sm flex-1 truncate">
        {task.taskReferenceName || t.unnamed}
      </span>
      {task.name && (
        <span className="text-xs text-muted-foreground truncate max-w-32">
          {task.name}
        </span>
      )}
      <Badge variant="outline" className="text-[10px]">{task.type}</Badge>
      {task.optional && <Badge className="text-[10px] bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200">{t.optional}</Badge>}
      <div className="flex gap-0.5" onClick={(e) => e.stopPropagation()}>
        <Button variant="ghost" size="icon" className="h-6 w-6" onClick={onMoveUp} disabled={index === 0} title={t.moveUp}>
          <ArrowUp className="h-3 w-3" />
        </Button>
        <Button variant="ghost" size="icon" className="h-6 w-6" onClick={onMoveDown} disabled={index === total - 1} title={t.moveDown}>
          <ArrowDown className="h-3 w-3" />
        </Button>
        <Button variant="ghost" size="icon" className="h-6 w-6" onClick={onDuplicate} title={t.duplicate}>
          <Copy className="h-3 w-3" />
        </Button>
        <Button variant="ghost" size="icon" className="h-6 w-6 text-destructive" onClick={onRemove} title={t.remove}>
          <Trash2 className="h-3 w-3" />
        </Button>
      </div>
    </div>
  );
}

function TaskCoreFields({
  task,
  onUpdate,
  registeredTasks,
  onSelectRegisteredTask,
}: {
  task: TaskFormState;
  onUpdate: (u: Partial<TaskFormState>) => void;
  registeredTasks: TaskDef[];
  onSelectRegisteredTask: (name: string) => void;
}) {
  const t = useThemeText();
  return (
    <div className="grid grid-cols-3 gap-4">
      <div className="space-y-2">
        <Label>{t.taskType}</Label>
        <Select value={task.type} onValueChange={(v) => onUpdate({ type: v })}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {TASK_TYPES.map((t) => (
              <SelectItem key={t.value} value={t.value}>
                <div>
                  <div className="font-medium">{t.label}</div>
                  <div className="text-[10px] text-muted-foreground">{t.description}</div>
                </div>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      <div className="space-y-2">
        <Label>{t.referenceNameRequired}</Label>
        <Input
          value={task.taskReferenceName}
          onChange={(e) => onUpdate({ taskReferenceName: e.target.value })}
          placeholder="unique_ref_name"
        />
      </div>
      <div className="space-y-2">
        <Label>{t.taskName}</Label>
        {task.type === "SIMPLE" && registeredTasks.length > 0 ? (
          <Select value={task.name} onValueChange={onSelectRegisteredTask}>
            <SelectTrigger className="text-left">
              <SelectValue placeholder={t.selectRegisteredTask} />
            </SelectTrigger>
            <SelectContent>
              {registeredTasks.map((d) => (
                <SelectItem key={d.name} value={d.name}>
                  <div>
                    <div className="font-medium">{d.name}</div>
                    {d.description && (
                      <div className="text-[10px] text-muted-foreground">{d.description}</div>
                    )}
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        ) : (
          <Input
            value={task.name}
            onChange={(e) => onUpdate({ name: e.target.value })}
            placeholder={task.type === "SIMPLE" ? "task_definition_name" : task.type.toLowerCase()}
          />
        )}
      </div>
    </div>
  );
}

function TaskMetaFields({
  task,
  onUpdate,
}: {
  task: TaskFormState;
  onUpdate: (u: Partial<TaskFormState>) => void;
}) {
  const t = useThemeText();
  return (
    <div className="grid grid-cols-2 gap-4">
      <div className="space-y-2">
        <Label>{t.description}</Label>
        <Input
          value={task.description}
          onChange={(e) => onUpdate({ description: e.target.value })}
          placeholder={t.optional}
        />
      </div>
      <div className="flex gap-4">
        <div className="space-y-2 flex-1">
          <Label>{t.startDelay}</Label>
          <Input
            type="number"
            min={0}
            value={task.startDelay}
            onChange={(e) => onUpdate({ startDelay: Number(e.target.value) || 0 })}
          />
        </div>
        <div className="space-y-2 flex items-end gap-2 pb-0.5">
          <label className="flex items-center gap-2 cursor-pointer text-sm">
            <input
              type="checkbox"
              checked={task.optional}
              onChange={(e) => onUpdate({ optional: e.target.checked })}
              className="rounded"
            />
            {t.optional}
          </label>
        </div>
      </div>
    </div>
  );
}
