import { useCallback, useEffect, useRef } from "react";
import { buildInputFromFields } from "@/lib/field-parser";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Code, FormInput } from "lucide-react";
import { useThemeText } from "@/components/ThemeContext";
import { JsonEditor, buildJsonTemplate } from "@/components/JsonEditor";

interface WorkflowInputEditorProps {
  params: (string | unknown)[];
  fieldValues: Record<string, string>;
  setFieldValues: React.Dispatch<React.SetStateAction<Record<string, string>>>;
  inputJson: string;
  setInputJson: (v: string) => void;
  inputMode: "fields" | "json";
  setInputMode: (m: "fields" | "json") => void;
  disabled: boolean;
}

export function WorkflowInputEditor({
  params,
  fieldValues,
  setFieldValues,
  inputJson,
  setInputJson,
  inputMode,
  setInputMode,
  disabled,
}: WorkflowInputEditorProps) {
  const t = useThemeText();
  const hasParams = params.length > 0;
  const paramKey = params.filter((p): p is string => typeof p === "string").join(",");

  // Auto-prefill the JSON editor with a template derived from the workflow's
  // declared input parameters whenever the parameter list changes — but only
  // if the user hasn't typed anything beyond the empty `{}` default. This
  // means switching workflows shows `{ "userId": "", "amount": "" }` instead
  // of forcing the user to retype every key.
  const lastParamKeyRef = useRef<string>("");
  useEffect(() => {
    if (paramKey === lastParamKeyRef.current) return;
    lastParamKeyRef.current = paramKey;
    const trimmed = inputJson.trim();
    const isPristine = trimmed === "" || trimmed === "{}";
    if (hasParams && isPristine) {
      setInputJson(buildJsonTemplate(params));
    }
    // We deliberately exclude inputJson/setInputJson from deps to avoid
    // rebuilding the template on every keystroke.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paramKey, hasParams]);

  const syncFieldsToJson = useCallback(() => {
    // Merge field values onto the parameter template so empty fields still
    // appear as keys in the JSON output (easier to fill in directly).
    const built = buildInputFromFields(fieldValues);
    const template: Record<string, unknown> = {};
    for (const p of params) {
      if (typeof p === "string") template[p] = built[p] ?? "";
    }
    // Include any extra fields the user may have entered that aren't in params
    for (const [k, v] of Object.entries(built)) {
      if (!(k in template)) template[k] = v;
    }
    setInputJson(JSON.stringify(template, null, 2));
  }, [fieldValues, params, setInputJson]);

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
  }, [inputJson, params, setFieldValues]);

  const toggleMode = () => {
    if (inputMode === "fields") {
      syncFieldsToJson();
      setInputMode("json");
    } else {
      syncJsonToFields();
      setInputMode("fields");
    }
  };

  return (
    <div className={cn("space-y-2", disabled && "opacity-50")}>
      <div className="flex items-center justify-between">
        <Label>{t.inputLabel}</Label>
        {hasParams && !disabled && (
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
        <div className="space-y-3 rounded-lg border p-3 max-h-[40vh] overflow-y-auto">
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
                  disabled={disabled}
                />
              </div>
            );
          })}
          <p className="text-[10px] text-muted-foreground">
            {t.autoParseHint}
          </p>
        </div>
      ) : (
        <JsonEditor
          value={inputJson}
          onChange={setInputJson}
          rows={10}
          placeholder="{}"
          disabled={disabled}
          ariaLabel={t.inputLabel}
        />
      )}
    </div>
  );
}
