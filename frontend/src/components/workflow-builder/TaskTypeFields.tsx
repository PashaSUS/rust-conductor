import type { TaskFormState } from "./types";
import { createEmptyTask } from "./types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Plus, Trash2 } from "lucide-react";

interface TypeFieldProps {
  task: TaskFormState;
  onUpdate: (u: Partial<TaskFormState>) => void;
}

export function SubWorkflowFields({ task, onUpdate }: TypeFieldProps) {
  return (
    <div className="border-t pt-3 space-y-3">
      <p className="text-xs font-semibold text-muted-foreground">Sub-Workflow Configuration</p>
      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label>Workflow Name</Label>
          <Input
            value={task.subWorkflowName}
            onChange={(e) => onUpdate({ subWorkflowName: e.target.value })}
            placeholder="sub_workflow_name"
          />
        </div>
        <div className="space-y-2">
          <Label>Version (optional)</Label>
          <Input
            value={task.subWorkflowVersion}
            onChange={(e) => onUpdate({ subWorkflowVersion: e.target.value })}
            placeholder="Latest"
          />
        </div>
      </div>
    </div>
  );
}

export function ForkJoinFields({ task, onUpdate }: TypeFieldProps) {
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
        <p className="text-xs font-semibold text-muted-foreground">Parallel Branches</p>
        <Button size="sm" variant="outline" onClick={addBranch}>
          <Plus className="h-3 w-3 mr-1" />
          Add Branch
        </Button>
      </div>
      <div className="grid gap-3" style={{ gridTemplateColumns: `repeat(${Math.min(task.forkBranches.length, 3)}, 1fr)` }}>
        {task.forkBranches.map((branch, bi) => (
          <div key={bi} className="border rounded-lg p-3 bg-muted/20 space-y-2">
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
                  <span className="text-xs font-medium">{bt.taskReferenceName || "(unnamed)"}</span>
                  <Button variant="ghost" size="icon" className="h-5 w-5" onClick={() => removeTaskFromBranch(bi, ti)}>
                    <Trash2 className="h-2.5 w-2.5" />
                  </Button>
                </div>
                <Input
                  className="h-7 text-xs"
                  placeholder="Reference name"
                  value={bt.taskReferenceName}
                  onChange={(e) => updateBranchTask(bi, ti, { taskReferenceName: e.target.value })}
                />
                <Input
                  className="h-7 text-xs"
                  placeholder="Task name"
                  value={bt.name}
                  onChange={(e) => updateBranchTask(bi, ti, { name: e.target.value })}
                />
              </div>
            ))}
            <Button size="sm" variant="ghost" className="w-full text-xs h-7" onClick={() => addTaskToBranch(bi)}>
              <Plus className="h-3 w-3 mr-1" />
              Add Task
            </Button>
          </div>
        ))}
      </div>
      <p className="text-[10px] text-muted-foreground">
        Note: A JOIN task will be automatically appended after the fork. Make sure each branch task has a unique reference name.
      </p>
    </div>
  );
}

export function DecisionFields({ task, onUpdate }: TypeFieldProps) {
  const addCase = () => {
    onUpdate({
      decisionCases: [...task.decisionCases, { caseName: `case${task.decisionCases.length + 1}`, tasks: [] }],
    });
  };

  const removeCase = (idx: number) => {
    onUpdate({ decisionCases: task.decisionCases.filter((_, i) => i !== idx) });
  };

  const addTaskToCase = (caseIdx: number) => {
    const newCases = [...task.decisionCases];
    const newTask = createEmptyTask();
    newTask.taskReferenceName = `${task.taskReferenceName}_case${caseIdx + 1}_t${newCases[caseIdx].tasks.length + 1}`;
    newCases[caseIdx] = { ...newCases[caseIdx], tasks: [...newCases[caseIdx].tasks, newTask] };
    onUpdate({ decisionCases: newCases });
  };

  return (
    <div className="border-t pt-3 space-y-3">
      <p className="text-xs font-semibold text-muted-foreground">Decision/Switch Configuration</p>
      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label>Case Expression</Label>
          <Input
            value={task.caseExpression}
            onChange={(e) => onUpdate({ caseExpression: e.target.value })}
            placeholder="$.taskReferenceName.output.value"
          />
        </div>
        <div className="space-y-2">
          <Label>Case Value Param</Label>
          <Input
            value={task.caseValueParam}
            onChange={(e) => onUpdate({ caseValueParam: e.target.value })}
            placeholder="Optional"
          />
        </div>
      </div>
      <Separator />
      <div className="flex items-center justify-between">
        <p className="text-xs font-medium">Cases</p>
        <Button size="sm" variant="outline" onClick={addCase}>
          <Plus className="h-3 w-3 mr-1" />
          Add Case
        </Button>
      </div>
      {task.decisionCases.map((c, ci) => (
        <div key={ci} className="border rounded-lg p-3 bg-muted/20 space-y-2">
          <div className="flex items-center gap-2">
            <Input
              className="h-7 text-xs font-medium flex-1"
              value={c.caseName}
              onChange={(e) => {
                const newCases = [...task.decisionCases];
                newCases[ci] = { ...newCases[ci], caseName: e.target.value };
                onUpdate({ decisionCases: newCases });
              }}
              placeholder="Case value"
            />
            <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => removeCase(ci)}>
              <Trash2 className="h-3 w-3" />
            </Button>
          </div>
          {c.tasks.map((ct, ti) => (
            <div key={ti} className="border rounded p-2 bg-background">
              <div className="flex items-center gap-2">
                <Input
                  className="h-7 text-xs flex-1"
                  placeholder="Reference name"
                  value={ct.taskReferenceName}
                  onChange={(e) => {
                    const newCases = [...task.decisionCases];
                    newCases[ci] = {
                      ...newCases[ci],
                      tasks: newCases[ci].tasks.map((t, i) => i === ti ? { ...t, taskReferenceName: e.target.value } : t),
                    };
                    onUpdate({ decisionCases: newCases });
                  }}
                />
                <Input
                  className="h-7 text-xs flex-1"
                  placeholder="Task name"
                  value={ct.name}
                  onChange={(e) => {
                    const newCases = [...task.decisionCases];
                    newCases[ci] = {
                      ...newCases[ci],
                      tasks: newCases[ci].tasks.map((t, i) => i === ti ? { ...t, name: e.target.value } : t),
                    };
                    onUpdate({ decisionCases: newCases });
                  }}
                />
                <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => {
                  const newCases = [...task.decisionCases];
                  newCases[ci] = { ...newCases[ci], tasks: newCases[ci].tasks.filter((_, i) => i !== ti) };
                  onUpdate({ decisionCases: newCases });
                }}>
                  <Trash2 className="h-2.5 w-2.5" />
                </Button>
              </div>
            </div>
          ))}
          <Button size="sm" variant="ghost" className="w-full text-xs h-7" onClick={() => addTaskToCase(ci)}>
            <Plus className="h-3 w-3 mr-1" />
            Add Task
          </Button>
        </div>
      ))}
    </div>
  );
}

export function DoWhileFields({ task, onUpdate }: TypeFieldProps) {
  const addLoopTask = () => {
    const newTask = createEmptyTask();
    newTask.taskReferenceName = `${task.taskReferenceName}_loop_t${task.loopTasks.length + 1}`;
    onUpdate({ loopTasks: [...task.loopTasks, newTask] });
  };

  return (
    <div className="border-t pt-3 space-y-3">
      <p className="text-xs font-semibold text-muted-foreground">Do-While Loop Configuration</p>
      <div className="space-y-2">
        <Label>Loop Condition</Label>
        <Input
          value={task.loopCondition}
          onChange={(e) => onUpdate({ loopCondition: e.target.value })}
          placeholder='if ($.task_ref.output.value > 0) { true; } else { false; }'
        />
      </div>
      <Separator />
      <div className="flex items-center justify-between">
        <p className="text-xs font-medium">Loop Body Tasks</p>
        <Button size="sm" variant="outline" onClick={addLoopTask}>
          <Plus className="h-3 w-3 mr-1" />
          Add Task
        </Button>
      </div>
      {task.loopTasks.map((lt, ti) => (
        <div key={ti} className="border rounded p-2 bg-muted/20">
          <div className="flex items-center gap-2">
            <Input
              className="h-7 text-xs flex-1"
              placeholder="Reference name"
              value={lt.taskReferenceName}
              onChange={(e) => {
                const newTasks = [...task.loopTasks];
                newTasks[ti] = { ...newTasks[ti], taskReferenceName: e.target.value };
                onUpdate({ loopTasks: newTasks });
              }}
            />
            <Input
              className="h-7 text-xs flex-1"
              placeholder="Task name"
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
