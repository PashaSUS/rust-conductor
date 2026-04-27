import { useState, useEffect, useCallback, useRef } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { scheduleApi, type ScheduledWorkflow } from "@/api/conductor";
import { buildInputFromFields } from "@/lib/field-parser";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { JsonEditor, buildJsonTemplate } from "@/components/JsonEditor";
import { SearchableSelect } from "@/components/SearchableSelect";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Code, FormInput } from "lucide-react";
import { cn } from "@/lib/utils";
import { useThemeText } from "@/components/ThemeContext";

export function CreateScheduleDialog({
  open,
  onOpenChange,
  workflowDefs,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workflowDefs: { name: string; version: number; inputParameters?: string[]; description?: string }[];
}) {
  const t = useThemeText();
  const queryClient = useQueryClient();
  const [name, setName] = useState("");
  const [cronExpression, setCronExpression] = useState("");
  const [timezone, setTimezone] = useState("UTC");
  const [selectedName, setSelectedName] = useState("");
  const [selectedVersion, setSelectedVersion] = useState("");
  const [inputJson, setInputJson] = useState("{}");
  const [fieldValues, setFieldValues] = useState<Record<string, string>>({});
  const [inputMode, setInputMode] = useState<"fields" | "json">("fields");

  const uniqueNames = [...new Set(workflowDefs.map((d) => d.name))].sort();
  const selectedDef = workflowDefs.find((d) => d.name === selectedName && d.version === Number(selectedVersion));
  const versionsForName = workflowDefs.filter((d) => d.name === selectedName).map((d) => d.version).sort((a, b) => b - a);
  const params = selectedDef?.inputParameters ?? [];

  useEffect(() => {
    const newFields: Record<string, string> = {};
    for (const p of params) { if (typeof p === "string") newFields[p] = fieldValues[p] ?? ""; }
    setFieldValues(newFields);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedName, selectedVersion, params.join(",")]);

  // Pre-fill the JSON editor with a template containing the workflow's
  // declared input parameters when the workflow selection changes (only if
  // the JSON area hasn't been edited beyond the empty `{}` default).
  const lastParamKeyRef = useRef<string>("");
  const paramKey = params.filter((p): p is string => typeof p === "string").join(",");
  useEffect(() => {
    if (paramKey === lastParamKeyRef.current) return;
    lastParamKeyRef.current = paramKey;
    const trimmed = inputJson.trim();
    if (params.length > 0 && (trimmed === "" || trimmed === "{}")) {
      setInputJson(buildJsonTemplate(params));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paramKey]);

  const syncFieldsToJson = useCallback(() => {
    const built = buildInputFromFields(fieldValues);
    const template: Record<string, unknown> = {};
    for (const p of params) { if (typeof p === "string") template[p] = built[p] ?? ""; }
    for (const [k, v] of Object.entries(built)) { if (!(k in template)) template[k] = v; }
    setInputJson(JSON.stringify(template, null, 2));
  }, [fieldValues, params]);

  const syncJsonToFields = useCallback(() => {
    try {
      const parsed = JSON.parse(inputJson);
      if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
        const newFields: Record<string, string> = {};
        for (const p of params) {
          if (typeof p === "string") {
            const val = parsed[p];
            newFields[p] = val === undefined || val === null ? "" : typeof val === "object" ? JSON.stringify(val) : String(val);
          }
        }
        setFieldValues(newFields);
      }
    } catch { /* invalid JSON */ }
  }, [inputJson, params]);

  const toggleMode = () => {
    if (inputMode === "fields") { syncFieldsToJson(); setInputMode("json"); }
    else { syncJsonToFields(); setInputMode("fields"); }
  };

  const getInputObject = (): unknown => {
    if (inputMode === "fields" && params.length > 0) return buildInputFromFields(fieldValues);
    return JSON.parse(inputJson);
  };

  const createMut = useMutation({
    mutationFn: (s: Omit<ScheduledWorkflow, "scheduleId" | "lastRunAt" | "nextRunAt">) => scheduleApi.create(s),
    onSuccess: () => {
      toast.success(t.scheduleCreated);
      queryClient.invalidateQueries({ queryKey: ["schedules"] });
      onOpenChange(false);
      resetForm();
    },
  });

  const resetForm = () => {
    setName(""); setCronExpression(""); setTimezone("UTC");
    setSelectedName(""); setSelectedVersion(""); setInputJson("{}");
    setFieldValues({}); setInputMode("fields");
  };

  const handleSubmit = () => {
    if (!name.trim() || !cronExpression.trim() || !selectedName.trim()) return;
    let parsedInput: unknown = {};
    try { parsedInput = getInputObject(); } catch { toast.error(t.toastInvalidJson); return; }
    createMut.mutate({
      name: name.trim(), cronExpression: cronExpression.trim(), timezone,
      workflowName: selectedName.trim(), workflowVersion: Number(selectedVersion) || 1,
      workflowInput: parsedInput, enabled: true,
    });
  };

  const hasParams = params.length > 0;

  return (
    <Dialog open={open} onOpenChange={(v) => { if (!v) resetForm(); onOpenChange(v); }}>
      <DialogContent className="max-w-2xl">
        <DialogHeader><DialogTitle>{t.createSchedule}</DialogTitle></DialogHeader>
        <div className="space-y-4 py-2">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>{t.name}</Label>
              <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="daily-report" />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <div className="space-y-2">
                <Label>{t.cronExpression}</Label>
                <Input value={cronExpression} onChange={(e) => setCronExpression(e.target.value)} placeholder="0 0 * * *" className="font-mono" />
              </div>
              <div className="space-y-2">
                <Label>{t.timezoneLabel}</Label>
                <Input value={timezone} onChange={(e) => setTimezone(e.target.value)} placeholder="UTC" />
              </div>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>{t.workflow}</Label>
              <SearchableSelect
                options={uniqueNames.map((n) => ({ label: n, value: n }))}
                value={selectedName}
                onChange={(v) => {
                  setSelectedName(v);
                  const versions = workflowDefs.filter((d) => d.name === v).map((d) => d.version).sort((a, b) => b - a);
                  setSelectedVersion(versions[0]?.toString() ?? "");
                }}
                placeholder={t.searchPlaceholder}
                searchPlaceholder={t.searchPlaceholder}
              />
            </div>
            <div className="space-y-2">
              <Label>{t.versionLabel}</Label>
              <Select value={selectedVersion} onValueChange={setSelectedVersion} disabled={!selectedName}>
                <SelectTrigger><SelectValue placeholder={t.versionLabel} /></SelectTrigger>
                <SelectContent>{versionsForName.map((v) => <SelectItem key={v} value={v.toString()}>v{v}</SelectItem>)}</SelectContent>
              </Select>
            </div>
          </div>

          {selectedDef?.description && <p className="text-sm text-muted-foreground bg-muted rounded-lg px-3 py-2">{selectedDef.description}</p>}

          <div className={cn("space-y-2", !selectedName && "opacity-50")}>
            <div className="flex items-center justify-between">
              <Label>{t.input}</Label>
              {hasParams && selectedName && (
                <Button type="button" variant="ghost" size="sm" className="h-7 text-xs gap-1" onClick={toggleMode}>
                  {inputMode === "fields" ? <><Code className="h-3 w-3" /> {t.json}</> : <><FormInput className="h-3 w-3" /> {t.fields}</>}
                </Button>
              )}
            </div>
            {inputMode === "fields" && hasParams ? (
              <div className="space-y-3 rounded-lg border p-3 max-h-[40vh] overflow-y-auto">
                {params.map((p) => {
                  if (typeof p !== "string") return null;
                  return (
                    <div key={p} className="space-y-1">
                      <Label className="text-xs font-mono">{p}</Label>
                      <Input value={fieldValues[p] ?? ""} onChange={(e) => setFieldValues((prev) => ({ ...prev, [p]: e.target.value }))} placeholder={`${t.valueFor} ${p}`} className="font-mono text-xs" disabled={!selectedName} />
                    </div>
                  );
                })}
                <p className="text-[10px] text-muted-foreground">{t.autoParseHint}</p>
              </div>
            ) : (
              <JsonEditor
                value={inputJson}
                onChange={setInputJson}
                rows={8}
                placeholder="{}"
                disabled={!selectedName}
                ariaLabel={t.input}
              />
            )}
          </div>
        </div>
        <div className="flex justify-end gap-2">
          <Button variant="outline" onClick={() => { resetForm(); onOpenChange(false); }}>{t.cancel}</Button>
          <Button onClick={handleSubmit} disabled={createMut.isPending || !name.trim() || !cronExpression.trim() || !selectedName.trim()}>
            {createMut.isPending ? t.creating : t.createSchedule}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
