import { createContext, useContext } from "react";
import { Handle, Position } from "@xyflow/react";
import { Play, Flag, Code, Trash2, GitBranch, GitFork, Repeat } from "lucide-react";
import { TASK_TYPE_COLORS } from "./constants";

/** Context for node edit/delete actions — avoids stale callbacks in node data */
export const DesignerActionsContext = createContext<{
  editTask: (id: string) => void;
  deleteTask: (id: string) => void;
}>({ editTask: () => {}, deleteTask: () => {} });

/* ── Start Node ───────────────────────────────────────── */
export function WorkflowStartNode({ data, selected }: {
  data: { label: string; inputKeys: string[] }; selected?: boolean;
}) {
  return (
    <div className={`rounded-xl px-5 py-3 min-w-48 shadow-md ${selected ? "ring-2 ring-primary" : ""}`}
      style={{ border: "2px solid #22c55e", background: "var(--color-card, #fff)" }}>
      <div className="flex items-center gap-2 mb-1">
        <div className="w-7 h-7 rounded-full flex items-center justify-center bg-green-500/15">
          <Play className="h-3.5 w-3.5 text-green-600" />
        </div>
        <span className="text-xs font-bold text-green-600 dark:text-green-400">START</span>
        <span className="text-[9px] text-muted-foreground ml-auto">workflow inputs</span>
      </div>
      {data.inputKeys.length > 0 ? (
        <div className="mt-2 space-y-1.5">
          {data.inputKeys.map((key) => (
            <div key={key} className="flex items-center justify-end gap-1.5 relative">
              <span className="text-[10px] font-mono text-emerald-600 dark:text-emerald-400">{key}</span>
              <Handle type="source" position={Position.Right} id={`out-${key}`}
                style={{ top: "auto", right: -8, position: "absolute" }}
                className="w-2.5! h-2.5! bg-emerald-500! border! border-background! rounded-full!" />
            </div>
          ))}
        </div>
      ) : (
        <p className="text-[9px] text-muted-foreground mt-1">Add inputs in Settings to create output ports</p>
      )}
      <Handle type="source" position={Position.Bottom} id="flow-out"
        className="w-3! h-3! bg-gray-400! border-2! border-background! rounded-full!" />
    </div>
  );
}

/* ── End Node ─────────────────────────────────────────── */
export function WorkflowEndNode({ data, selected }: {
  data: { label: string; connectedOutputs: string[] }; selected?: boolean;
}) {
  return (
    <div className={`rounded-xl px-5 py-3 min-w-48 shadow-md ${selected ? "ring-2 ring-primary" : ""}`}
      style={{ border: "2px solid #ef4444", background: "var(--color-card, #fff)" }}>
      <Handle type="target" position={Position.Top} id="flow-in"
        className="w-3! h-3! bg-gray-400! border-2! border-background! rounded-full!" />
      <Handle type="target" position={Position.Left} id="data-in"
        className="w-3! h-3! bg-blue-500! border-2! border-background! rounded-full!" />
      <div className="flex items-center gap-2 mb-1">
        <div className="w-7 h-7 rounded-full flex items-center justify-center bg-red-500/15">
          <Flag className="h-3.5 w-3.5 text-red-600" />
        </div>
        <span className="text-xs font-bold text-red-600 dark:text-red-400">END</span>
        <span className="text-[9px] text-muted-foreground ml-auto">workflow output</span>
      </div>
      {data.connectedOutputs.length > 0 ? (
        <div className="mt-2 space-y-1">
          {data.connectedOutputs.map((entry) => (
            <div key={entry} className="flex items-center gap-1.5">
              <span className="w-1.5 h-1.5 rounded-full bg-red-500 shrink-0" />
              <span className="text-[9px] font-mono text-muted-foreground truncate">{entry}</span>
            </div>
          ))}
        </div>
      ) : (
        <p className="text-[9px] text-muted-foreground mt-1">Connect task outputs here to define workflow output</p>
      )}
    </div>
  );
}

/* ── Task Node ────────────────────────────────────────── */
export function DesignerTaskNode({ id, data, selected }: {
  id: string;
  data: {
    label: string; taskType: string; refName: string; stepNumber: number;
    inputKeys: string[]; outputKeys: string[];
    branchMeta?: { caseCount?: number; caseNames?: string[]; expression?: string; condition?: string };
  };
  selected?: boolean;
}) {
  const { editTask, deleteTask } = useContext(DesignerActionsContext);
  const color = TASK_TYPE_COLORS[data.taskType] ?? "#9ca3af";
  const inputKeys: string[] = data.inputKeys ?? [];
  const outputKeys: string[] = data.outputKeys ?? [];
  return (
    <div className={`rounded-xl min-w-56 shadow-md transition-shadow hover:shadow-lg ${selected ? "ring-2 ring-primary" : ""}`}
      style={{ border: `2px solid ${color}`, background: "var(--color-card, #fff)" }}>
      <Handle type="target" position={Position.Top} id="flow-in"
        className="w-2.5! h-2.5! bg-gray-400! border! border-background! rounded-full!" />
      {/* Header */}
      <div className="px-3 py-1.5 flex items-center gap-2 border-b border-border/30"
        style={{ background: `${color}15`, borderRadius: "10px 10px 0 0" }}>
        <span className="w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold text-white shrink-0"
          style={{ backgroundColor: color }}>{data.stepNumber}</span>
        <span className="text-[10px] font-bold tracking-wide uppercase" style={{ color }}>
          {data.taskType.replace(/_/g, " ")}
        </span>
        <div className="ml-auto flex gap-1">
          <button onClick={() => editTask(id)} className="text-muted-foreground hover:text-foreground p-0.5 rounded hover:bg-muted" title="Edit">
            <Code className="h-3 w-3" />
          </button>
          <button onClick={() => deleteTask(id)} className="text-muted-foreground hover:text-destructive p-0.5 rounded hover:bg-muted" title="Delete">
            <Trash2 className="h-3 w-3" />
          </button>
        </div>
      </div>
      {/* Body: inputs & outputs */}
      <div className="px-3 py-2">
        <div className="text-xs font-semibold truncate max-w-48 mb-1.5">{data.label}</div>
        <div className="flex gap-4">
          <div className="flex-1 min-w-0 space-y-1">
            {inputKeys.length > 0 && (<>
              <div className="text-[7px] font-bold text-blue-500 uppercase tracking-wider">Inputs</div>
              {inputKeys.slice(0, 8).map((k) => (
                <div key={k} className="flex items-center gap-1 relative">
                  <Handle type="target" position={Position.Left} id={`in-${k}`}
                    style={{ top: "auto", left: -8, position: "absolute" }}
                    className="w-2! h-2! bg-blue-500! border! border-background! rounded-full!" />
                  <span className="text-[9px] font-mono text-blue-600 dark:text-blue-400 truncate ml-0.5">{k}</span>
                </div>
              ))}
              {inputKeys.length > 8 && <span className="text-[8px] text-muted-foreground">+{inputKeys.length - 8} more</span>}
            </>)}
          </div>
          <div className="flex-1 min-w-0 space-y-1">
            {outputKeys.length > 0 ? (<>
              <div className="text-[7px] font-bold text-emerald-500 uppercase tracking-wider text-right">Outputs</div>
              {outputKeys.slice(0, 8).map((k) => (
                <div key={k} className="flex items-center justify-end gap-1 relative">
                  <span className="text-[9px] font-mono text-emerald-600 dark:text-emerald-400 truncate mr-0.5">{k}</span>
                  <Handle type="source" position={Position.Right} id={`out-${k}`}
                    style={{ top: "auto", right: -8, position: "absolute" }}
                    className="w-2! h-2! bg-emerald-500! border! border-background! rounded-full!" />
                </div>
              ))}
              {outputKeys.length > 8 && <span className="text-[8px] text-muted-foreground float-right">+{outputKeys.length - 8} more</span>}
            </>) : (
              <div className="text-right">
                <div className="text-[7px] font-bold text-emerald-500 uppercase tracking-wider">Output</div>
                <div className="flex items-center justify-end gap-1 relative">
                  <span className="text-[9px] font-mono text-emerald-600 dark:text-emerald-400">result</span>
                  <Handle type="source" position={Position.Right} id="out-result"
                    style={{ top: "auto", right: -8, position: "absolute" }}
                    className="w-2! h-2! bg-emerald-500! border! border-background! rounded-full!" />
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
      {/* Decision/Switch branch handles */}
      {(data.taskType === "DECISION" || data.taskType === "SWITCH") && (
        <div className="border-t border-border/30" style={{ background: `${color}08` }}>
          <div className="px-3 py-1.5 flex items-center gap-1.5">
            <GitBranch className="h-3 w-3 shrink-0" style={{ color }} />
            <span className="text-[9px] font-semibold" style={{ color }}>
              {data.branchMeta?.caseCount ? `${data.branchMeta.caseCount} case${data.branchMeta.caseCount !== 1 ? "s" : ""}` : "No cases"}
            </span>
            {data.branchMeta?.expression && (
              <span className="text-[8px] text-muted-foreground font-mono truncate ml-auto max-w-24">{data.branchMeta.expression}</span>
            )}
          </div>
          {data.branchMeta?.caseNames && data.branchMeta.caseNames.length > 0 ? (
            <div className="pb-1">
              {data.branchMeta.caseNames.map((caseName) => (
                <div key={caseName} className="flex items-center gap-2 pl-3 pr-1 py-1.5 relative"
                  style={{ borderTop: `1px dashed ${color}30` }}>
                  <span className="w-4 h-4 rounded flex items-center justify-center shrink-0"
                    style={{ backgroundColor: `${color}20` }}>
                    <GitBranch className="h-2.5 w-2.5" style={{ color }} />
                  </span>
                  <span className="text-[10px] font-mono font-semibold" style={{ color }}>{caseName}</span>
                  <div className="flex-1 h-px mx-1"
                    style={{ backgroundImage: `linear-gradient(to right, ${color}60, ${color})` }} />
                  <span className="text-[9px] mr-2" style={{ color }}>→</span>
                  <Handle type="source" position={Position.Right} id={`case-${caseName}`}
                    style={{ top: "auto", right: -8, position: "absolute", backgroundColor: color }}
                    className="w-3! h-3! border-2! border-background! rounded-full!" />
                </div>
              ))}
            </div>
          ) : (
            <p className="text-[9px] text-muted-foreground italic px-3 pb-1.5">Add cases to create branch paths</p>
          )}
        </div>
      )}
      {/* Fork info */}
      {(data.taskType === "FORK_JOIN" || data.taskType === "DYNAMIC_FORK_JOIN") && (
        <div className="px-3 py-1.5 border-t border-border/30 flex items-center gap-1.5" style={{ background: `${color}08` }}>
          <GitFork className="h-3 w-3 shrink-0" style={{ color }} />
          <span className="text-[9px]" style={{ color }}>
            {data.taskType === "DYNAMIC_FORK_JOIN" ? "Dynamic parallel" : "Parallel branches"}
          </span>
        </div>
      )}
      {/* Do While info */}
      {data.taskType === "DO_WHILE" && (
        <div className="px-3 py-1.5 border-t border-border/30 flex items-center gap-1.5" style={{ background: `${color}08` }}>
          <Repeat className="h-3 w-3 shrink-0" style={{ color }} />
          <span className="text-[9px]" style={{ color }}>Loop</span>
          {data.branchMeta?.condition && (
            <span className="text-[8px] text-muted-foreground font-mono truncate ml-auto max-w-24">{data.branchMeta.condition}</span>
          )}
        </div>
      )}
      <Handle type="source" position={Position.Bottom} id="flow-out"
        className="w-2.5! h-2.5! bg-gray-400! border! border-background! rounded-full!" />
    </div>
  );
}
