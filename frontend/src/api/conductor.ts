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

  // 109. Workflow validation
  validateWorkflow: (def: WorkflowDef) =>
    request<ValidationResult>("/metadata/workflow/validate", {
      method: "POST",
      body: JSON.stringify(def),
    }),
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
    request<string>(`/workflow/${encodeURIComponent(id)}/restart`, { method: "POST" }),

  retry: (id: string) =>
    request<string>(`/workflow/${encodeURIComponent(id)}/retry`, { method: "POST" }),

  stats: () => request<Record<string, number>>("/workflow/stats"),

  search: (params?: SearchParams) => {
    const qs = new URLSearchParams();
    if (params?.status) qs.set("status", params.status);
    if (params?.workflowType) qs.set("workflowType", params.workflowType);
    if (params?.freeText) qs.set("freeText", params.freeText);
    if (params?.start !== undefined) qs.set("start", String(params.start));
    if (params?.size !== undefined) qs.set("size", String(params.size));
    if (params?.tags) qs.set("tags", params.tags);
    return request<SearchResult<WorkflowSummary>>(`/workflow/search?${qs}`);
  },

  metrics: (name: string) =>
    request<WorkflowMetrics>(`/workflow/metrics/${encodeURIComponent(name)}`),

  // 102. Dynamic modification
  modify: (id: string, req: ModifyWorkflowRequest) =>
    request<Workflow>(`/workflow/${encodeURIComponent(id)}/modify`, {
      method: "POST",
      body: JSON.stringify(req),
    }),

  // 103. Signals
  sendSignal: (req: SendSignalRequest) =>
    request<SignalResponse>("/workflow/signal", {
      method: "POST",
      body: JSON.stringify(req),
    }),

  // 105. Checkpoints
  createCheckpoint: (id: string, label?: string) =>
    request<WorkflowCheckpoint>(`/workflow/${encodeURIComponent(id)}/checkpoint`, {
      method: "POST",
      body: JSON.stringify({ label }),
    }),

  listCheckpoints: (id: string) =>
    request<WorkflowCheckpoint[]>(`/workflow/${encodeURIComponent(id)}/checkpoints`),

  restoreCheckpoint: (workflowId: string, checkpointId: string) =>
    request<string>(
      `/workflow/${encodeURIComponent(workflowId)}/restore/${encodeURIComponent(checkpointId)}`,
      { method: "POST" }
    ),
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

  // 110. Heartbeat
  heartbeat: (taskId: string) =>
    request<{ acknowledged: boolean }>(`/tasks/${encodeURIComponent(taskId)}/heartbeat`, {
      method: "POST",
    }),
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

// ── Types (re-exported from conductor-types.ts) ──

export type {
  WorkflowDef,
  WorkflowTask,
  TaskDef,
  StartWorkflowRequest,
  Workflow,
  TaskResult,
  TaskUpdateRequest,
  PollTask,
  SearchResult,
  WorkflowSummary,
  SearchParams,
  WorkflowMetrics,
  ModifyWorkflowRequest,
  SendSignalRequest,
  SignalResponse,
  WorkflowCheckpoint,
  ValidationResult,
  ConditionNode,
  CompareOp,
  WorkflowInputParameterDef,
  TaskInputParameterDef,
} from "./conductor-types";

export { formatTs } from "./conductor-types";

import type {
  WorkflowDef,
  TaskDef,
  StartWorkflowRequest,
  Workflow,
  TaskResult,
  TaskUpdateRequest,
  PollTask,
  SearchResult,
  WorkflowSummary,
  SearchParams,
  WorkflowMetrics,
  ModifyWorkflowRequest,
  SendSignalRequest,
  SignalResponse,
  WorkflowCheckpoint,
  ValidationResult,
} from "./conductor-types";
