import { useState, useEffect, useCallback } from "react";
import { useQuery, useMutation } from "@tanstack/react-query";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { metadataApi, workflowApi, type WorkflowDef } from "@/api/conductor";
import { cn } from "@/lib/utils";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog";
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
import { SearchableSelect } from "@/components/SearchableSelect";
import { Code, FormInput } from "lucide-react";
import { useThemeText } from "@/components/ThemeContext";

/** Try to parse a string value into a typed value (number, boolean, object/array, or string) */
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
    try {
      return JSON.parse(trimmed);
    } catch {
      // fall through to string
    }
  }
  return raw;
}

/** Build an input object from field values, parsing each to its inferred type */
function buildInputFromFields(fields: Record<string, string>): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  for (const [key, val] of Object.entries(fields)) {
    result[key] = parseFieldValue(val);
  }
  return result;
}

interface StartWorkflowDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Pre-select a specific workflow definition */
  preselectedDef?: WorkflowDef;
}

export function StartWorkflowDialog({
  open,
  onOpenChange,
  preselectedDef,
}: StartWorkflowDialogProps) {
  const navigate = useNavigate();
  const t = useThemeText();

  const { data: defs, isLoading: defsLoading } = useQuery({
    queryKey: ["workflow-defs"],
    queryFn: metadataApi.listWorkflowDefs,
    enabled: open,
  });

  const [selectedName, setSelectedName] = useState(preselectedDef?.name ?? "");
  const [selectedVersion, setSelectedVersion] = useState(
    preselectedDef?.version?.toString() ?? ""
  );
  const [inputJson, setInputJson] = useState("{}");
  const [fieldValues, setFieldValues] = useState<Record<string, string>>({});
  const [inputMode, setInputMode] = useState<"fields" | "json">("fields");
  const [correlationId, setCorrelationId] = useState("");

  // Sync when preselectedDef changes (e.g. play button on a different workflow)
  useEffect(() => {
    if (preselectedDef) {
      setSelectedName(preselectedDef.name);
      setSelectedVersion(preselectedDef.version.toString());
    }
  }, [preselectedDef]);

  const selectedDef = defs?.find(
    (d) => d.name === selectedName && d.version === Number(selectedVersion)
  );

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

  // Sync between modes
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
    } catch {
      // invalid JSON — don't sync
    }
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

  // Reset when preselected changes
  const handleOpenChange = (open: boolean) => {
    if (open && preselectedDef) {
      setSelectedName(preselectedDef.name);
      setSelectedVersion(preselectedDef.version.toString());
    }
    if (!open) {
      if (!preselectedDef) {
        setSelectedName("");
        setSelectedVersion("");
      }
      setInputJson("{}");
      setFieldValues({});
      setInputMode("fields");
      setCorrelationId("");
    }
    onOpenChange(open);
  };

  const uniqueNames = defs
    ? [...new Set(defs.map((d) => d.name))].sort()
    : preselectedDef
      ? [preselectedDef.name]
      : [];

  const versionsForName = defs
    ? defs.filter((d) => d.name === selectedName).map((d) => d.version).sort((a, b) => b - a)
    : preselectedDef && selectedName === preselectedDef.name
      ? [preselectedDef.version]
      : [];

  const getInputObject = (): Record<string, unknown> => {
    if (inputMode === "fields" && params.length > 0) {
      return buildInputFromFields(fieldValues);
    }
    return JSON.parse(inputJson);
  };

  const startMut = useMutation({
    mutationFn: () => {
      const input = getInputObject();
      return workflowApi.start({
        name: selectedName,
        version: Number(selectedVersion) || undefined,
        input,
        correlationId: correlationId || undefined,
      });
    },
    onSuccess: (workflowId) => {
      toast.success(t.toastWorkflowStarted);
      handleOpenChange(false);
      navigate(`/executions/${workflowId}`);
    },
    onError: (e) => toast.error(e.message),
  });

  const handleStart = () => {
    try {
      getInputObject();
    } catch {
      toast.error(t.toastInvalidJson);
      return;
    }
    startMut.mutate();
  };

  const hasParams = params.length > 0;

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t.startWorkflowTitle}</DialogTitle>
          <DialogDescription>
            {t.startWorkflowDesc}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>{t.workflowLabel}</Label>
              {preselectedDef ? (
                <Input value={preselectedDef.name} disabled />
              ) : (
                <SearchableSelect
                  options={uniqueNames.map((name) => ({ label: name, value: name }))}
                  value={selectedName}
                  onChange={(v) => {
                    setSelectedName(v);
                    const versions = defs
                      ?.filter((d) => d.name === v)
                      .map((d) => d.version)
                      .sort((a, b) => b - a);
                    setSelectedVersion(versions?.[0]?.toString() ?? "");
                  }}
                  placeholder={t.selectWorkflow}
                  searchPlaceholder={t.searchWorkflows ?? "Search workflows..."}
                />
              )}
            </div>
            <div className="space-y-2">
              <Label>{t.versionLabel}</Label>
              {preselectedDef ? (
                <Input value={`v${preselectedDef.version}`} disabled />
              ) : (
                <Select value={selectedVersion} onValueChange={setSelectedVersion} disabled={!selectedName}>
                  <SelectTrigger>
                    <SelectValue placeholder={t.versionOptional} />
                  </SelectTrigger>
                  <SelectContent>
                    {versionsForName.map((v) => (
                      <SelectItem key={v} value={v.toString()}>
                        v{v}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
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
              <Label>{t.inputLabel}</Label>
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
                rows={8}
                className="font-mono text-xs"
                value={inputJson}
                onChange={(e) => setInputJson(e.target.value)}
                placeholder="{}"
                disabled={!selectedName}
              />
            )}
          </div>

          <div className="space-y-2">
            <Label>{t.correlationId}</Label>
            <Input
              value={correlationId}
              onChange={(e) => setCorrelationId(e.target.value)}
              placeholder="my-correlation-id"
              disabled={!selectedName}
            />
          </div>

          <div className="flex justify-end gap-2">
            <Button variant="outline" onClick={() => handleOpenChange(false)}>
              {t.cancel}
            </Button>
            <Button
              onClick={handleStart}
              disabled={!selectedName || startMut.isPending}
            >
              {startMut.isPending ? t.startingWorkflow : t.startWorkflow}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
