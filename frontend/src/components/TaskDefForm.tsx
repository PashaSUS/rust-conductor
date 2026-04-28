import { useState } from "react";
import type { TaskDef, TaskInputParameterDef } from "@/api/conductor";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { TagInput } from "@/components/TagInput";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useThemeText } from "@/components/ThemeContext";

interface TaskDefFormProps {
  onSubmit: (defs: TaskDef[]) => void;
  isPending?: boolean;
}

export function TaskDefForm({ onSubmit, isPending }: TaskDefFormProps) {
  const t = useThemeText();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [retryCount, setRetryCount] = useState(3);
  const [retryLogic, setRetryLogic] = useState("FIXED");
  const [retryDelaySeconds, setRetryDelaySeconds] = useState(60);
  const [timeoutSeconds, setTimeoutSeconds] = useState(3600);
  const [timeoutPolicy, setTimeoutPolicy] = useState("TIME_OUT_WF");
  const [responseTimeoutSeconds, setResponseTimeoutSeconds] = useState(600);
  const [concurrentExecLimit, setConcurrentExecLimit] = useState(0);
  const [ownerEmail, setOwnerEmail] = useState("");
  const [inputKeys, setInputKeys] = useState<string[]>([]);
  const [outputKeys, setOutputKeys] = useState<string[]>([]);
  /**
   * Per-key descriptions stored separately. Synced into the saved
   * `inputParameterDefinitions` / `outputParameterDefinitions` arrays.
   * Map shape keeps the UI simple — at submit time we build the array.
   */
  const [inputDescriptions, setInputDescriptions] = useState<Record<string, string>>({});
  const [outputDescriptions, setOutputDescriptions] = useState<Record<string, string>>({});
  const [retryOnErrors, setRetryOnErrors] = useState<string[]>([]);
  const [envVars, setEnvVars] = useState("");

  const handleSubmit = () => {
    if (!name.trim()) return;
    const def: TaskDef = { name: name.trim() };
    if (description) def.description = description;
    if (retryCount !== 3) def.retryCount = retryCount;
    if (retryLogic !== "FIXED") def.retryLogic = retryLogic;
    if (retryDelaySeconds !== 60) def.retryDelaySeconds = retryDelaySeconds;
    if (timeoutSeconds !== 3600) def.timeoutSeconds = timeoutSeconds;
    if (timeoutPolicy !== "TIME_OUT_WF") def.timeoutPolicy = timeoutPolicy;
    if (responseTimeoutSeconds !== 600) def.responseTimeoutSeconds = responseTimeoutSeconds;
    if (concurrentExecLimit > 0) def.concurrentExecLimit = concurrentExecLimit;
    if (ownerEmail) def.ownerEmail = ownerEmail;
    if (inputKeys.length > 0) def.inputKeys = inputKeys;
    if (outputKeys.length > 0) def.outputKeys = outputKeys;
    // Build the additive parameter-definition arrays from the per-key
    // description maps. Only keys with a non-empty description are
    // emitted to keep the saved JSON minimal.
    const inDefs: TaskInputParameterDef[] = inputKeys
      .filter((k) => (inputDescriptions[k] ?? "").trim() !== "")
      .map((k) => ({ name: k, description: inputDescriptions[k].trim() }));
    if (inDefs.length > 0) def.inputParameterDefinitions = inDefs;
    const outDefs: TaskInputParameterDef[] = outputKeys
      .filter((k) => (outputDescriptions[k] ?? "").trim() !== "")
      .map((k) => ({ name: k, description: outputDescriptions[k].trim() }));
    if (outDefs.length > 0) def.outputParameterDefinitions = outDefs;
    if (retryOnErrors.length > 0) def.retryOnErrors = retryOnErrors;
    if (envVars.trim()) {
      try {
        def.envVars = JSON.parse(envVars);
      } catch { /* skip invalid json */ }
    }
    onSubmit([def]);
  };

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label htmlFor="td-name">{t.nameRequired}</Label>
          <Input
            id="td-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="my_task"
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="td-owner">{t.ownerEmail}</Label>
          <Input
            id="td-owner"
            type="email"
            value={ownerEmail}
            onChange={(e) => setOwnerEmail(e.target.value)}
            placeholder="team@example.com"
          />
        </div>
      </div>

      <div className="space-y-2">
          <Label htmlFor="td-desc">{t.description}</Label>
        <Textarea
          id="td-desc"
          rows={2}
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder={t.taskDescriptionHint}
        />
      </div>

      <div className="grid grid-cols-3 gap-4">
        <div className="space-y-2">
          <Label>{t.retryCount}</Label>
          <Input
            type="number"
            min={0}
            value={retryCount}
            onChange={(e) => setRetryCount(Number(e.target.value) || 0)}
          />
        </div>
        <div className="space-y-2">
          <Label>{t.retryLogic}</Label>
          <Select value={retryLogic} onValueChange={setRetryLogic}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="FIXED">{t.retryFixed}</SelectItem>
              <SelectItem value="EXPONENTIAL_BACKOFF">{t.retryExponential}</SelectItem>
              <SelectItem value="LINEAR_BACKOFF">{t.retryLinear}</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="space-y-2">
          <Label>{t.retryDelay}</Label>
          <Input
            type="number"
            min={0}
            value={retryDelaySeconds}
            onChange={(e) => setRetryDelaySeconds(Number(e.target.value) || 0)}
          />
        </div>
      </div>

      <div className="grid grid-cols-3 gap-4">
        <div className="space-y-2">
          <Label>{t.timeoutSeconds}</Label>
          <Input
            type="number"
            min={0}
            value={timeoutSeconds}
            onChange={(e) => setTimeoutSeconds(Number(e.target.value) || 0)}
          />
        </div>
        <div className="space-y-2">
          <Label>{t.timeoutPolicy}</Label>
          <Select value={timeoutPolicy} onValueChange={setTimeoutPolicy}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="TIME_OUT_WF">{t.timeoutPolicyTimeOut}</SelectItem>
              <SelectItem value="ALERT_ONLY">{t.timeoutPolicyAlert}</SelectItem>
              <SelectItem value="RETRY">{t.timeoutPolicyRetry}</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="space-y-2">
          <Label>{t.responseTimeoutLabel}</Label>
          <Input
            type="number"
            min={0}
            value={responseTimeoutSeconds}
            onChange={(e) => setResponseTimeoutSeconds(Number(e.target.value) || 0)}
          />
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label>{t.concurrentExecLimit}</Label>
          <Input
            type="number"
            min={0}
            value={concurrentExecLimit}
            onChange={(e) => setConcurrentExecLimit(Number(e.target.value) || 0)}
            placeholder={t.unlimitedHint}
          />
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label>{t.inputKeys}</Label>
          <TagInput
            tags={inputKeys}
            onChange={setInputKeys}
            placeholder={t.typeKeyAndEnter}
          />
          {inputKeys.length > 0 && (
            <div className="space-y-1.5 pt-1">
              {inputKeys.map((k) => (
                <div key={k} className="flex items-center gap-2">
                  <span className="text-[11px] font-mono text-blue-600 dark:text-blue-400 min-w-20 truncate">{k}</span>
                  <Input
                    value={inputDescriptions[k] ?? ""}
                    onChange={(e) => setInputDescriptions({ ...inputDescriptions, [k]: e.target.value })}
                    placeholder="Describe this input… (optional)"
                    className="h-7 text-xs"
                    aria-label={`Description for input ${k}`}
                  />
                </div>
              ))}
            </div>
          )}
        </div>
        <div className="space-y-2">
          <Label>{t.outputKeys}</Label>
          <TagInput
            tags={outputKeys}
            onChange={setOutputKeys}
            placeholder={t.typeKeyAndEnter}
          />
          {outputKeys.length > 0 && (
            <div className="space-y-1.5 pt-1">
              {outputKeys.map((k) => (
                <div key={k} className="flex items-center gap-2">
                  <span className="text-[11px] font-mono text-emerald-600 dark:text-emerald-400 min-w-20 truncate">{k}</span>
                  <Input
                    value={outputDescriptions[k] ?? ""}
                    onChange={(e) => setOutputDescriptions({ ...outputDescriptions, [k]: e.target.value })}
                    placeholder="Describe this output… (optional)"
                    className="h-7 text-xs"
                    aria-label={`Description for output ${k}`}
                  />
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      <div className="space-y-2">
        <Label>{t.retryOnErrors}</Label>
        <TagInput
          tags={retryOnErrors}
          onChange={setRetryOnErrors}
          placeholder={t.retryOnErrorsHint}
        />
      </div>

      <div className="space-y-2">
        <Label>{t.envVars}</Label>
        <Textarea
          rows={3}
          value={envVars}
          onChange={(e) => setEnvVars(e.target.value)}
          placeholder={t.envVarsHint}
          className="font-mono text-xs"
        />
      </div>

      <div className="flex justify-end">
        <Button onClick={handleSubmit} disabled={isPending || !name.trim()}>
          {isPending ? t.creating : t.createTaskDef}
        </Button>
      </div>
    </div>
  );
}
