import { useState, useEffect, useCallback } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { scheduleApi, metadataApi, type ScheduledWorkflow } from "@/api/conductor";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Textarea } from "@/components/ui/textarea";
import { SearchableSelect } from "@/components/SearchableSelect";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { RelativeTime } from "@/components/RelativeTime";
import { Plus, Trash2, Calendar, Code, FormInput, AlertTriangle } from "lucide-react";
import { cn } from "@/lib/utils";
import { useThemeText } from "@/components/ThemeContext";

export default function Schedules() {
  const t = useThemeText();
  const queryClient = useQueryClient();
  const [createOpen, setCreateOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);

  const { data: schedules, isLoading } = useQuery({
    queryKey: ["schedules"],
    queryFn: scheduleApi.list,
  });

  const { data: workflowDefs } = useQuery({
    queryKey: ["workflow-defs-list"],
    queryFn: () => metadataApi.listWorkflowDefs(),
  });

  const enableMut = useMutation({
    mutationFn: (id: string) => scheduleApi.enable(id),
    onSuccess: () => {
      toast.success(t.scheduleEnabled);
      queryClient.invalidateQueries({ queryKey: ["schedules"] });
    },
  });

  const disableMut = useMutation({
    mutationFn: (id: string) => scheduleApi.disable(id),
    onSuccess: () => {
      toast.success(t.scheduleDisabled);
      queryClient.invalidateQueries({ queryKey: ["schedules"] });
    },
  });

  const deleteMut = useMutation({
    mutationFn: (id: string) => scheduleApi.delete(id),
    onSuccess: () => {
      toast.success(t.scheduleDeleted);
      queryClient.invalidateQueries({ queryKey: ["schedules"] });
    },
  });

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold tracking-tight">{t.schedulesTitle}</h2>
        <Button size="sm" onClick={() => setCreateOpen(true)}>
          <Plus className="h-4 w-4 mr-1" />
          {t.createSchedule}
        </Button>
      </div>

      <Card>
        <CardContent className="pt-6">
          {isLoading ? (
            <p className="text-muted-foreground text-sm">{t.loading}</p>
          ) : !schedules?.length ? (
            <div className="text-center py-8 text-muted-foreground">
              <Calendar className="h-8 w-8 mx-auto mb-2 opacity-50" />
              <p className="text-sm">{t.noSchedules}</p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{t.name}</TableHead>
                  <TableHead>{t.cronExpression}</TableHead>
                  <TableHead>{t.timezoneLabel}</TableHead>
                  <TableHead>{t.workflow}</TableHead>
                  <TableHead>{t.status}</TableHead>
                  <TableHead>{t.lastRun}</TableHead>
                  <TableHead>{t.nextRun}</TableHead>
                  <TableHead>{t.scheduleLastError}</TableHead>
                  <TableHead className="text-right">{t.actions}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {schedules.map((s) => (
                  <TableRow key={s.scheduleId}>
                    <TableCell className="font-medium">{s.name}</TableCell>
                    <TableCell className="font-mono text-xs">{s.cronExpression}</TableCell>
                    <TableCell className="text-xs">{s.timezone}</TableCell>
                    <TableCell>
                      <span className="text-sm">{s.workflowName}</span>
                      <span className="text-xs text-muted-foreground ml-1">v{s.workflowVersion}</span>
                    </TableCell>
                    <TableCell>
                      <Badge
                        variant={s.enabled ? "default" : "secondary"}
                        className="cursor-pointer"
                        onClick={() => s.enabled ? disableMut.mutate(s.scheduleId) : enableMut.mutate(s.scheduleId)}
                      >
                        {s.enabled ? t.enabled : t.disabled}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-xs"><RelativeTime value={s.lastRunAt} /></TableCell>
                    <TableCell className="text-xs"><RelativeTime value={s.nextRunAt} /></TableCell>
                    <TableCell className="text-xs max-w-[200px]">
                      {s.lastError ? (
                        <span className="flex items-center gap-1 text-destructive" title={s.lastError}>
                          <AlertTriangle className="h-3 w-3 flex-shrink-0" />
                          <span className="truncate">{s.lastError}</span>
                        </span>
                      ) : (
                        <span className="text-muted-foreground">—</span>
                      )}
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => setDeleteTarget(s.scheduleId)}
                        title={t.deleteSchedule}
                      >
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      <CreateScheduleDialog
        open={createOpen}
        onOpenChange={setCreateOpen}
        workflowDefs={workflowDefs ?? []}
      />

      <ConfirmDialog
        open={!!deleteTarget}
        onOpenChange={(open) => { if (!open) setDeleteTarget(null); }}
        title={t.deleteSchedule}
        description={t.bulkConfirm}
        confirmLabel={t.deleteSchedule}
        variant="destructive"
        onConfirm={async () => {
          if (deleteTarget) await deleteMut.mutateAsync(deleteTarget);
        }}
      />
    </div>
  );
}

function parseFieldValue(raw: string): unknown {
  const trimmed = raw.trim();
  if (trimmed === "") return "";
  if (trimmed === "true") return true;
  if (trimmed === "false") return false;
  if (trimmed === "null") return null;
  const num = Number(trimmed);
  if (!isNaN(num) && trimmed !== "") return num;
  if (
    (trimmed.startsWith("{") && trimmed.endsWith("}")) ||
    (trimmed.startsWith("[") && trimmed.endsWith("]"))
  ) {
    try { return JSON.parse(trimmed); } catch { /* fall through */ }
  }
  return raw;
}

function buildInputFromFields(fields: Record<string, string>): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  for (const [key, val] of Object.entries(fields)) {
    result[key] = parseFieldValue(val);
  }
  return result;
}

function CreateScheduleDialog({
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

  const selectedDef = workflowDefs.find(
    (d) => d.name === selectedName && d.version === Number(selectedVersion)
  );

  const versionsForName = workflowDefs
    .filter((d) => d.name === selectedName)
    .map((d) => d.version)
    .sort((a, b) => b - a);

  const params = selectedDef?.inputParameters ?? [];

  // Sync field values when workflow selection changes
  useEffect(() => {
    const newFields: Record<string, string> = {};
    for (const p of params) {
      if (typeof p === "string") newFields[p] = fieldValues[p] ?? "";
    }
    setFieldValues(newFields);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedName, selectedVersion, params.join(",")]);

  const syncFieldsToJson = useCallback(() => {
    setInputJson(JSON.stringify(buildInputFromFields(fieldValues), null, 2));
  }, [fieldValues]);

  const syncJsonToFields = useCallback(() => {
    try {
      const parsed = JSON.parse(inputJson);
      if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
        const newFields: Record<string, string> = {};
        for (const p of params) {
          if (typeof p === "string") {
            const val = parsed[p];
            newFields[p] =
              val === undefined || val === null
                ? ""
                : typeof val === "object"
                  ? JSON.stringify(val)
                  : String(val);
          }
        }
        setFieldValues(newFields);
      }
    } catch { /* invalid JSON */ }
  }, [inputJson, params]);

  const toggleMode = () => {
    if (inputMode === "fields") {
      syncFieldsToJson();
      setInputMode("json");
    } else {
      syncJsonToFields();
      setInputMode("fields");
    }
  };

  const getInputObject = (): unknown => {
    if (inputMode === "fields" && params.length > 0) {
      return buildInputFromFields(fieldValues);
    }
    return JSON.parse(inputJson);
  };

  const createMut = useMutation({
    mutationFn: (s: Omit<ScheduledWorkflow, "scheduleId" | "lastRunAt" | "nextRunAt">) =>
      scheduleApi.create(s),
    onSuccess: () => {
      toast.success(t.scheduleCreated);
      queryClient.invalidateQueries({ queryKey: ["schedules"] });
      onOpenChange(false);
      resetForm();
    },
  });

  const resetForm = () => {
    setName("");
    setCronExpression("");
    setTimezone("UTC");
    setSelectedName("");
    setSelectedVersion("");
    setInputJson("{}");
    setFieldValues({});
    setInputMode("fields");
  };

  const handleSubmit = () => {
    if (!name.trim() || !cronExpression.trim() || !selectedName.trim()) return;
    let parsedInput: unknown = {};
    try {
      parsedInput = getInputObject();
    } catch {
      toast.error(t.toastInvalidJson);
      return;
    }
    createMut.mutate({
      name: name.trim(),
      cronExpression: cronExpression.trim(),
      timezone,
      workflowName: selectedName.trim(),
      workflowVersion: Number(selectedVersion) || 1,
      workflowInput: parsedInput,
      enabled: true,
    });
  };

  const hasParams = params.length > 0;

  return (
    <Dialog open={open} onOpenChange={(v) => { if (!v) resetForm(); onOpenChange(v); }}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t.createSchedule}</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-2">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>{t.name}</Label>
              <Input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="daily-report"
              />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <div className="space-y-2">
                <Label>{t.cronExpression}</Label>
                <Input
                  value={cronExpression}
                  onChange={(e) => setCronExpression(e.target.value)}
                  placeholder="0 0 * * *"
                  className="font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label>{t.timezoneLabel}</Label>
                <Input
                  value={timezone}
                  onChange={(e) => setTimezone(e.target.value)}
                  placeholder="UTC"
                />
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
                  const versions = workflowDefs
                    .filter((d) => d.name === v)
                    .map((d) => d.version)
                    .sort((a, b) => b - a);
                  setSelectedVersion(versions[0]?.toString() ?? "");
                }}
                placeholder={t.searchPlaceholder}
                searchPlaceholder={t.searchPlaceholder}
              />
            </div>
            <div className="space-y-2">
              <Label>{t.versionLabel}</Label>
              <Select value={selectedVersion} onValueChange={setSelectedVersion} disabled={!selectedName}>
                <SelectTrigger>
                  <SelectValue placeholder={t.versionLabel} />
                </SelectTrigger>
                <SelectContent>
                  {versionsForName.map((v) => (
                    <SelectItem key={v} value={v.toString()}>
                      v{v}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          {selectedDef?.description && (
            <p className="text-sm text-muted-foreground bg-muted rounded-lg px-3 py-2">
              {selectedDef.description}
            </p>
          )}

          {/* Input section with mode toggle */}
          <div className={cn("space-y-2", !selectedName && "opacity-50")}>
            <div className="flex items-center justify-between">
              <Label>{t.input}</Label>
              {hasParams && selectedName && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-7 text-xs gap-1"
                  onClick={toggleMode}
                >
                  {inputMode === "fields" ? (
                    <><Code className="h-3 w-3" /> {t.json}</>
                  ) : (
                    <><FormInput className="h-3 w-3" /> {t.fields}</>
                  )}
                </Button>
              )}
            </div>

            {inputMode === "fields" && hasParams ? (
              <div className="space-y-3 rounded-lg border p-3">
                {params.map((p) => {
                  if (typeof p !== "string") return null;
                  return (
                    <div key={p} className="space-y-1">
                      <Label className="text-xs font-mono">{p}</Label>
                      <Input
                        value={fieldValues[p] ?? ""}
                        onChange={(e) =>
                          setFieldValues((prev) => ({ ...prev, [p]: e.target.value }))
                        }
                        placeholder={`${t.valueFor} ${p}`}
                        className="font-mono text-xs"
                        disabled={!selectedName}
                      />
                    </div>
                  );
                })}
                <p className="text-[10px] text-muted-foreground">
                  {t.autoParseHint}
                </p>
              </div>
            ) : (
              <Textarea
                rows={6}
                className="font-mono text-xs"
                value={inputJson}
                onChange={(e) => setInputJson(e.target.value)}
                placeholder="{}"
                disabled={!selectedName}
              />
            )}
          </div>
        </div>
        <div className="flex justify-end gap-2">
          <Button variant="outline" onClick={() => { resetForm(); onOpenChange(false); }}>{t.cancel}</Button>
          <Button
            onClick={handleSubmit}
            disabled={createMut.isPending || !name.trim() || !cronExpression.trim() || !selectedName.trim()}
          >
            {createMut.isPending ? t.creating : t.createSchedule}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
