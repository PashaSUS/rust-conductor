import { useState } from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Plus, X, GitBranch, Repeat, GitFork } from "lucide-react";
import type { DesignerTask } from "./types";
import { TASK_TYPE_COLORS } from "./constants";

export function DecisionFields({ editingTask, setEditingTask }: { editingTask: DesignerTask; setEditingTask: (t: DesignerTask) => void }) {
  const [newCase, setNewCase] = useState("");
  if (editingTask.type !== "DECISION" && editingTask.type !== "SWITCH") return null;
  const caseNames = editingTask.caseNames ?? [];
  const color = TASK_TYPE_COLORS[editingTask.type] ?? "#ec4899";

  const addCase = () => {
    const name = newCase.trim();
    if (!name || caseNames.includes(name)) return;
    setEditingTask({ ...editingTask, caseNames: [...caseNames, name] });
    setNewCase("");
  };
  const removeCase = (name: string) => {
    setEditingTask({ ...editingTask, caseNames: caseNames.filter((c) => c !== name) });
  };

  return (
    <div className="space-y-2 border-t pt-2">
      <div className="flex items-center gap-1.5">
        <GitBranch className="h-3.5 w-3.5" style={{ color }} />
        <label className="text-xs font-medium" style={{ color }}>
          {editingTask.type === "DECISION" ? "Decision" : "Switch"} Configuration
        </label>
      </div>
      <div>
        <label className="text-[10px] text-muted-foreground">
          {editingTask.type === "SWITCH" ? "Expression (JavaScript)" : "Case Expression"}
        </label>
        <Input
          value={editingTask.caseExpression ?? ""}
          onChange={(e) => setEditingTask({ ...editingTask, caseExpression: e.target.value })}
          placeholder={editingTask.type === "SWITCH" ? "$.taskRef.output.value" : "${task_ref.output.status}"}
          className="h-7 text-xs font-mono"
        />
      </div>
      {editingTask.type === "DECISION" && (
        <div>
          <label className="text-[10px] text-muted-foreground">Case Value Parameter</label>
          <Input
            value={editingTask.caseValueParam ?? ""}
            onChange={(e) => setEditingTask({ ...editingTask, caseValueParam: e.target.value })}
            placeholder="caseValue"
            className="h-7 text-xs"
          />
        </div>
      )}
      <div>
        <div className="flex items-center justify-between mb-1">
          <label className="text-[10px] text-muted-foreground">Cases</label>
          <Badge variant="secondary" className="text-[10px]">{caseNames.length}</Badge>
        </div>
        <div className="space-y-1 mb-1.5">
          {caseNames.map((name) => (
            <div key={name} className="flex items-center gap-1.5 bg-muted/50 rounded px-2 py-1">
              <span className="w-1.5 h-1.5 rounded-full shrink-0" style={{ backgroundColor: color }} />
              <span className="text-[10px] font-mono flex-1">{name}</span>
              <button onClick={() => removeCase(name)} className="text-muted-foreground hover:text-destructive p-0.5">
                <X className="h-2.5 w-2.5" />
              </button>
            </div>
          ))}
          {caseNames.length === 0 && (
            <p className="text-[9px] text-muted-foreground italic">No cases yet. Add case names to define branches.</p>
          )}
        </div>
        <div className="flex gap-1">
          <Input value={newCase} onChange={(e) => setNewCase(e.target.value)} onKeyDown={(e) => e.key === "Enter" && addCase()} placeholder="Case name (e.g. approved)" className="h-7 text-xs flex-1" />
          <Button variant="outline" size="sm" className="h-7 px-2" onClick={addCase}><Plus className="h-3 w-3" /></Button>
        </div>
      </div>
    </div>
  );
}

export function DoWhileFields({ editingTask, setEditingTask }: { editingTask: DesignerTask; setEditingTask: (t: DesignerTask) => void }) {
  if (editingTask.type !== "DO_WHILE") return null;
  const color = TASK_TYPE_COLORS.DO_WHILE ?? "#10b981";
  return (
    <div className="space-y-2 border-t pt-2">
      <div className="flex items-center gap-1.5">
        <Repeat className="h-3.5 w-3.5" style={{ color }} />
        <label className="text-xs font-medium" style={{ color }}>Do While Configuration</label>
      </div>
      <div>
        <label className="text-[10px] text-muted-foreground">Loop Condition</label>
        <Input
          value={editingTask.loopCondition ?? ""}
          onChange={(e) => setEditingTask({ ...editingTask, loopCondition: e.target.value })}
          placeholder="$.loopTask['iteration'] < $.value"
          className="h-7 text-xs font-mono"
        />
        <p className="text-[9px] text-muted-foreground mt-1">Loop continues while this evaluates to <code className="px-0.5 bg-muted rounded">true</code>.</p>
      </div>
    </div>
  );
}

export function DynamicForkFields({ editingTask, setEditingTask }: { editingTask: DesignerTask; setEditingTask: (t: DesignerTask) => void }) {
  if (editingTask.type !== "DYNAMIC_FORK_JOIN") return null;
  const color = TASK_TYPE_COLORS.DYNAMIC_FORK_JOIN ?? "#d97706";
  return (
    <div className="space-y-2 border-t pt-2">
      <div className="flex items-center gap-1.5">
        <GitFork className="h-3.5 w-3.5" style={{ color }} />
        <label className="text-xs font-medium" style={{ color }}>Dynamic Fork Config</label>
      </div>
      <p className="text-[9px] text-muted-foreground">Set the input parameter containing the array of tasks to fork.</p>
      <div>
        <label className="text-[10px] text-muted-foreground">Tasks Param Name</label>
        <Input value={(editingTask.inputParameters?.dynamicForkJoinTasksParam as string) ?? "dynamicTasks"} onChange={(e) => setEditingTask({ ...editingTask, inputParameters: { ...editingTask.inputParameters, dynamicForkJoinTasksParam: e.target.value } })} className="h-8 text-xs" placeholder="dynamicTasks" />
      </div>
    </div>
  );
}
