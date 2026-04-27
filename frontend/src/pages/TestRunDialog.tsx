import { useEffect, useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { metadataApi, workflowApi, type TaskDef } from "@/api/conductor";
import { buildInputFromFields } from "@/lib/field-parser";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Code, FormInput } from "lucide-react";
import { useThemeText } from "@/components/ThemeContext";
import { JsonEditor, buildJsonTemplate } from "@/components/JsonEditor";

interface TestRunDialogProps {
  def: TaskDef | null;
  onClose: () => void;
}

export function TestRunDialog({ def, onClose }: TestRunDialogProps) {
  const navigate = useNavigate();
  const t = useThemeText();
  const [testInput, setTestInput] = useState("{}");
  const [fieldValues, setFieldValues] = useState<Record<string, string>>({});
  const [inputMode, setInputMode] = useState<"fields" | "json">(
    (def?.inputKeys?.length ?? 0) > 0 ? "fields" : "json"
  );

  // Pre-fill JSON template with the task's declared inputKeys whenever the
  // selected task changes — saves the user from typing the schema by hand.
  const lastDefRef = useRef<string>("");
  useEffect(() => {
    const defKey = def ? `${def.name}::${(def.inputKeys ?? []).join(",")}` : "";
    if (defKey === lastDefRef.current) return;
    lastDefRef.current = defKey;
    const keys = def?.inputKeys ?? [];
    const trimmed = testInput.trim();
    if (keys.length > 0 && (trimmed === "" || trimmed === "{}")) {
      setTestInput(buildJsonTemplate(keys));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [def?.name]);

  const testRunMut = useMutation({
    mutationFn: async ({ taskName, input }: { taskName: string; input: Record<string, unknown> }) => {
      const wfName = "__test_run_task";

      let needsRegister = false;
      try {
        await metadataApi.getWorkflowDef(wfName, 1);
      } catch {
        needsRegister = true;
      }

      if (needsRegister) {
        await metadataApi.registerWorkflowDef({
          name: wfName,
          version: 1,
          description: "Dynamic test-run wrapper — runs any task type via DYNAMIC dispatch",
          tasks: [
            {
              name: "__dynamic_test",
              taskReferenceName: "dynamic_test",
              type: "DYNAMIC",
              dynamicTaskNameParam: "taskToExecute",
              inputParameters: {
                taskToExecute: "${workflow.input.taskToExecute}",
              },
            },
          ],
        });
      }

      return workflowApi.start({
        name: wfName,
        version: 1,
        input: { taskToExecute: taskName, ...input },
      });
    },
    onSuccess: (workflowId) => {
      toast.success(t.toastWorkflowStarted);
      onClose();
      navigate(`/executions/${workflowId}`);
    },
    onError: (e) => toast.error(e.message),
  });

  const handleSubmit = () => {
    if (!def) return;
    try {
      const input =
        inputMode === "fields" && (def.inputKeys?.length ?? 0) > 0
          ? buildInputFromFields(fieldValues)
          : JSON.parse(testInput);
      testRunMut.mutate({ taskName: def.name, input });
    } catch {
      toast.error(t.toastInvalidJson);
    }
  };

  const handleOpenChange = (open: boolean) => {
    if (!open) {
      onClose();
      setTestInput("{}");
      setFieldValues({});
    }
  };

  return (
    <Dialog open={!!def} onOpenChange={handleOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t.testRun}: {def?.name}</DialogTitle>
        </DialogHeader>
        <p className="text-sm text-muted-foreground">
          {t.testRunDescription}
        </p>

        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <Label className="text-sm font-medium">{t.taskInput}</Label>
            {(def?.inputKeys?.length ?? 0) > 0 && (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-7 text-xs gap-1"
                onClick={() => {
                  if (inputMode === "fields") {
                    const built = buildInputFromFields(fieldValues);
                    const template: Record<string, unknown> = {};
                    for (const k of def?.inputKeys ?? []) template[k] = built[k] ?? "";
                    for (const [k, v] of Object.entries(built)) if (!(k in template)) template[k] = v;
                    setTestInput(JSON.stringify(template, null, 2));
                    setInputMode("json");
                  } else {
                    try {
                      const parsed = JSON.parse(testInput);
                      if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
                        const fields: Record<string, string> = {};
                        for (const k of def?.inputKeys ?? []) {
                          const val = parsed[k];
                          fields[k] = val === undefined || val === null ? "" : typeof val === "object" ? JSON.stringify(val) : String(val);
                        }
                        setFieldValues(fields);
                      }
                    } catch { /* ignore */ }
                    setInputMode("fields");
                  }
                }}
              >
                {inputMode === "fields" ? (
                  <><Code className="h-3 w-3" /> {t.json}</>
                ) : (
                  <><FormInput className="h-3 w-3" /> {t.fields}</>
                )}
              </Button>
            )}
          </div>

          {inputMode === "fields" && (def?.inputKeys?.length ?? 0) > 0 ? (
            <div className="space-y-3 rounded-lg border p-3 max-h-[40vh] overflow-auto">
              {(def?.inputKeys ?? []).map((k) => (
                <div key={k} className="space-y-1">
                  <Label className="text-xs font-mono">{k}</Label>
                  <Input
                    value={fieldValues[k] ?? ""}
                    onChange={(e) =>
                      setFieldValues((prev) => ({ ...prev, [k]: e.target.value }))
                    }
                    placeholder={`${t.valueFor} ${k}`}
                    className="font-mono text-xs"
                  />
                </div>
              ))}
              <p className="text-[10px] text-muted-foreground">
                {t.autoParseHint}
              </p>
            </div>
          ) : (
            <JsonEditor
              value={testInput}
              onChange={setTestInput}
              rows={10}
              placeholder="{}"
              ariaLabel={t.taskInput}
            />
          )}
        </div>

        <Button
          onClick={handleSubmit}
          disabled={testRunMut.isPending}
        >
          {testRunMut.isPending ? t.startingWorkflow : t.runTest}
        </Button>
      </DialogContent>
    </Dialog>
  );
}
