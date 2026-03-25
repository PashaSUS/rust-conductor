import { useState, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { metadataApi } from "@/api/conductor";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";

type DiffLine = {
  type: "same" | "added" | "removed";
  lineNum: { left?: number; right?: number };
  text: string;
};

function computeDiff(a: string, b: string): DiffLine[] {
  const linesA = a.split("\n");
  const linesB = b.split("\n");
  const result: DiffLine[] = [];

  // Simple LCS-based diff
  const m = linesA.length;
  const n = linesB.length;
  const dp: number[][] = Array.from({ length: m + 1 }, () => Array(n + 1).fill(0));

  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (linesA[i - 1] === linesB[j - 1]) {
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }

  // Backtrack to build diff
  let i = m, j = n;
  const temp: DiffLine[] = [];
  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && linesA[i - 1] === linesB[j - 1]) {
      temp.push({ type: "same", lineNum: { left: i, right: j }, text: linesA[i - 1] });
      i--; j--;
    } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      temp.push({ type: "added", lineNum: { right: j }, text: linesB[j - 1] });
      j--;
    } else {
      temp.push({ type: "removed", lineNum: { left: i }, text: linesA[i - 1] });
      i--;
    }
  }
  temp.reverse();
  result.push(...temp);
  return result;
}

/**
 * #176 — Workflow definition visual diff.
 * Side-by-side comparison of two workflow definition versions with
 * syntax highlighting and change markers.
 */
export default function WorkflowDiff() {
  const [selectedWorkflow, setSelectedWorkflow] = useState<string>("");
  const [leftVersion, setLeftVersion] = useState<string>("");
  const [rightVersion, setRightVersion] = useState<string>("");

  const { data: allDefs } = useQuery({
    queryKey: ["workflowDefs"],
    queryFn: metadataApi.listWorkflowDefs,
  });

  // Group by name → versions
  const workflowGroups = useMemo(() => {
    const map = new Map<string, number[]>();
    if (allDefs) {
      for (const d of allDefs) {
        const existing = map.get(d.name) ?? [];
        existing.push(d.version);
        map.set(d.name, existing);
      }
      for (const versions of map.values()) versions.sort((a, b) => a - b);
    }
    return map;
  }, [allDefs]);

  const versions = selectedWorkflow ? workflowGroups.get(selectedWorkflow) ?? [] : [];

  const { data: leftDef } = useQuery({
    queryKey: ["workflowDef", selectedWorkflow, leftVersion],
    queryFn: () => metadataApi.getWorkflowDef(selectedWorkflow, Number(leftVersion)),
    enabled: !!selectedWorkflow && !!leftVersion,
  });

  const { data: rightDef } = useQuery({
    queryKey: ["workflowDef", selectedWorkflow, rightVersion],
    queryFn: () => metadataApi.getWorkflowDef(selectedWorkflow, Number(rightVersion)),
    enabled: !!selectedWorkflow && !!rightVersion,
  });

  const diffLines = useMemo(() => {
    if (!leftDef || !rightDef) return [];
    const leftJson = JSON.stringify(leftDef, null, 2);
    const rightJson = JSON.stringify(rightDef, null, 2);
    return computeDiff(leftJson, rightJson);
  }, [leftDef, rightDef]);

  const stats = useMemo(() => {
    const added = diffLines.filter((l) => l.type === "added").length;
    const removed = diffLines.filter((l) => l.type === "removed").length;
    const same = diffLines.filter((l) => l.type === "same").length;
    return { added, removed, same };
  }, [diffLines]);

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-bold tracking-tight">Workflow Definition Diff</h2>

      <div className="flex gap-3 items-end flex-wrap">
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">Workflow</label>
          <Select value={selectedWorkflow} onValueChange={(v) => { setSelectedWorkflow(v); setLeftVersion(""); setRightVersion(""); }}>
            <SelectTrigger className="w-56">
              <SelectValue placeholder="Select workflow..." />
            </SelectTrigger>
            <SelectContent>
              {Array.from(workflowGroups.keys()).map((name) => (
                <SelectItem key={name} value={name}>{name}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">Left Version</label>
          <Select value={leftVersion} onValueChange={setLeftVersion} disabled={!selectedWorkflow}>
            <SelectTrigger className="w-28">
              <SelectValue placeholder="v..." />
            </SelectTrigger>
            <SelectContent>
              {versions.map((v) => (
                <SelectItem key={v} value={String(v)}>v{v}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">Right Version</label>
          <Select value={rightVersion} onValueChange={setRightVersion} disabled={!selectedWorkflow}>
            <SelectTrigger className="w-28">
              <SelectValue placeholder="v..." />
            </SelectTrigger>
            <SelectContent>
              {versions.map((v) => (
                <SelectItem key={v} value={String(v)}>v{v}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {leftDef && rightDef && (
        <>
          {/* Stats bar */}
          <div className="flex gap-2">
            <Badge variant="outline" className="text-green-600">+{stats.added} added</Badge>
            <Badge variant="outline" className="text-red-600">-{stats.removed} removed</Badge>
            <Badge variant="secondary">{stats.same} unchanged</Badge>
          </div>

          {/* Diff view — true side-by-side */}
          <Card>
            <CardContent className="pt-4">
              <div className="overflow-auto max-h-150 font-mono text-xs rounded border">
                <div className="grid grid-cols-2 divide-x">
                  {/* Left column header */}
                  <div className="bg-muted text-muted-foreground px-2 py-1 text-center font-semibold sticky top-0 z-10 border-b">
                    v{leftVersion} (Left)
                  </div>
                  {/* Right column header */}
                  <div className="bg-muted text-muted-foreground px-2 py-1 text-center font-semibold sticky top-0 z-10 border-b">
                    v{rightVersion} (Right)
                  </div>
                  {/* Left column lines */}
                  <div className="overflow-auto">
                    {diffLines.map((line, idx) => {
                      if (line.type === "added") {
                        return (
                          <div key={idx} className="px-2 py-0.5 bg-muted text-transparent select-none min-h-[1.5em]">&nbsp;</div>
                        );
                      }
                      const bg = line.type === "removed" ? "bg-red-50 dark:bg-red-950" : "";
                      const textColor = line.type === "removed" ? "text-red-700 dark:text-red-400" : "";
                      return (
                        <div key={idx} className={`px-2 py-0.5 whitespace-pre ${bg} ${textColor} min-h-[1.5em]`}>
                          <span className="select-none mr-2 text-muted-foreground inline-block w-6 text-right">
                            {line.lineNum.left ?? ""}
                          </span>
                          <span className="select-none mr-1 text-muted-foreground">
                            {line.type === "removed" ? "-" : " "}
                          </span>
                          {line.text}
                        </div>
                      );
                    })}
                  </div>
                  {/* Right column lines */}
                  <div className="overflow-auto">
                    {diffLines.map((line, idx) => {
                      if (line.type === "removed") {
                        return (
                          <div key={idx} className="px-2 py-0.5 bg-muted text-transparent select-none min-h-[1.5em]">&nbsp;</div>
                        );
                      }
                      const bg = line.type === "added" ? "bg-green-50 dark:bg-green-950" : "";
                      const textColor = line.type === "added" ? "text-green-700 dark:text-green-400" : "";
                      return (
                        <div key={idx} className={`px-2 py-0.5 whitespace-pre ${bg} ${textColor} min-h-[1.5em]`}>
                          <span className="select-none mr-2 text-muted-foreground inline-block w-6 text-right">
                            {line.lineNum.right ?? ""}
                          </span>
                          <span className="select-none mr-1 text-muted-foreground">
                            {line.type === "added" ? "+" : " "}
                          </span>
                          {line.text}
                        </div>
                      );
                    })}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}
