import { useState, useCallback, useRef, useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  metadataApi,
  workflowApi,
  type WorkflowDef,
  type TaskDef,
  type StartWorkflowRequest,
} from "@/api/conductor";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Checkbox } from "@/components/ui/checkbox";
import { SearchableSelect } from "@/components/SearchableSelect";
import { JsonView } from "@/components/JsonView";
import {
  Zap,
  Play,
  Pause,
  Square,
  RefreshCw,
  Shuffle,
  AlertTriangle,
  CheckCircle2,
  XCircle,
  Clock,
  Flame,
  BarChart3,
  Trash2,
  Plus,
} from "lucide-react";
import { toast } from "sonner";

// ── Random data generators ──

function randInt(min: number, max: number) {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}
function randFloat(min: number, max: number) {
  return +(Math.random() * (max - min) + min).toFixed(2);
}
function randPick<T>(arr: T[]): T {
  return arr[randInt(0, arr.length - 1)];
}
function randBool() {
  return Math.random() > 0.5;
}
function randString(len: number) {
  const chars = "abcdefghijklmnopqrstuvwxyz0123456789";
  return Array.from({ length: len }, () => randPick(chars.split(""))).join("");
}
function randWord() {
  const words = [
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel",
    "india", "juliet", "kilo", "lima", "mike", "november", "oscar", "papa",
    "quebec", "romeo", "sierra", "tango", "uniform", "victor", "whiskey",
    "xray", "yankee", "zulu", "phoenix", "thunder", "storm", "blaze",
    "rocket", "nebula", "comet", "aurora", "glacier", "canyon", "summit",
  ];
  return randPick(words);
}
function randName() {
  const first = ["Alice", "Bob", "Charlie", "Diana", "Eve", "Frank", "Grace", "Hank", "Ivy", "Jack"];
  const last = ["Smith", "Jones", "Brown", "Wilson", "Taylor", "Clark", "Moore", "White", "King", "Hall"];
  return `${randPick(first)} ${randPick(last)}`;
}
function randEmail() {
  return `${randWord()}.${randWord()}@${randWord()}.com`;
}
function randUUID() {
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    return (c === "x" ? r : (r & 0x3) | 0x8).toString(16);
  });
}
function randIP() {
  return `${randInt(10, 192)}.${randInt(0, 255)}.${randInt(0, 255)}.${randInt(1, 254)}`;
}
function randUrl() {
  const protos = ["https"];
  const tlds = ["com", "io", "dev", "org", "net"];
  return `${randPick(protos)}://${randWord()}.${randPick(tlds)}/api/${randWord()}`;
}

type ValueType = "string" | "number" | "boolean" | "email" | "uuid" | "ip" | "url" | "name" | "json_object" | "json_array" | "regex";

const VALUE_TYPES: { value: ValueType; label: string }[] = [
  { value: "string", label: "Random String" },
  { value: "number", label: "Random Number" },
  { value: "boolean", label: "Random Boolean" },
  { value: "email", label: "Random Email" },
  { value: "uuid", label: "Random UUID" },
  { value: "ip", label: "Random IP Address" },
  { value: "url", label: "Random URL" },
  { value: "name", label: "Random Name" },
  { value: "json_object", label: "Random JSON Object" },
  { value: "json_array", label: "Random JSON Array" },
  { value: "regex", label: "Regex Pattern" },
];

function generateValue(type: ValueType): unknown {
  switch (type) {
    case "string": return `${randWord()}_${randString(6)}`;
    case "number": return randFloat(0, 10000);
    case "boolean": return randBool();
    case "email": return randEmail();
    case "uuid": return randUUID();
    case "ip": return randIP();
    case "url": return randUrl();
    case "name": return randName();
    case "json_object": return {
      id: randUUID(),
      name: randName(),
      value: randFloat(0, 100),
      active: randBool(),
      tags: [randWord(), randWord()],
    };
    case "json_array": return Array.from({ length: randInt(2, 5) }, () => ({
      key: randWord(),
      value: randFloat(0, 1000),
    }));
    default: return randString(10);
  }
}

// ── Simple regex-to-string generator ──
// Supports: literal chars, [a-z] [A-Z] [0-9] char classes, \d \w \s shortcuts,
// {n} {n,m} quantifiers, + * ?, (group), | alternation, . wildcard
function generateFromRegex(pattern: string): string {
  let pos = 0;
  const src = pattern;

  function peek() { return src[pos]; }
  function next() { return src[pos++]; }
  function hasMore() { return pos < src.length; }

  function charRange(a: string, b: string): string[] {
    const result: string[] = [];
    for (let c = a.charCodeAt(0); c <= b.charCodeAt(0); c++) result.push(String.fromCharCode(c));
    return result;
  }

  const DIGITS = charRange("0", "9");
  const LOWER = charRange("a", "z");
  const UPPER = charRange("A", "Z");
  const WORD = [...LOWER, ...UPPER, ...DIGITS, "_"];
  const PRINTABLE = [...WORD, " ", "!", "@", "#", "$", "%", "&", "*", "-", "+", "=", ".", ",", ";", ":" ];

  function parseCharClass(): string[] {
    const chars: string[] = [];
    const negated = peek() === "^";
    if (negated) next();
    while (hasMore() && peek() !== "]") {
      const c = next()!;
      if (c === "\\" && hasMore()) {
        const esc = next()!;
        if (esc === "d") chars.push(...DIGITS);
        else if (esc === "w") chars.push(...WORD);
        else if (esc === "s") chars.push(" ", "\t");
        else chars.push(esc);
      } else if (peek() === "-" && pos + 1 < src.length && src[pos + 1] !== "]") {
        next(); // consume -
        const end = next()!;
        chars.push(...charRange(c, end));
      } else {
        chars.push(c);
      }
    }
    if (peek() === "]") next();
    if (negated) {
      const set = new Set(chars);
      return PRINTABLE.filter((c) => !set.has(c));
    }
    return chars.length > 0 ? chars : ["a"];
  }

  function parseAtom(): () => string {
    if (!hasMore()) return () => "";
    const c = peek()!;
    if (c === "(") {
      next();
      const inner = parseAlternation();
      if (peek() === ")") next();
      return inner;
    }
    if (c === "[") {
      next();
      const chars = parseCharClass();
      return () => randPick(chars);
    }
    if (c === "\\") {
      next();
      const esc = next()!;
      if (esc === "d") return () => randPick(DIGITS);
      if (esc === "w") return () => randPick(WORD);
      if (esc === "s") return () => randPick([" ", "\t"]);
      return () => esc;
    }
    if (c === ".") {
      next();
      return () => randPick(PRINTABLE);
    }
    next();
    return () => c;
  }

  function parseQuantifier(atom: () => string): () => string {
    if (!hasMore()) return atom;
    const c = peek();
    if (c === "*") { next(); const n = randInt(0, 5); return () => Array.from({ length: n }, () => atom()).join(""); }
    if (c === "+") { next(); const n = randInt(1, 5); return () => Array.from({ length: n }, () => atom()).join(""); }
    if (c === "?") { next(); return () => (randBool() ? atom() : ""); }
    if (c === "{") {
      const saved = pos;
      next();
      let numStr = "";
      while (hasMore() && /\d/.test(peek()!)) numStr += next();
      if (peek() === "}") {
        next();
        const n = parseInt(numStr, 10) || 1;
        return () => Array.from({ length: n }, () => atom()).join("");
      }
      if (peek() === ",") {
        next();
        let maxStr = "";
        while (hasMore() && /\d/.test(peek()!)) maxStr += next();
        if (peek() === "}") {
          next();
          const min = parseInt(numStr, 10) || 0;
          const max = maxStr ? parseInt(maxStr, 10) : min + 5;
          return () => { const n = randInt(min, max); return Array.from({ length: n }, () => atom()).join(""); };
        }
      }
      pos = saved;
    }
    return atom;
  }

  function parseSequence(): () => string {
    const parts: (() => string)[] = [];
    while (hasMore() && peek() !== ")" && peek() !== "|") {
      const atom = parseAtom();
      parts.push(parseQuantifier(atom));
    }
    return () => parts.map((p) => p()).join("");
  }

  function parseAlternation(): () => string {
    const alternatives: (() => string)[] = [parseSequence()];
    while (hasMore() && peek() === "|") {
      next();
      alternatives.push(parseSequence());
    }
    return () => randPick(alternatives)();
  }

  const gen = parseAlternation();
  return gen();
}

// ── Parameter config for stress test ──
interface ParamConfig {
  key: string;
  type: ValueType;
  fixedValue?: string;
  regexPattern?: string;
  useFixed: boolean;
}

// ── Stress run result tracking ──
interface StressResult {
  index: number;
  workflowId?: string;
  error?: string;
  startedAt: number;
  finishedAt?: number;
  input: Record<string, unknown>;
}

// ── Preset templates for quick start ──
interface StressPreset {
  label: string;
  description: string;
  count: number;
  concurrency: number;
  delayMs: number;
  genWorkflow: boolean;
  taskCount?: number;
}

const PRESETS: StressPreset[] = [
  { label: "Quick Smoke Test", description: "10 workflows, fast", count: 10, concurrency: 5, delayMs: 0, genWorkflow: false },
  { label: "Medium Load", description: "100 workflows, moderate pace", count: 100, concurrency: 10, delayMs: 50, genWorkflow: false },
  { label: "Heavy Load", description: "500 workflows, high concurrency", count: 500, concurrency: 25, delayMs: 10, genWorkflow: false },
  { label: "Soak Test", description: "1000 workflows, steady drip", count: 1000, concurrency: 5, delayMs: 100, genWorkflow: false },
  { label: "Random Workflows", description: "Generate 50 random workflow defs + 200 runs", count: 200, concurrency: 10, delayMs: 20, genWorkflow: true, taskCount: 5 },
];

// ── Random workflow definition generator ──

const RANDOM_TASK_TYPES = ["SIMPLE", "HTTP", "WAIT", "SUB_WORKFLOW", "EVENT"];

function generateRandomWorkflowDef(taskDefs: TaskDef[], taskCount: number): WorkflowDef {
  const name = `stress_${randWord()}_${randWord()}_${randString(4)}`;
  const inputParams = Array.from({ length: randInt(1, 4) }, () => randWord());

  const tasks = Array.from({ length: taskCount }, (_, i) => {
    const type = randPick(RANDOM_TASK_TYPES);
    const useDef = type === "SIMPLE" && taskDefs.length > 0 && randBool();
    const td = useDef ? randPick(taskDefs) : null;
    const taskName = td ? td.name : `${type.toLowerCase()}_${randWord()}`;

    const inputParameters: Record<string, unknown> = {};
    if (td?.inputKeys) {
      for (const k of td.inputKeys) {
        inputParameters[k] = i === 0
          ? `\${workflow.input.${randPick(inputParams)}}`
          : `\${${name}_task_${i - 1}.output.result}`;
      }
    } else {
      inputParameters["input_" + randWord()] = i === 0
        ? `\${workflow.input.${randPick(inputParams)}}`
        : `\${${name}_task_${i - 1}.output.result}`;
    }

    if (type === "HTTP") {
      inputParameters["http_request"] = {
        uri: randUrl(),
        method: randPick(["GET", "POST", "PUT"]),
        headers: { "Content-Type": "application/json" },
      };
    }

    return {
      name: taskName,
      taskReferenceName: `${name}_task_${i}`,
      type,
      inputParameters,
      optional: randBool() && i > 0,
    };
  });

  return {
    name,
    version: 1,
    description: `Auto-generated stress test workflow (${taskCount} tasks)`,
    tasks,
    inputParameters: inputParams,
    tags: ["stress-test", "auto-generated"],
  };
}

function generateRandomInput(paramConfigs: ParamConfig[]): Record<string, unknown> {
  const input: Record<string, unknown> = {};
  for (const p of paramConfigs) {
    if (p.useFixed) {
      input[p.key] = tryParse(p.fixedValue ?? "");
    } else if (p.type === "regex") {
      input[p.key] = p.regexPattern ? generateFromRegex(p.regexPattern) : randString(8);
    } else {
      input[p.key] = generateValue(p.type);
    }
  }
  return input;
}

function tryParse(s: string): unknown {
  try { return JSON.parse(s); } catch { return s; }
}

export default function WorkflowStresser() {
  const queryClient = useQueryClient();

  // Workflow selection
  const [selectedWorkflow, setSelectedWorkflow] = useState("");
  const [selectedVersion, setSelectedVersion] = useState<number | undefined>();

  // Stress config
  const [count, setCount] = useState(100);
  const [concurrency, setConcurrency] = useState(10);
  const [delayMs, setDelayMs] = useState(0);
  const [correlationPrefix, setCorrelationPrefix] = useState("stress");
  const [tags, setTags] = useState("stress-test");
  const [priority, setPriority] = useState(0);

  // Random workflow generation
  const [generateWorkflows, setGenerateWorkflows] = useState(false);
  const [genCount, setGenCount] = useState(5);
  const [genTaskCount, setGenTaskCount] = useState(4);

  // Parameter configs
  const [paramConfigs, setParamConfigs] = useState<ParamConfig[]>([]);

  // Run state
  const [running, setRunning] = useState(false);
  const [paused, setPaused] = useState(false);
  const [results, setResults] = useState<StressResult[]>([]);
  const [progress, setProgress] = useState(0);
  const abortRef = useRef(false);
  const pauseRef = useRef(false);
  const startTimeRef = useRef(0);
  const [elapsed, setElapsed] = useState(0);
  const timerRef = useRef<ReturnType<typeof setInterval>>(undefined);

  // Generated workflow names for the "generate" mode
  const [generatedDefs, setGeneratedDefs] = useState<string[]>([]);

  // Fetch data
  const { data: workflowDefs } = useQuery({
    queryKey: ["workflowDefs"],
    queryFn: metadataApi.listWorkflowDefs,
  });
  const { data: taskDefs } = useQuery({
    queryKey: ["task-defs"],
    queryFn: metadataApi.listTaskDefs,
  });

  const workflowOptions = (workflowDefs ?? []).map((d) => ({
    label: `${d.name} (v${d.version})`,
    value: `${d.name}::${d.version}`,
  }));

  // When user picks a workflow, auto-populate param configs from its inputParameters
  const onSelectWorkflow = useCallback((val: string) => {
    setSelectedWorkflow(val);
    const [name, ver] = val.split("::");
    setSelectedVersion(ver ? Number(ver) : undefined);
    const def = workflowDefs?.find((d) => d.name === name && d.version === Number(ver));
    if (def?.inputParameters) {
      setParamConfigs(
        def.inputParameters.map((k) => ({
          key: k,
          type: "string" as ValueType,
          useFixed: false,
        }))
      );
    } else {
      setParamConfigs([]);
    }
  }, [workflowDefs]);

  const addParam = useCallback(() => {
    setParamConfigs((prev) => [...prev, { key: "", type: "string", useFixed: false }]);
  }, []);

  const removeParam = useCallback((idx: number) => {
    setParamConfigs((prev) => prev.filter((_, i) => i !== idx));
  }, []);

  const updateParam = useCallback((idx: number, update: Partial<ParamConfig>) => {
    setParamConfigs((prev) => prev.map((p, i) => (i === idx ? { ...p, ...update } : p)));
  }, []);

  // Apply preset
  const applyPreset = useCallback((preset: StressPreset) => {
    setCount(preset.count);
    setConcurrency(preset.concurrency);
    setDelayMs(preset.delayMs);
    setGenerateWorkflows(preset.genWorkflow);
    if (preset.taskCount) setGenTaskCount(preset.taskCount);
    toast.success(`Applied preset: ${preset.label}`);
  }, []);

  // Preview a single random input
  const [previewInput, setPreviewInput] = useState<Record<string, unknown> | null>(null);
  const generatePreview = useCallback(() => {
    if (paramConfigs.length === 0 && !generateWorkflows) {
      setPreviewInput({ _note: "No input parameters configured" });
      return;
    }
    setPreviewInput(generateRandomInput(paramConfigs));
  }, [paramConfigs, generateWorkflows]);

  // ── MAIN: Run stress test ──
  const runStressTest = useCallback(async () => {
    if (!generateWorkflows && !selectedWorkflow) {
      toast.error("Select a workflow or enable random generation");
      return;
    }

    abortRef.current = false;
    pauseRef.current = false;
    setRunning(true);
    setPaused(false);
    setResults([]);
    setProgress(0);
    setElapsed(0);
    startTimeRef.current = Date.now();
    timerRef.current = setInterval(() => setElapsed(Date.now() - startTimeRef.current), 200);

    const allResults: StressResult[] = [];
    let completed = 0;
    let workflowNames: string[] = [];

    try {
      // Step 1: If generating random workflows, create the definitions first
      if (generateWorkflows) {
        toast.info(`Generating ${genCount} random workflow definitions...`);
        const defs: WorkflowDef[] = [];
        for (let i = 0; i < genCount; i++) {
          const def = generateRandomWorkflowDef(taskDefs ?? [], genTaskCount);
          defs.push(def);
          try {
            await metadataApi.registerWorkflowDef(def);
            workflowNames.push(def.name);
          } catch (err) {
            toast.error(`Failed to register ${def.name}: ${(err as Error).message}`);
          }
        }
        setGeneratedDefs(workflowNames);
        if (workflowNames.length === 0) {
          toast.error("No workflow definitions were created");
          setRunning(false);
          clearInterval(timerRef.current);
          return;
        }
        toast.success(`Created ${workflowNames.length} workflow definitions`);
        queryClient.invalidateQueries({ queryKey: ["workflowDefs"] });
      } else {
        const [name] = selectedWorkflow.split("::");
        workflowNames = [name];
      }

      // Step 2: Fire workflows in batches of `concurrency`
      const total = count;
      const pending: Promise<void>[] = [];

      for (let i = 0; i < total; i++) {
        if (abortRef.current) break;

        // Pause loop
        while (pauseRef.current && !abortRef.current) {
          await new Promise((r) => setTimeout(r, 200));
        }
        if (abortRef.current) break;

        const wfName = generateWorkflows ? randPick(workflowNames) : workflowNames[0];
        const input = generateRandomInput(paramConfigs);
        const tagList = tags.split(",").map((t) => t.trim()).filter(Boolean);

        const req: StartWorkflowRequest = {
          name: wfName,
          version: generateWorkflows ? 1 : selectedVersion,
          input,
          correlationId: `${correlationPrefix}-${Date.now()}-${i}`,
          priority,
          tags: tagList.length > 0 ? tagList : undefined,
        };

        const idx = i;
        const result: StressResult = {
          index: idx,
          startedAt: Date.now(),
          input,
        };

        const job = (async () => {
          try {
            const id = await workflowApi.start(req);
            result.workflowId = id;
            result.finishedAt = Date.now();
          } catch (err) {
            result.error = (err as Error).message;
            result.finishedAt = Date.now();
          }
          allResults.push(result);
          completed++;
          setProgress(completed);
          setResults([...allResults]);
        })();

        pending.push(job);

        // Concurrency control: when batch is full, wait for all
        if (pending.length >= concurrency) {
          await Promise.all(pending);
          pending.length = 0;
        }

        // Optional delay between batches
        if (delayMs > 0 && pending.length === 0) {
          await new Promise((r) => setTimeout(r, delayMs));
        }
      }

      // Drain remaining
      if (pending.length > 0) {
        await Promise.all(pending);
      }
    } catch (err) {
      toast.error(`Stress test error: ${(err as Error).message}`);
    } finally {
      setRunning(false);
      clearInterval(timerRef.current);
      setElapsed(Date.now() - startTimeRef.current);
      queryClient.invalidateQueries({ queryKey: ["workflowSearch"] });

      const successCount = allResults.filter((r) => r.workflowId).length;
      const failCount = allResults.filter((r) => r.error).length;
      if (allResults.length > 0) {
        toast.info(`Done: ${successCount} succeeded, ${failCount} failed out of ${allResults.length}`);
      }
    }
  }, [
    selectedWorkflow, selectedVersion, count, concurrency, delayMs,
    correlationPrefix, tags, priority, paramConfigs,
    generateWorkflows, genCount, genTaskCount, taskDefs, queryClient,
  ]);

  const stopTest = useCallback(() => {
    abortRef.current = true;
    pauseRef.current = false;
    setPaused(false);
  }, []);

  const togglePause = useCallback(() => {
    pauseRef.current = !pauseRef.current;
    setPaused(pauseRef.current);
  }, []);

  // Stats
  const successCount = results.filter((r) => r.workflowId).length;
  const failCount = results.filter((r) => r.error).length;
  const avgLatency = results.length > 0
    ? Math.round(results.filter((r) => r.finishedAt).reduce((sum, r) => sum + (r.finishedAt! - r.startedAt), 0) / results.filter((r) => r.finishedAt).length)
    : 0;
  const throughput = elapsed > 0 ? ((successCount / (elapsed / 1000))).toFixed(1) : "0";

  // Cleanup timer
  useEffect(() => {
    return () => { if (timerRef.current) clearInterval(timerRef.current); };
  }, []);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-orange-500/10 flex items-center justify-center">
            <Flame className="h-5 w-5 text-orange-500" />
          </div>
          <div>
            <h1 className="text-2xl font-bold tracking-tight">Workflow Stresser</h1>
            <p className="text-sm text-muted-foreground">Generate random data & spawn hundreds of workflows for testing</p>
          </div>
        </div>
        {running && (
          <Badge variant="destructive" className="animate-pulse gap-1.5 text-sm px-3 py-1">
            <Zap className="h-3.5 w-3.5" />
            Running...
          </Badge>
        )}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Left column: Configuration */}
        <div className="lg:col-span-2 space-y-4">
          {/* Presets */}
          <div className="rounded-lg border bg-card p-4">
            <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
              <Zap className="h-4 w-4 text-amber-500" /> Quick Presets
            </h3>
            <div className="flex flex-wrap gap-2">
              {PRESETS.map((preset) => (
                <button
                  key={preset.label}
                  onClick={() => applyPreset(preset)}
                  disabled={running}
                  className="px-3 py-2 rounded-lg border bg-background hover:bg-muted transition-colors text-left disabled:opacity-50"
                >
                  <p className="text-xs font-medium">{preset.label}</p>
                  <p className="text-[10px] text-muted-foreground">{preset.description}</p>
                </button>
              ))}
            </div>
          </div>

          {/* Workflow source */}
          <div className="rounded-lg border bg-card p-4 space-y-4">
            <h3 className="text-sm font-semibold flex items-center gap-2">
              <Shuffle className="h-4 w-4 text-blue-500" /> Workflow Source
            </h3>

            <div className="flex items-center gap-3">
              <Checkbox
                checked={generateWorkflows}
                onCheckedChange={(c) => setGenerateWorkflows(c === true)}
                disabled={running}
              />
              <Label className="text-sm cursor-pointer">
                Auto-generate random workflow definitions (ignores selection below)
              </Label>
            </div>

            {generateWorkflows ? (
              <div className="grid grid-cols-2 gap-3 bg-muted/30 rounded-lg p-3">
                <div>
                  <Label className="text-xs text-muted-foreground">Definitions to generate</Label>
                  <Input
                    type="number" min={1} max={50} value={genCount}
                    onChange={(e) => setGenCount(Number(e.target.value))}
                    disabled={running} className="h-8 mt-1"
                  />
                </div>
                <div>
                  <Label className="text-xs text-muted-foreground">Tasks per workflow</Label>
                  <Input
                    type="number" min={1} max={20} value={genTaskCount}
                    onChange={(e) => setGenTaskCount(Number(e.target.value))}
                    disabled={running} className="h-8 mt-1"
                  />
                </div>
                {generatedDefs.length > 0 && (
                  <div className="col-span-2">
                    <Label className="text-xs text-muted-foreground">Generated definitions</Label>
                    <div className="flex flex-wrap gap-1 mt-1">
                      {generatedDefs.map((n) => (
                        <Badge key={n} variant="outline" className="text-[10px]">{n}</Badge>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            ) : (
              <div>
                <Label className="text-xs text-muted-foreground">Select existing workflow</Label>
                <SearchableSelect
                  options={workflowOptions}
                  value={selectedWorkflow}
                  onChange={onSelectWorkflow}
                  placeholder="Choose a workflow definition..."
                />
              </div>
            )}
          </div>

          {/* Stress configuration */}
          <div className="rounded-lg border bg-card p-4 space-y-4">
            <h3 className="text-sm font-semibold flex items-center gap-2">
              <BarChart3 className="h-4 w-4 text-purple-500" /> Stress Configuration
            </h3>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
              <div>
                <Label className="text-xs text-muted-foreground">Total Workflows</Label>
                <Input
                  type="number" min={1} max={10000} value={count}
                  onChange={(e) => setCount(Number(e.target.value))}
                  disabled={running} className="h-8 mt-1"
                />
              </div>
              <div>
                <Label className="text-xs text-muted-foreground">Concurrency</Label>
                <Input
                  type="number" min={1} max={100} value={concurrency}
                  onChange={(e) => setConcurrency(Number(e.target.value))}
                  disabled={running} className="h-8 mt-1"
                />
              </div>
              <div>
                <Label className="text-xs text-muted-foreground">Delay (ms)</Label>
                <Input
                  type="number" min={0} max={5000} value={delayMs}
                  onChange={(e) => setDelayMs(Number(e.target.value))}
                  disabled={running} className="h-8 mt-1"
                />
              </div>
              <div>
                <Label className="text-xs text-muted-foreground">Priority</Label>
                <Input
                  type="number" min={0} max={99} value={priority}
                  onChange={(e) => setPriority(Number(e.target.value))}
                  disabled={running} className="h-8 mt-1"
                />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div>
                <Label className="text-xs text-muted-foreground">Correlation ID Prefix</Label>
                <Input
                  value={correlationPrefix}
                  onChange={(e) => setCorrelationPrefix(e.target.value)}
                  disabled={running} className="h-8 mt-1" placeholder="stress"
                />
              </div>
              <div>
                <Label className="text-xs text-muted-foreground">Tags (comma-separated)</Label>
                <Input
                  value={tags}
                  onChange={(e) => setTags(e.target.value)}
                  disabled={running} className="h-8 mt-1" placeholder="stress-test"
                />
              </div>
            </div>
          </div>

          {/* Input parameters */}
          {!generateWorkflows && (
            <div className="rounded-lg border bg-card p-4 space-y-3">
              <div className="flex items-center justify-between">
                <h3 className="text-sm font-semibold flex items-center gap-2">
                  <Shuffle className="h-4 w-4 text-emerald-500" /> Input Parameters
                </h3>
                <div className="flex gap-2">
                  <Button variant="outline" size="sm" className="h-7 text-xs" onClick={generatePreview}>
                    <RefreshCw className="h-3 w-3 mr-1" /> Preview
                  </Button>
                  <Button variant="outline" size="sm" className="h-7 text-xs" onClick={addParam} disabled={running}>
                    <Plus className="h-3 w-3 mr-1" /> Add
                  </Button>
                </div>
              </div>

              {paramConfigs.length === 0 && (
                <p className="text-xs text-muted-foreground py-2">
                  {selectedWorkflow
                    ? "No input parameters defined for this workflow. Add custom ones or run with empty input."
                    : "Select a workflow to auto-populate parameters, or add them manually."}
                </p>
              )}

              <div className="space-y-2">
                {paramConfigs.map((param, idx) => (
                  <div key={idx} className="flex items-center gap-2 bg-muted/30 rounded-lg p-2">
                    <Input
                      placeholder="Key"
                      value={param.key}
                      onChange={(e) => updateParam(idx, { key: e.target.value })}
                      disabled={running}
                      className="h-7 text-xs w-32"
                    />
                    <div className="flex items-center gap-1.5">
                      <Checkbox
                        checked={param.useFixed}
                        onCheckedChange={(c) => updateParam(idx, { useFixed: c === true })}
                        disabled={running}
                      />
                      <span className="text-[10px] text-muted-foreground whitespace-nowrap">Fixed</span>
                    </div>
                    {param.useFixed ? (
                      <Input
                        placeholder="Fixed value (JSON or string)"
                        value={param.fixedValue ?? ""}
                        onChange={(e) => updateParam(idx, { fixedValue: e.target.value })}
                        disabled={running}
                        className="h-7 text-xs flex-1"
                      />
                    ) : (
                      <div className="flex gap-1.5 flex-1 items-center">
                        <Select
                          value={param.type}
                          onValueChange={(v) => updateParam(idx, { type: v as ValueType })}
                          disabled={running}
                        >
                          <SelectTrigger className="h-7 text-xs w-36 shrink-0"><SelectValue /></SelectTrigger>
                          <SelectContent>
                            {VALUE_TYPES.map((vt) => (
                              <SelectItem key={vt.value} value={vt.value}>{vt.label}</SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                        {param.type === "regex" && (
                          <Input
                            placeholder="e.g. [A-Z]{3}-\d{4}"
                            value={param.regexPattern ?? ""}
                            onChange={(e) => updateParam(idx, { regexPattern: e.target.value })}
                            disabled={running}
                            className="h-7 text-xs flex-1 font-mono"
                          />
                        )}
                      </div>
                    )}
                    <Button
                      variant="ghost" size="sm" className="h-7 px-1.5 text-muted-foreground hover:text-destructive"
                      onClick={() => removeParam(idx)} disabled={running}
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                  </div>
                ))}
              </div>

              {previewInput && (
                <div className="border-t pt-2">
                  <Label className="text-xs text-muted-foreground mb-1 block">Preview (sample random input)</Label>
                  <JsonView data={previewInput} maxHeight="10rem" />
                </div>
              )}
            </div>
          )}

          {/* Action buttons */}
          <div className="flex gap-3">
            <Button
              size="lg"
              onClick={runStressTest}
              disabled={running || (!generateWorkflows && !selectedWorkflow)}
              className="gap-2"
            >
              <Play className="h-4 w-4" />
              Start Stress Test ({count} workflows)
            </Button>
            {running && (
              <>
                <Button variant="outline" size="lg" onClick={togglePause} className="gap-2">
                  {paused ? <Play className="h-4 w-4" /> : <Pause className="h-4 w-4" />}
                  {paused ? "Resume" : "Pause"}
                </Button>
                <Button variant="destructive" size="lg" onClick={stopTest} className="gap-2">
                  <Square className="h-4 w-4" /> Stop
                </Button>
              </>
            )}
            {!running && results.length > 0 && (
              <Button variant="outline" size="lg" onClick={() => { setResults([]); setProgress(0); setElapsed(0); }} className="gap-2">
                <Trash2 className="h-4 w-4" /> Clear Results
              </Button>
            )}
          </div>
        </div>

        {/* Right column: Live stats + results */}
        <div className="space-y-4">
          {/* Progress */}
          {(running || results.length > 0) && (
            <div className="rounded-lg border bg-card p-4 space-y-3">
              <h3 className="text-sm font-semibold flex items-center gap-2">
                <BarChart3 className="h-4 w-4" /> Progress
              </h3>
              <div className="h-3 bg-muted rounded-full overflow-hidden">
                <div
                  className="h-full bg-primary rounded-full transition-all duration-300"
                  style={{ width: `${count > 0 ? (progress / count) * 100 : 0}%` }}
                />
              </div>
              <p className="text-xs text-muted-foreground text-center">
                {progress} / {count} ({count > 0 ? Math.round((progress / count) * 100) : 0}%)
              </p>

              <div className="grid grid-cols-2 gap-2">
                <div className="bg-muted/50 rounded-lg p-2.5 text-center">
                  <div className="flex items-center justify-center gap-1 text-emerald-600">
                    <CheckCircle2 className="h-3.5 w-3.5" />
                    <span className="text-lg font-bold">{successCount}</span>
                  </div>
                  <p className="text-[10px] text-muted-foreground">Succeeded</p>
                </div>
                <div className="bg-muted/50 rounded-lg p-2.5 text-center">
                  <div className="flex items-center justify-center gap-1 text-red-600">
                    <XCircle className="h-3.5 w-3.5" />
                    <span className="text-lg font-bold">{failCount}</span>
                  </div>
                  <p className="text-[10px] text-muted-foreground">Failed</p>
                </div>
                <div className="bg-muted/50 rounded-lg p-2.5 text-center">
                  <div className="flex items-center justify-center gap-1 text-blue-600">
                    <Clock className="h-3.5 w-3.5" />
                    <span className="text-lg font-bold">{avgLatency}ms</span>
                  </div>
                  <p className="text-[10px] text-muted-foreground">Avg Latency</p>
                </div>
                <div className="bg-muted/50 rounded-lg p-2.5 text-center">
                  <div className="flex items-center justify-center gap-1 text-purple-600">
                    <Zap className="h-3.5 w-3.5" />
                    <span className="text-lg font-bold">{throughput}/s</span>
                  </div>
                  <p className="text-[10px] text-muted-foreground">Throughput</p>
                </div>
              </div>

              <div className="flex items-center justify-between text-xs text-muted-foreground">
                <span>Elapsed: {(elapsed / 1000).toFixed(1)}s</span>
                {running && paused && <Badge variant="secondary" className="text-[10px]">Paused</Badge>}
              </div>
            </div>
          )}

          {/* Recent results */}
          {results.length > 0 && (
            <div className="rounded-lg border bg-card p-4 space-y-2">
              <div className="flex items-center justify-between">
                <h3 className="text-sm font-semibold">Recent Results</h3>
                <Badge variant="outline" className="text-[10px]">{results.length} total</Badge>
              </div>
              <div className="max-h-100 overflow-y-auto space-y-1">
                {results.slice(-50).reverse().map((r) => (
                  <div
                    key={r.index}
                    className={`flex items-center gap-2 p-1.5 rounded text-xs ${
                      r.error ? "bg-destructive/5" : "bg-emerald-500/5"
                    }`}
                  >
                    {r.error ? (
                      <XCircle className="h-3 w-3 text-destructive shrink-0" />
                    ) : (
                      <CheckCircle2 className="h-3 w-3 text-emerald-600 shrink-0" />
                    )}
                    <span className="text-muted-foreground w-6 text-right shrink-0">#{r.index + 1}</span>
                    {r.workflowId ? (
                      <span className="font-mono text-[10px] truncate flex-1">{r.workflowId}</span>
                    ) : (
                      <span className="text-destructive text-[10px] truncate flex-1">{r.error}</span>
                    )}
                    {r.finishedAt && (
                      <span className="text-muted-foreground text-[10px] shrink-0">{r.finishedAt - r.startedAt}ms</span>
                    )}
                  </div>
                ))}
              </div>
              {results.length > 50 && (
                <p className="text-[10px] text-muted-foreground text-center">Showing latest 50 of {results.length}</p>
              )}
            </div>
          )}

          {/* Error summary */}
          {failCount > 0 && (
            <div className="rounded-lg border border-destructive/30 bg-destructive/5 p-4 space-y-2">
              <h3 className="text-sm font-semibold text-destructive flex items-center gap-2">
                <AlertTriangle className="h-4 w-4" /> Errors ({failCount})
              </h3>
              <div className="space-y-1 max-h-40 overflow-y-auto">
                {[...new Set(results.filter((r) => r.error).map((r) => r.error!))].map((err, i) => {
                  const errCount = results.filter((r) => r.error === err).length;
                  return (
                    <div key={i} className="flex items-start gap-2 text-xs">
                      <Badge variant="destructive" className="text-[9px] shrink-0">{errCount}×</Badge>
                      <span className="text-destructive/80 break-all">{err}</span>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
