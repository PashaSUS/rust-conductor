import { useState } from "react";
import { Badge } from "@/components/ui/badge";
import { ChevronDown, ChevronRight, Equal } from "lucide-react";
import type { TaskResult } from "@/api/conductor";

interface Props {
  leftTasks: TaskResult[];
  rightTasks: TaskResult[];
}

function stableStringify(data: unknown): string {
  return JSON.stringify(data, null, 2) ?? "null";
}

function computeLineDiff(a: string, b: string): Array<{ type: "same" | "added" | "removed"; text: string }> {
  const linesA = a.split("\n");
  const linesB = b.split("\n");
  const result: Array<{ type: "same" | "added" | "removed"; text: string }> = [];

  let ai = 0, bi = 0;
  while (ai < linesA.length || bi < linesB.length) {
    if (ai < linesA.length && bi < linesB.length && linesA[ai] === linesB[bi]) {
      result.push({ type: "same", text: linesA[ai] });
      ai++;
      bi++;
    } else if (ai < linesA.length && (bi >= linesB.length || linesA[ai] !== linesB[bi])) {
      result.push({ type: "removed", text: linesA[ai] });
      ai++;
    } else {
      result.push({ type: "added", text: linesB[bi] });
      bi++;
    }
  }
  return result;
}

export function TaskOutputDiff({ leftTasks, rightTasks }: Props) {
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  const allRefs = new Set<string>();
  leftTasks.forEach((t) => allRefs.add(t.referenceTaskName));
  rightTasks.forEach((t) => allRefs.add(t.referenceTaskName));

  const leftMap = new Map(leftTasks.map((t) => [t.referenceTaskName, t]));
  const rightMap = new Map(rightTasks.map((t) => [t.referenceTaskName, t]));

  const refs = [...allRefs];

  const toggle = (ref: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(ref)) next.delete(ref); else next.add(ref);
      return next;
    });
  };

  return (
    <div className="space-y-1">
      {refs.map((refName) => {
        const lt = leftMap.get(refName);
        const rt = rightMap.get(refName);
        const leftOut = stableStringify(lt?.outputData);
        const rightOut = stableStringify(rt?.outputData);
        const same = leftOut === rightOut;
        const isExpanded = expanded.has(refName);

        return (
          <div key={refName} className="border rounded-md">
            <button
              className="flex items-center gap-2 w-full px-3 py-2 hover:bg-muted transition-colors text-left"
              onClick={() => toggle(refName)}
            >
              {isExpanded ? <ChevronDown className="h-3 w-3 shrink-0" /> : <ChevronRight className="h-3 w-3 shrink-0" />}
              <span className="text-sm font-medium flex-1">{refName}</span>
              {same ? (
                <Badge variant="outline" className="text-[10px] gap-1">
                  <Equal className="h-3 w-3" /> identical
                </Badge>
              ) : (
                <Badge variant="secondary" className="text-[10px] gap-1 text-amber-600">
                  changed
                </Badge>
              )}
            </button>
            {isExpanded && !same && (
              <div className="border-t px-3 py-2 max-h-64 overflow-auto bg-muted">
                <div className="font-mono text-[11px] leading-relaxed">
                  {computeLineDiff(leftOut, rightOut).map((line, i) => (
                    <div
                      key={i}
                      className={
                        line.type === "added"
                          ? "bg-emerald-50 dark:bg-emerald-950 text-emerald-700 dark:text-emerald-400"
                          : line.type === "removed"
                          ? "bg-red-50 dark:bg-red-950 text-red-700 dark:text-red-400"
                          : "text-muted-foreground"
                      }
                    >
                      <span className="inline-block w-4 text-center text-muted-foreground">
                        {line.type === "added" ? "+" : line.type === "removed" ? "−" : " "}
                      </span>
                      {line.text}
                    </div>
                  ))}
                </div>
              </div>
            )}
            {isExpanded && same && (
              <div className="border-t px-3 py-2 text-xs text-muted-foreground">
                Outputs are identical.
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
