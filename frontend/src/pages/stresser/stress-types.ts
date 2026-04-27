import type { WorkflowDef, TaskDef } from "@/api/conductor";
import { type ValueType, generateValue, randPick, randWord, randUrl } from "./random-generators";
import { generateFromRegex } from "./regex-generator";

export interface ParamConfig {
  key: string;
  type: ValueType;
  fixedValue?: string;
  regexPattern?: string;
  useFixed: boolean;
}

export interface StressResult {
  index: number;
  workflowId?: string;
  error?: string;
  startedAt: number;
  finishedAt?: number;
  input: Record<string, unknown>;
}

export interface StressPreset {
  label: string;
  description: string;
  count: number;
  concurrency: number;
  delayMs: number;
  genWorkflow: boolean;
  taskCount?: number;
}

export const PRESETS: StressPreset[] = [
  { label: "Quick Smoke Test", description: "10 workflows, fast", count: 10, concurrency: 5, delayMs: 0, genWorkflow: false },
  { label: "Medium Load", description: "100 workflows, moderate pace", count: 100, concurrency: 10, delayMs: 50, genWorkflow: false },
  { label: "Heavy Load", description: "500 workflows, high concurrency", count: 500, concurrency: 25, delayMs: 10, genWorkflow: false },
  { label: "Soak Test", description: "1000 workflows, steady drip", count: 1000, concurrency: 5, delayMs: 100, genWorkflow: false },
  { label: "Random Workflows", description: "Generate 50 random workflow defs + 200 runs", count: 200, concurrency: 10, delayMs: 20, genWorkflow: true, taskCount: 5 },
];

const RANDOM_TASK_TYPES = ["SIMPLE", "HTTP", "WAIT", "SUB_WORKFLOW", "EVENT"];

function randInt(min: number, max: number) { return Math.floor(Math.random() * (max - min + 1)) + min; }
function randBool() { return Math.random() > 0.5; }
function randString(len: number) {
  const chars = "abcdefghijklmnopqrstuvwxyz0123456789";
  return Array.from({ length: len }, () => chars[randInt(0, chars.length - 1)]).join("");
}

export function generateRandomWorkflowDef(taskDefs: TaskDef[], taskCount: number): WorkflowDef {
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

export function generateRandomInput(paramConfigs: ParamConfig[]): Record<string, unknown> {
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

export { type ValueType, VALUE_TYPES } from "./random-generators";
