const BASE = (import.meta.env.VITE_API_BASE || "") + "/api";

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { "Content-Type": "application/json" },
    ...options,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: res.statusText }));
    throw new Error(err.message || res.statusText);
  }
  if (res.status === 204) return undefined as T;
  const text = await res.text();
  if (!text) return undefined as T;
  return JSON.parse(text) as T;
}

// ── Metadata ──

export const metadataApi = {
  listWorkflowDefs: () => request<WorkflowDef[]>("/metadata/workflow"),

  getWorkflowDef: (name: string, version?: number) =>
    request<WorkflowDef>(`/metadata/workflow/${encodeURIComponent(name)}${version ? `?version=${version}` : ""}`),

  registerWorkflowDef: (def: WorkflowDef) =>
    request<void>("/metadata/workflow", { method: "POST", body: JSON.stringify(def) }),

  deleteWorkflowDef: (name: string, version: number) =>
    request<void>(`/metadata/workflow/${encodeURIComponent(name)}/${version}`, { method: "DELETE" }),

  listTaskDefs: () => request<TaskDef[]>("/metadata/taskdefs"),

  getTaskDef: (name: string) =>
    request<TaskDef>(`/metadata/taskdefs/${encodeURIComponent(name)}`),

  registerTaskDefs: (defs: TaskDef[]) =>
    request<void>("/metadata/taskdefs", { method: "POST", body: JSON.stringify(defs) }),

  deleteTaskDef: (name: string) =>
    request<void>(`/metadata/taskdefs/${encodeURIComponent(name)}`, { method: "DELETE" }),
};

// ── Workflow ──

export const workflowApi = {
  start: (req: StartWorkflowRequest) =>
    request<string>("/workflow", { method: "POST", body: JSON.stringify(req) }),

  get: (id: string) => request<Workflow>(`/workflow/${encodeURIComponent(id)}`),

  terminate: (id: string, reason?: string) =>
    request<void>(`/workflow/${encodeURIComponent(id)}${reason ? `?reason=${encodeURIComponent(reason)}` : ""}`, { method: "DELETE" }),

  pause: (id: string) =>
    request<void>(`/workflow/${encodeURIComponent(id)}/pause`, { method: "PUT" }),

  resume: (id: string) =>
    request<void>(`/workflow/${encodeURIComponent(id)}/resume`, { method: "PUT" }),

  restart: (id: string) =>
    request<void>(`/workflow/${encodeURIComponent(id)}/restart`, { method: "POST" }),

  retry: (id: string) =>
    request<void>(`/workflow/${encodeURIComponent(id)}/retry`, { method: "POST" }),

  stats: () => request<Record<string, number>>("/workflow/stats"),

  search: (params?: SearchParams) => {
    const qs = new URLSearchParams();
    if (params?.status) qs.set("status", params.status);
    if (params?.workflowType) qs.set("workflowType", params.workflowType);
    if (params?.freeText) qs.set("freeText", params.freeText);
    if (params?.start !== undefined) qs.set("start", String(params.start));
    if (params?.size !== undefined) qs.set("size", String(params.size));
    return request<SearchResult<WorkflowSummary>>(`/workflow/search?${qs}`);
  },
};

// ── Tasks ──

export const taskApi = {
  poll: (taskType: string, workerId?: string) =>
    request<PollTask | undefined>(`/tasks/poll/${encodeURIComponent(taskType)}${workerId ? `?workerId=${encodeURIComponent(workerId)}` : ""}`),

  update: (req: TaskUpdateRequest) =>
    request<string>("/tasks", { method: "POST", body: JSON.stringify(req) }),

  get: (taskId: string) =>
    request<TaskResult>(`/tasks/${encodeURIComponent(taskId)}`),

  queueSizes: () => request<Record<string, number>>("/tasks/queue/sizes"),
};

// ── Health ──

export const healthApi = {
  check: () => fetch(`${import.meta.env.VITE_API_BASE || ""}/health`).then((r) => r.json()),
};

// ── Schedules ──

export interface ScheduledWorkflow {
  scheduleId: string;
  name: string;
  cronExpression: string;
  timezone: string;
  workflowName: string;
  workflowVersion: number;
  workflowInput: unknown;
  enabled: boolean;
  lastRunAt?: number;
  nextRunAt?: number;
  lastError?: string;
}

export const scheduleApi = {
  list: () => request<ScheduledWorkflow[]>("/schedule"),

  get: (id: string) => request<ScheduledWorkflow>(`/schedule/${encodeURIComponent(id)}`),

  create: (s: Omit<ScheduledWorkflow, "scheduleId" | "lastRunAt" | "nextRunAt">) =>
    request<ScheduledWorkflow>("/schedule", { method: "POST", body: JSON.stringify({ ...s, scheduleId: "" }) }),

  delete: (id: string) =>
    request<void>(`/schedule/${encodeURIComponent(id)}`, { method: "DELETE" }),

  enable: (id: string) =>
    request<void>(`/schedule/${encodeURIComponent(id)}/enable`, { method: "PUT" }),

  disable: (id: string) =>
    request<void>(`/schedule/${encodeURIComponent(id)}/disable`, { method: "PUT" }),
};

// ── Bulk Operations ──

export const bulkApi = {
  pause: (ids: string[]) =>
    request<Record<string, string>>("/workflow/bulk/pause", { method: "PUT", body: JSON.stringify(ids) }),

  resume: (ids: string[]) =>
    request<Record<string, string>>("/workflow/bulk/resume", { method: "PUT", body: JSON.stringify(ids) }),

  retry: (ids: string[]) =>
    request<Record<string, string>>("/workflow/bulk/retry", { method: "POST", body: JSON.stringify(ids) }),

  restart: (ids: string[]) =>
    request<Record<string, string>>("/workflow/bulk/restart", { method: "POST", body: JSON.stringify(ids) }),

  terminate: (ids: string[], reason?: string) =>
    request<Record<string, string>>("/workflow/bulk/terminate", {
      method: "POST",
      body: JSON.stringify({ workflowIds: ids, reason }),
    }),
};

// ── Types ──

export interface WorkflowDef {
  name: string;
  description?: string;
  version: number;
  tasks: WorkflowTask[];
  inputParameters?: string[];
  outputParameters?: Record<string, unknown>;
  failureWorkflow?: string;
  timeoutSeconds?: number;
  timeoutPolicy?: string;
  schemaVersion?: number;
  restartable?: boolean;
  ownerEmail?: string;
  variables?: Record<string, unknown>;
  inputTemplate?: Record<string, unknown>;
  onCompleteWebhook?: string;
  onFailureWebhook?: string;
  tags?: string[];
  slaDeadlineSeconds?: number;
}

export interface WorkflowTask {
  name: string;
  taskReferenceName: string;
  type?: string;
  description?: string;
  inputParameters?: Record<string, unknown>;
  optional?: boolean;
  startDelay?: number;
  forkTasks?: WorkflowTask[][];
  joinOn?: string[];
  decisionCases?: Record<string, WorkflowTask[]>;
  defaultCase?: WorkflowTask[];
  caseExpression?: string;
  caseValueParam?: string;
  loopCondition?: string;
  loopOver?: WorkflowTask[];
  subWorkflowParam?: { name: string; version?: number };
}

export interface TaskDef {
  name: string;
  description?: string;
  retryCount?: number;
  retryLogic?: string;
  retryDelaySeconds?: number;
  timeoutSeconds?: number;
  timeoutPolicy?: string;
  responseTimeoutSeconds?: number;
  concurrentExecLimit?: number;
  inputKeys?: string[];
  outputKeys?: string[];
  ownerEmail?: string;
  createdOn?: string;
  updatedOn?: string;
  retryOnErrors?: string[];
  envVars?: Record<string, unknown>;
}

export interface StartWorkflowRequest {
  name: string;
  version?: number;
  input?: Record<string, unknown>;
  correlationId?: string;
  priority?: number;
  tags?: string[];
}

export interface Workflow {
  workflowId: string;
  workflowName: string;
  workflowVersion: number;
  status: string;
  input: unknown;
  output: unknown;
  tasks: TaskResult[];
  correlationId?: string;
  startTime: number;
  endTime?: number;
  updateTime: number;
  createdBy?: string;
  reasonForIncompletion?: string;
  priority: number;
  tags?: string[];
}

export interface TaskResult {
  taskId: string;
  workflowInstanceId: string;
  taskType: string;
  taskDefName: string;
  referenceTaskName: string;
  status: string;
  inputData: unknown;
  outputData: unknown;
  scheduledTime?: number;
  startTime?: number;
  endTime?: number;
  updateTime?: number;
  pollCount: number;
  workerId?: string;
  seq: number;
  retryCount: number;
  reasonForIncompletion?: string;
  subWorkflowId?: string;
}

export interface TaskUpdateRequest {
  taskId: string;
  workflowInstanceId: string;
  status: string;
  outputData?: unknown;
  reasonForIncompletion?: string;
  workerId?: string;
}

export interface PollTask {
  taskId: string;
  workflowInstanceId: string;
  taskType: string;
  taskDefName: string;
  referenceTaskName: string;
  status: string;
  inputData: unknown;
  scheduledTime?: number;
  startTime?: number;
  callbackAfterSeconds: number;
  pollCount: number;
  retryCount: number;
}

export interface SearchResult<T> {
  totalHits: number;
  results: T[];
}

export interface WorkflowSummary {
  workflowId: string;
  workflowType: string;
  version: number;
  status: string;
  startTime?: string;
  endTime?: string;
  correlationId?: string;
  priority: number;
}

/** Parse a timestamp that may be epoch millis (number or numeric string) or an ISO string. */
export function formatTs(v: string | number | null | undefined): string {
  if (v == null) return "—";
  const n = typeof v === "number" ? v : Number(v);
  const d = Number.isFinite(n) && n > 1e12 ? new Date(n) : new Date(v);
  return Number.isNaN(d.getTime()) ? "—" : d.toLocaleString();
}

export interface SearchParams {
  status?: string;
  workflowType?: string;
  freeText?: string;
  start?: number;
  size?: number;
}
