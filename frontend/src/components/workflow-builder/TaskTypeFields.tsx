import type { TaskFormState } from "./types";
import { createEmptyTask } from "./types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Plus, Trash2 } from "lucide-react";
import { useThemeText } from "@/components/ThemeContext";

interface TypeFieldProps {
  task: TaskFormState;
  onUpdate: (u: Partial<TaskFormState>) => void;
}

export function SubWorkflowFields({ task, onUpdate }: TypeFieldProps) {
  const t = useThemeText();
  return (
    <div className="border-t pt-3 space-y-3">
      <p className="text-xs font-semibold text-muted-foreground">{t.subWorkflowConfig}</p>
      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label>{t.workflowName}</Label>
          <Input
            value={task.subWorkflowName}
            onChange={(e) => onUpdate({ subWorkflowName: e.target.value })}
            placeholder="sub_workflow_name"
          />
        </div>
        <div className="space-y-2">
          <Label>{t.versionOptional}</Label>
          <Input
            value={task.subWorkflowVersion}
            onChange={(e) => onUpdate({ subWorkflowVersion: e.target.value })}
            placeholder={t.latest}
          />
        </div>
      </div>
    </div>
  );
}

export function ForkJoinFields({ task, onUpdate }: TypeFieldProps) {
  const t = useThemeText();
  const addBranch = () => {
    onUpdate({
      forkBranches: [
        ...task.forkBranches,
        { name: `Branch ${task.forkBranches.length + 1}`, tasks: [] },
      ],
    });
  };

  const removeBranch = (idx: number) => {
    onUpdate({ forkBranches: task.forkBranches.filter((_, i) => i !== idx) });
  };

  const addTaskToBranch = (branchIdx: number) => {
    const newBranches = [...task.forkBranches];
    const newTask = createEmptyTask();
    newTask.taskReferenceName = `${task.taskReferenceName}_b${branchIdx + 1}_t${newBranches[branchIdx].tasks.length + 1}`;
    newBranches[branchIdx] = {
      ...newBranches[branchIdx],
      tasks: [...newBranches[branchIdx].tasks, newTask],
    };
    onUpdate({ forkBranches: newBranches });
  };

  const removeTaskFromBranch = (branchIdx: number, taskIdx: number) => {
    const newBranches = [...task.forkBranches];
    newBranches[branchIdx] = {
      ...newBranches[branchIdx],
      tasks: newBranches[branchIdx].tasks.filter((_, i) => i !== taskIdx),
    };
    onUpdate({ forkBranches: newBranches });
  };

  const updateBranchTask = (branchIdx: number, taskIdx: number, updates: Partial<TaskFormState>) => {
    const newBranches = [...task.forkBranches];
    newBranches[branchIdx] = {
      ...newBranches[branchIdx],
      tasks: newBranches[branchIdx].tasks.map((t, i) =>
        i === taskIdx ? { ...t, ...updates } : t
      ),
    };
    onUpdate({ forkBranches: newBranches });
  };

  return (
    <div className="border-t pt-3 space-y-3">
      <div className="flex items-center justify-between">
        <p className="text-xs font-semibold text-muted-foreground">{t.parallelBranches}</p>
        <Button size="sm" variant="outline" onClick={addBranch}>
          <Plus className="h-3 w-3 mr-1" />
          {t.addBranch}
        </Button>
      </div>
      <div className="grid gap-3" style={{ gridTemplateColumns: `repeat(${Math.min(task.forkBranches.length, 3)}, 1fr)` }}>
        {task.forkBranches.map((branch, bi) => (
          <div key={bi} className="border rounded-lg p-3 bg-muted space-y-2">
            <div className="flex items-center justify-between">
              <Input
                className="h-7 text-xs font-medium"
                value={branch.name}
                onChange={(e) => {
                  const newBranches = [...task.forkBranches];
                  newBranches[bi] = { ...newBranches[bi], name: e.target.value };
                  onUpdate({ forkBranches: newBranches });
                }}
              />
              {task.forkBranches.length > 1 && (
                <Button variant="ghost" size="icon" className="h-6 w-6 ml-1" onClick={() => removeBranch(bi)}>
                  <Trash2 className="h-3 w-3" />
                </Button>
              )}
            </div>
            {branch.tasks.map((bt, ti) => (
              <div key={ti} className="border rounded p-2 bg-background space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-medium">{bt.taskReferenceName || t.unnamed}</span>
                  <Button variant="ghost" size="icon" className="h-5 w-5" onClick={() => removeTaskFromBranch(bi, ti)}>
                    <Trash2 className="h-2.5 w-2.5" />
                  </Button>
                </div>
                <Input
                  className="h-7 text-xs"
                  placeholder={t.referenceName}
                  value={bt.taskReferenceName}
                  onChange={(e) => updateBranchTask(bi, ti, { taskReferenceName: e.target.value })}
                />
                <Input
                  className="h-7 text-xs"
                  placeholder={t.taskName}
                  value={bt.name}
                  onChange={(e) => updateBranchTask(bi, ti, { name: e.target.value })}
                />
              </div>
            ))}
            <Button size="sm" variant="ghost" className="w-full text-xs h-7" onClick={() => addTaskToBranch(bi)}>
              <Plus className="h-3 w-3 mr-1" />
              {t.addTask}
            </Button>
          </div>
        ))}
      </div>
      <p className="text-[10px] text-muted-foreground">
        {t.forkJoinNote}
      </p>
    </div>
  );
}

export function DoWhileFields({ task, onUpdate }: TypeFieldProps) {
  const t = useThemeText();
  const addLoopTask = () => {
    const newTask = createEmptyTask();
    newTask.taskReferenceName = `${task.taskReferenceName}_loop_t${task.loopTasks.length + 1}`;
    onUpdate({ loopTasks: [...task.loopTasks, newTask] });
  };

  return (
    <div className="border-t pt-3 space-y-3">
      <p className="text-xs font-semibold text-muted-foreground">{t.doWhileConfig}</p>
      <div className="space-y-2">
        <Label>{t.loopCondition}</Label>
        <Input
          value={task.loopCondition}
          onChange={(e) => onUpdate({ loopCondition: e.target.value })}
          placeholder='if ($.task_ref.output.value > 0) { true; } else { false; }'
        />
      </div>
      <Separator />
      <div className="flex items-center justify-between">
        <p className="text-xs font-medium">{t.loopBodyTasks}</p>
        <Button size="sm" variant="outline" onClick={addLoopTask}>
          <Plus className="h-3 w-3 mr-1" />
          {t.addTask}
        </Button>
      </div>
      {task.loopTasks.map((lt, ti) => (
        <div key={ti} className="border rounded p-2 bg-muted">
          <div className="flex items-center gap-2">
            <Input
              className="h-7 text-xs flex-1"
              placeholder={t.referenceName}
              value={lt.taskReferenceName}
              onChange={(e) => {
                const newTasks = [...task.loopTasks];
                newTasks[ti] = { ...newTasks[ti], taskReferenceName: e.target.value };
                onUpdate({ loopTasks: newTasks });
              }}
            />
            <Input
              className="h-7 text-xs flex-1"
              placeholder={t.taskName}
              value={lt.name}
              onChange={(e) => {
                const newTasks = [...task.loopTasks];
                newTasks[ti] = { ...newTasks[ti], name: e.target.value };
                onUpdate({ loopTasks: newTasks });
              }}
            />
            <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => {
              onUpdate({ loopTasks: task.loopTasks.filter((_, i) => i !== ti) });
            }}>
              <Trash2 className="h-3 w-3" />
            </Button>
          </div>
        </div>
      ))}
    </div>
  );
}
