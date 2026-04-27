import { MarkerType } from "@xyflow/react";

export const TASK_TYPE_GROUPS = [
  {
    label: "Worker Tasks",
    types: [
      { value: "SIMPLE", label: "Simple", desc: "Worker-polled task" },
      { value: "HTTP", label: "HTTP", desc: "HTTP API call" },
      { value: "DYNAMIC", label: "Dynamic", desc: "Dynamic task resolution" },
    ],
  },
  {
    label: "Flow Control",
    types: [
      { value: "FORK_JOIN", label: "Fork/Join", desc: "Static parallel" },
      { value: "DYNAMIC_FORK_JOIN", label: "Dynamic Fork", desc: "Dynamic parallel" },
      { value: "JOIN", label: "Join", desc: "Wait for branches" },
      { value: "DECISION", label: "Decision", desc: "Conditional branch" },
      { value: "SWITCH", label: "Switch", desc: "Multi-branch switch" },
      { value: "DO_WHILE", label: "Do While", desc: "Loop until condition" },
    ],
  },
  {
    label: "Utilities",
    types: [
      { value: "SUB_WORKFLOW", label: "Sub-Workflow", desc: "Run another workflow" },
      { value: "WAIT", label: "Wait", desc: "Wait for signal" },
      { value: "EVENT", label: "Event", desc: "Publish event" },
      { value: "TERMINATE", label: "Terminate", desc: "End workflow" },
    ],
  },
] as const;

export const TASK_TYPES = TASK_TYPE_GROUPS.flatMap((g) => g.types.map((t) => t.value));

export const TASK_TYPE_COLORS: Record<string, string> = {
  SIMPLE: "#3b82f6",
  HTTP: "#8b5cf6",
  SUB_WORKFLOW: "#06b6d4",
  FORK_JOIN: "#f59e0b",
  DYNAMIC_FORK_JOIN: "#d97706",
  JOIN: "#eab308",
  DECISION: "#ec4899",
  SWITCH: "#ec4899",
  DO_WHILE: "#10b981",
  WAIT: "#6b7280",
  EVENT: "#f97316",
  TERMINATE: "#ef4444",
  DYNAMIC: "#14b8a6",
};

export const DATA_EDGE_STYLE = {
  markerEnd: { type: MarkerType.ArrowClosed as const, width: 12, height: 12 },
  animated: false,
  style: { stroke: "#3b82f6", strokeWidth: 1.5, strokeDasharray: "6 3" },
};

export const FLOW_EDGE_STYLE = {
  markerEnd: { type: MarkerType.ArrowClosed as const, width: 14, height: 14 },
  animated: false,
  type: "smoothstep" as const,
  style: { stroke: "#6b7280", strokeWidth: 2 },
};

export const TASK_TYPE_DEFAULT_INPUTS: Record<string, Record<string, unknown>> = {
  HTTP: {
    "http_request": {
      uri: "https://",
      method: "GET",
      headers: {},
      body: {},
      connectionTimeOut: 3000,
      readTimeOut: 3000,
    },
  },
  EVENT: {
    sink: "conductor:event-name",
  },
  TERMINATE: {
    terminationStatus: "COMPLETED",
    terminationReason: "",
    workflowOutput: {},
  },
  WAIT: {
    duration: "1s",
  },
  DECISION: {
    caseValueParam: "caseValue",
  },
  SWITCH: {},
  DO_WHILE: {},
  FORK_JOIN: {},
};
