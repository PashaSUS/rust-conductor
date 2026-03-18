import { useState } from "react";
import type { TaskFormState, InputSource } from "./types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Code, FormInput } from "lucide-react";

interface TaskInputFieldsProps {
  task: TaskFormState;
  onUpdate: (u: Partial<TaskFormState>) => void;
  /** The locked input keys from the selected task definition */
  lockedKeys: string[];
  /** Available sources (workflow inputs + previous task outputs) */
  sources: InputSource[];
  /** Whether a registered task is selected (locks the field list) */
  hasRegisteredTask: boolean;
}

/** Group sources by their group label */
function groupSources(sources: InputSource[]): Map<string, InputSource[]> {
  const grouped = new Map<string, InputSource[]>();
  for (const s of sources) {
    const list = grouped.get(s.group) ?? [];
    list.push(s);
    grouped.set(s.group, list);
  }
  return grouped;
}

/** Find which group a value belongs to */
function findGroupForValue(sources: InputSource[], value: string): string | null {
  const src = sources.find((s) => s.value === value);
  return src?.group ?? null;
}

export function TaskInputFields({
  task,
  onUpdate,
  lockedKeys,
  sources,
  hasRegisteredTask,
}: TaskInputFieldsProps) {
  const [selectedGroups, setSelectedGroups] = useState<Record<string, string>>({});

  const toggleInputMode = () => {
    if (task.inputMode === "fields") {
      const params: Record<string, string> = {};
      for (const [key, val] of Object.entries(task.inputFields)) {
        if (val.trim()) params[key] = val;
      }
      onUpdate({
        inputParametersJson: JSON.stringify(params, null, 2),
        inputMode: "json",
      });
    } else {
      try {
        const parsed = JSON.parse(task.inputParametersJson);
        if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
          const keys = lockedKeys.length > 0 ? lockedKeys : Object.keys(parsed);
          const fields: Record<string, string> = {};
          for (const key of keys) {
            const val = parsed[key];
            fields[key] =
              val === undefined || val === null
                ? ""
                : typeof val === "object"
                  ? JSON.stringify(val)
                  : String(val);
          }
          onUpdate({ inputFields: fields, inputMode: "fields" });
        } else {
          onUpdate({ inputMode: "fields" });
        }
      } catch {
        onUpdate({ inputMode: "fields" });
      }
    }
  };

  const fieldKeys = hasRegisteredTask ? lockedKeys : Object.keys(task.inputFields);
  const grouped = groupSources(sources);
  const groupNames = [...grouped.keys()];

  const setFieldValue = (key: string, value: string) => {
    onUpdate({ inputFields: { ...task.inputFields, [key]: value } });
  };

  const getSelectedGroup = (key: string, currentValue: string): string => {
    if (selectedGroups[key]) return selectedGroups[key];
    if (currentValue) {
      const found = findGroupForValue(sources, currentValue);
      if (found) return found;
    }
    return "";
  };

  const setGroup = (key: string, group: string) => {
    setSelectedGroups((prev) => ({ ...prev, [key]: group }));
    // Clear value when changing group
    setFieldValue(key, "");
  };

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <Label>Input Parameters</Label>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-7 text-xs gap-1"
          onClick={toggleInputMode}
        >
          {task.inputMode === "fields" ? (
            <><Code className="h-3 w-3" /> JSON</>
          ) : (
            <><FormInput className="h-3 w-3" /> Fields</>
          )}
        </Button>
      </div>

      {task.inputMode === "fields" ? (
        <div className="space-y-3 rounded-lg border p-3">
          {fieldKeys.length > 0 ? (
            fieldKeys.map((key) => {
              const currentValue = task.inputFields[key] ?? "";
              const activeGroup = getSelectedGroup(key, currentValue);
              const isCustom = activeGroup === "__custom__";
              const groupItems = activeGroup && !isCustom ? (grouped.get(activeGroup) ?? []) : [];

              return (
                <div key={key} className="space-y-1.5">
                  <Label className="text-xs font-mono">{key}</Label>
                  <div className="flex gap-2">
                    {/* Step 1: pick source */}
                    <Select
                      value={isCustom ? "__custom__" : activeGroup}
                      onValueChange={(v) => {
                        if (v === "__custom__") {
                          setSelectedGroups((prev) => ({ ...prev, [key]: "__custom__" }));
                          setFieldValue(key, "");
                        } else {
                          setGroup(key, v);
                        }
                      }}
                    >
                      <SelectTrigger className="w-45 text-xs h-8 shrink-0">
                        <SelectValue placeholder="Source..." />
                      </SelectTrigger>
                      <SelectContent>
                        {groupNames.map((g) => (
                          <SelectItem key={g} value={g}>
                            <span className="text-xs">{g}</span>
                          </SelectItem>
                        ))}
                        <SelectItem value="__custom__">
                          <span className="text-xs text-muted-foreground italic">Custom value</span>
                        </SelectItem>
                      </SelectContent>
                    </Select>

                    {/* Step 2: pick param from source, or custom input */}
                    {isCustom ? (
                      <Input
                        value={currentValue}
                        onChange={(e) => setFieldValue(key, e.target.value)}
                        placeholder={`\${workflow.input.${key}}`}
                        className="flex-1 font-mono text-xs h-8"
                      />
                    ) : activeGroup && groupItems.length > 0 ? (
                      <Select
                        value={currentValue}
                        onValueChange={(v) => setFieldValue(key, v)}
                      >
                        <SelectTrigger className="flex-1 text-xs font-mono h-8 text-left">
                          <SelectValue placeholder="Select parameter..." />
                        </SelectTrigger>
                        <SelectContent>
                          {groupItems.map((s) => (
                            <SelectItem key={s.value} value={s.value}>
                              <span className="font-mono text-xs">{s.label}</span>
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    ) : (
                      <Input
                        value={currentValue}
                        onChange={(e) => setFieldValue(key, e.target.value)}
                        placeholder={`\${workflow.input.${key}}`}
                        className="flex-1 font-mono text-xs h-8"
                      />
                    )}
                  </div>
                </div>
              );
            })
          ) : (
            <p className="text-xs text-muted-foreground py-2">
              {hasRegisteredTask
                ? "This task has no defined input keys."
                : task.type === "SIMPLE"
                  ? "Select a registered task to see its input parameters, or switch to JSON mode."
                  : "Provide the needed parameters in JSON mode."}
            </p>
          )}
          {hasRegisteredTask && lockedKeys.length > 0 && (
            <p className="text-[10px] text-muted-foreground border-t pt-2">
              Pick a source first (workflow input or a previous task), then select the specific parameter.
            </p>
          )}
        </div>
      ) : (
        <Textarea
          rows={4}
          className="font-mono text-xs"
          value={task.inputParametersJson}
          onChange={(e) => onUpdate({ inputParametersJson: e.target.value })}
          placeholder='{"key": "${workflow.input.param1}"}'
        />
      )}
    </div>
  );
}
