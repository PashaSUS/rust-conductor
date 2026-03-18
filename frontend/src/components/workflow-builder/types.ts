import type { WorkflowTask, TaskDef } from "@/api/conductor";

export const TASK_TYPES = [
  { value: "SIMPLE", label: "Simple Task", description: "Worker-polled task" },
  { value: "HTTP", label: "HTTP", description: "HTTP API call" },
  { value: "SUB_WORKFLOW", label: "Sub-Workflow", description: "Execute another workflow" },
  { value: "FORK_JOIN", label: "Fork/Join", description: "Parallel execution" },
  { value: "DECISION", label: "Decision/Switch", description: "Conditional branching" },
  { value: "DO_WHILE", label: "Do While Loop", description: "Loop until condition" },
  { value: "WAIT", label: "Wait", description: "Wait for external signal" },
  { value: "EVENT", label: "Event", description: "Publish an event" },
  { value: "TERMINATE", label: "Terminate", description: "End workflow" },
  { value: "SET_VARIABLE", label: "Set Variable", description: "Set workflow variables" },
] as const;

export interface TaskFormState {
  name: string;
  taskReferenceName: string;
  type: string;
  description: string;
  inputParametersJson: string;
  optional: boolean;
  startDelay: number;
  subWorkflowName: string;
  subWorkflowVersion: string;
  caseExpression: string;
  caseValueParam: string;
  loopCondition: string;
  forkBranches: { name: string; tasks: TaskFormState[] }[];
  decisionCases: { caseName: string; tasks: TaskFormState[] }[];
  defaultCaseTasks: TaskFormState[];
  loopTasks: TaskFormState[];
  inputMode: "fields" | "json";
  inputFields: Record<string, string>;
}

export interface InputSource {
  label: string;
  value: string;
  group: string;
}

export function createEmptyTask(): TaskFormState {
  return {
    name: "",
    taskReferenceName: "",
    type: "SIMPLE",
    description: "",
    inputParametersJson: "{}",
    optional: false,
    startDelay: 0,
    subWorkflowName: "",
    subWorkflowVersion: "",
    caseExpression: "",
    caseValueParam: "",
    loopCondition: "",
    forkBranches: [{ name: "Branch 1", tasks: [] }],
    decisionCases: [{ caseName: "case1", tasks: [] }],
    defaultCaseTasks: [],
    loopTasks: [],
    inputMode: "fields",
    inputFields: {},
  };
}

export function taskFormToWorkflowTask(form: TaskFormState): WorkflowTask {
  const task: WorkflowTask = {
    name: form.name,
    taskReferenceName: form.taskReferenceName,
    type: form.type,
  };
  if (form.description) task.description = form.description;
  if (form.optional) task.optional = true;
  if (form.startDelay > 0) task.startDelay = form.startDelay;

  try {
    let params: Record<string, unknown>;
    if (form.inputMode === "fields" && Object.keys(form.inputFields).length > 0) {
      params = {};
      for (const [key, val] of Object.entries(form.inputFields)) {
        if (val.trim()) params[key] = val;
      }
    } else {
      params = JSON.parse(form.inputParametersJson);
    }
    if (Object.keys(params).length > 0) task.inputParameters = params;
  } catch {
    // ignore invalid JSON
  }

  if (form.type === "SUB_WORKFLOW") {
    task.subWorkflowParam = {
      name: form.subWorkflowName,
      ...(form.subWorkflowVersion ? { version: Number(form.subWorkflowVersion) } : {}),
    };
  }

  if (form.type === "FORK_JOIN") {
    task.forkTasks = form.forkBranches.map((b) =>
      b.tasks.map(taskFormToWorkflowTask)
    );
  }

  if (form.type === "DECISION") {
    task.caseExpression = form.caseExpression || undefined;
    task.caseValueParam = form.caseValueParam || undefined;
    const cases: Record<string, WorkflowTask[]> = {};
    form.decisionCases.forEach((c) => {
      cases[c.caseName] = c.tasks.map(taskFormToWorkflowTask);
    });
    task.decisionCases = cases;
    if (form.defaultCaseTasks.length > 0) {
      task.defaultCase = form.defaultCaseTasks.map(taskFormToWorkflowTask);
    }
  }

  if (form.type === "DO_WHILE") {
    task.loopCondition = form.loopCondition || undefined;
    task.loopOver = form.loopTasks.map(taskFormToWorkflowTask);
  }

  return task;
}

export function workflowTaskToForm(task: WorkflowTask): TaskFormState {
  const form = createEmptyTask();
  form.name = task.name;
  form.taskReferenceName = task.taskReferenceName;
  form.type = task.type || "SIMPLE";
  form.description = task.description || "";
  form.optional = task.optional || false;
  form.startDelay = task.startDelay || 0;
  form.inputParametersJson = task.inputParameters
    ? JSON.stringify(task.inputParameters, null, 2)
    : "{}";

  if (task.inputParameters) {
    const fields: Record<string, string> = {};
    for (const [key, val] of Object.entries(task.inputParameters)) {
      fields[key] =
        val === null || val === undefined
          ? ""
          : typeof val === "object"
            ? JSON.stringify(val)
            : String(val);
    }
    form.inputFields = fields;
    form.inputMode = "fields";
  }

  if (task.subWorkflowParam) {
    form.subWorkflowName = task.subWorkflowParam.name;
    form.subWorkflowVersion = task.subWorkflowParam.version?.toString() || "";
  }

  if (task.forkTasks) {
    form.forkBranches = task.forkTasks.map((branch, i) => ({
      name: `Branch ${i + 1}`,
      tasks: branch.map(workflowTaskToForm),
    }));
  }

  if (task.decisionCases) {
    form.decisionCases = Object.entries(task.decisionCases).map(([caseName, tasks]) => ({
      caseName,
      tasks: tasks.map(workflowTaskToForm),
    }));
  }

  if (task.defaultCase) {
    form.defaultCaseTasks = task.defaultCase.map(workflowTaskToForm);
  }

  form.caseExpression = task.caseExpression || "";
  form.caseValueParam = task.caseValueParam || "";

  if (task.loopCondition) form.loopCondition = task.loopCondition;
  if (task.loopOver) form.loopTasks = task.loopOver.map(workflowTaskToForm);

  return form;
}

/** Build available input sources from workflow inputs and previous task outputs */
export function getInputSources(
  allTasks: TaskFormState[],
  taskIndex: number,
  registeredTasks: TaskDef[],
  workflowInputParams: string[]
): InputSource[] {
  const sources: InputSource[] = [];

  for (const p of workflowInputParams) {
    sources.push({
      label: p,
      value: `\${workflow.input.${p}}`,
      group: "Workflow Input",
    });
  }

  for (let i = 0; i < taskIndex; i++) {
    const prev = allTasks[i];
    const ref = prev.taskReferenceName;
    if (!ref) continue;
    const group = ref;
    const def = registeredTasks.find((d) => d.name === prev.name);
    if (def?.outputKeys) {
      for (const key of def.outputKeys) {
        sources.push({
          label: key,
          value: `\${${ref}.output.${key}}`,
          group,
        });
      }
    }
    sources.push({
      label: "output (full)",
      value: `\${${ref}.output}`,
      group,
    });
  }

  return sources;
}
