// ── Types ──

/**
 * Rich, optional metadata for a workflow input parameter.
 * Carried alongside the legacy `inputParameters: string[]` for full
 * Netflix-Conductor backwards compatibility — vanilla clients ignore it.
 */
export interface WorkflowInputParameterDef {
  name: string;
  description?: string;
  type?: 'string' | 'number' | 'boolean' | 'object' | 'array';
  required?: boolean;
  defaultValue?: unknown;
  example?: unknown;
}

/**
 * Rich, optional metadata for a task input/output parameter.
 * Used on both `WorkflowTask.inputParameterDefinitions` and
 * `TaskDef.inputParameterDefinitions` / `outputParameterDefinitions`.
 * Additive — Netflix Conductor clients ignore it.
 */
export interface TaskInputParameterDef {
  name: string;
  description?: string;
  type?: 'string' | 'number' | 'boolean' | 'object' | 'array';
  required?: boolean;
  defaultValue?: unknown;
  example?: unknown;
}

export interface WorkflowDef {
  name: string;
  description?: string;
  version: number;
  tasks: WorkflowTask[];
  inputParameters?: string[];
  /** Optional rich metadata describing each entry of `inputParameters`. */
  inputParameterDefinitions?: WorkflowInputParameterDef[];
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
  sagaEnabled?: boolean;
  baseWorkflow?: string;
  baseWorkflowVersion?: number;
}

export interface WorkflowTask {
  name: string;
  taskReferenceName: string;
  type?: string;
  description?: string;
  inputParameters?: Record<string, unknown>;
  /** Optional rich metadata describing each entry of `inputParameters`. */
  inputParameterDefinitions?: TaskInputParameterDef[];
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
  dynamicTaskNameParam?: string;
  dynamicForkJoinTasksParam?: string;
  dynamicForkTasksParam?: string;
  dynamicForkTasksInputParamName?: string;
  sink?: string;
  asyncComplete?: boolean;
  compensationTask?: WorkflowTask;
  mapItemsParam?: string;
  mapParallelism?: number;
  mapTask?: WorkflowTask;
  heartbeatTimeoutSeconds?: number;
  conditionTree?: ConditionNode;
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
  /** Optional rich metadata describing each entry of `inputKeys`. */
  inputParameterDefinitions?: TaskInputParameterDef[];
  /** Optional rich metadata describing each entry of `outputKeys`. */
  outputParameterDefinitions?: TaskInputParameterDef[];
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
  parentWorkflowId?: string;
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
  tags?: string;
  rootOnly?: boolean;
}

export interface WorkflowMetrics {
  workflowName: string;
  sampleSize: number;
  statusDistribution: Record<string, number>;
  successRate: number;
  failureRate: number;
  avgDurationMs?: number;
  minDurationMs?: number;
  maxDurationMs?: number;
  p50DurationMs?: number;
  p95DurationMs?: number;
}

// ── Advanced Engine Types (101-110) ─────────────────────────────────

export interface ModifyWorkflowRequest {
  addTasks: WorkflowTask[];
  removeTaskRefs: string[];
}

export interface SendSignalRequest {
  signalName: string;
  payload: unknown;
}

export interface SignalResponse {
  signalName: string;
  deliveredTo: number;
}

export interface WorkflowCheckpoint {
  checkpointId: string;
  workflowId: string;
  createdAt: number;
  workflowSnapshot: unknown;
  tasksSnapshot: unknown;
  variablesSnapshot: unknown;
  label?: string;
}

export interface ValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

export type ConditionNode =
  | { And: ConditionNode[] }
  | { Or: ConditionNode[] }
  | { Not: ConditionNode }
  | { Compare: { field: string; op: CompareOp; value: unknown } };

export type CompareOp =
  | "Eq"
  | "Neq"
  | "Gt"
  | "Gte"
  | "Lt"
  | "Lte"
  | "Contains"
  | "StartsWith"
  | "EndsWith";
