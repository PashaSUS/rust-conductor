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

export function DecisionFields({ task, onUpdate }: TypeFieldProps) {
  const t = useThemeText();
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
      <p className="text-xs font-semibold text-muted-foreground">{t.decisionConfig}</p>
      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label>{t.caseExpression}</Label>
          <Input
            value={task.caseExpression}
            onChange={(e) => onUpdate({ caseExpression: e.target.value })}
            placeholder="$.taskReferenceName.output.value"
          />
        </div>
        <div className="space-y-2">
          <Label>{t.caseValueParam}</Label>
          <Input
            value={task.caseValueParam}
            onChange={(e) => onUpdate({ caseValueParam: e.target.value })}
            placeholder={t.optional}
          />
        </div>
      </div>
      <Separator />
      <div className="flex items-center justify-between">
        <p className="text-xs font-medium">{t.cases}</p>
        <Button size="sm" variant="outline" onClick={addCase}>
          <Plus className="h-3 w-3 mr-1" />
          {t.addCase}
        </Button>
      </div>
      {task.decisionCases.map((c, ci) => (
        <div key={ci} className="border rounded-lg p-3 bg-muted space-y-2">
          <div className="flex items-center gap-2">
            <Input
              className="h-7 text-xs font-medium flex-1"
              value={c.caseName}
              onChange={(e) => {
                const newCases = [...task.decisionCases];
                newCases[ci] = { ...newCases[ci], caseName: e.target.value };
                onUpdate({ decisionCases: newCases });
              }}
              placeholder={t.caseValue}
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
                  placeholder={t.referenceName}
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
                  placeholder={t.taskName}
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
            {t.addTask}
          </Button>
        </div>
      ))}
    </div>
  );
}

export function DynamicForkJoinFields({ task, onUpdate }: TypeFieldProps) {
  return (
    <div className="border-t pt-3 space-y-3">
      <p className="text-xs font-semibold text-muted-foreground">Dynamic Fork/Join Configuration</p>
      <p className="text-[10px] text-muted-foreground">
        Dynamically forks multiple tasks at runtime based on input parameters. Provide either a
        single param with task definitions or separate task list and input map params.
      </p>

      <Separator />
      <p className="text-xs font-medium">Format 1: Combined tasks param</p>
      <div className="space-y-2">
        <Label>Dynamic Fork/Join Tasks Param</Label>
        <Input
          value={task.dynamicForkJoinTasksParam}
          onChange={(e) => onUpdate({ dynamicForkJoinTasksParam: e.target.value })}
          placeholder="dynamicTasks"
        />
        <p className="text-[10px] text-muted-foreground">
          Name of the input parameter containing an array of {'{'} taskReferenceName, type, name, input {'}'} objects.
        </p>
      </div>

      <Separator />
      <p className="text-xs font-medium">Format 2: Separate task defs + inputs</p>
      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label>Dynamic Fork Tasks Param</Label>
          <Input
            value={task.dynamicForkTasksParam}
            onChange={(e) => onUpdate({ dynamicForkTasksParam: e.target.value })}
            placeholder="forkedTasks"
          />
        </div>
        <div className="space-y-2">
          <Label>Tasks Input Param Name</Label>
          <Input
            value={task.dynamicForkTasksInputParamName}
            onChange={(e) => onUpdate({ dynamicForkTasksInputParamName: e.target.value })}
            placeholder="forkedTasksInputs"
          />
        </div>
      </div>
      <p className="text-[10px] text-muted-foreground">
        One param holds an array of WorkflowTask definitions, the other holds a map of
        {'{'} taskReferenceName: inputObject {'}'}.
      </p>
    </div>
  );
}
