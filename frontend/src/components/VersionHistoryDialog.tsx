import { useState, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { metadataApi } from "@/api/conductor";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useThemeText } from "@/components/ThemeContext";

interface Props {
  workflowName: string;
  currentVersion: number;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** Compute a simple line-by-line diff between two strings */
function computeDiff(oldStr: string, newStr: string) {
  const oldLines = oldStr.split("\n");
  const newLines = newStr.split("\n");

  // Simple LCS-based diff
  const m = oldLines.length;
  const n = newLines.length;
  const dp: number[][] = Array.from({ length: m + 1 }, () => Array(n + 1).fill(0));

  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      dp[i][j] = oldLines[i - 1] === newLines[j - 1]
        ? dp[i - 1][j - 1] + 1
        : Math.max(dp[i - 1][j], dp[i][j - 1]);
    }
  }

  // Backtrack to produce diff
  const diff: { type: "same" | "added" | "removed"; text: string }[] = [];
  let i = m, j = n;
  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && oldLines[i - 1] === newLines[j - 1]) {
      diff.push({ type: "same", text: oldLines[i - 1] });
      i--; j--;
    } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      diff.push({ type: "added", text: newLines[j - 1] });
      j--;
    } else {
      diff.push({ type: "removed", text: oldLines[i - 1] });
      i--;
    }
  }
  diff.reverse();
  return diff;
}

export function VersionHistoryDialog({ workflowName, currentVersion, open, onOpenChange }: Props) {
  const t = useThemeText();
  const [leftVersion, setLeftVersion] = useState<number | null>(null);
  const [rightVersion, setRightVersion] = useState<number | null>(null);

  const { data: allDefs } = useQuery({
    queryKey: ["workflow-defs"],
    queryFn: metadataApi.listWorkflowDefs,
    enabled: open,
  });

  const versions = useMemo(() => {
    if (!allDefs) return [];
    return allDefs
      .filter((d) => d.name === workflowName)
      .sort((a, b) => b.version - a.version);
  }, [allDefs, workflowName]);

  // Auto-select versions for comparison when versions load
  const effectiveLeft = leftVersion ?? (versions.length >= 2 ? versions[1].version : null);
  const effectiveRight = rightVersion ?? (versions.length >= 1 ? versions[0].version : null);

  const leftDef = versions.find((v) => v.version === effectiveLeft);
  const rightDef = versions.find((v) => v.version === effectiveRight);

  const diff = useMemo(() => {
    if (!leftDef || !rightDef) return null;
    const oldJson = JSON.stringify(leftDef, null, 2);
    const newJson = JSON.stringify(rightDef, null, 2);
    return computeDiff(oldJson, newJson);
  }, [leftDef, rightDef]);

  const addedCount = diff?.filter((l) => l.type === "added").length ?? 0;
  const removedCount = diff?.filter((l) => l.type === "removed").length ?? 0;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>{t.versionHistory}: {workflowName}</DialogTitle>
        </DialogHeader>

        {versions.length <= 1 ? (
          <p className="text-muted-foreground text-sm py-4">{t.noOtherVersions}</p>
        ) : (
          <div className="flex flex-col gap-4 min-h-0 flex-1">
            {/* Version selectors */}
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-2">
                <span className="text-sm text-muted-foreground whitespace-nowrap">v</span>
                <Select
                  value={String(effectiveLeft ?? "")}
                  onValueChange={(v) => setLeftVersion(Number(v))}
                >
                  <SelectTrigger className="w-24">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {versions.map((v) => (
                      <SelectItem key={v.version} value={String(v.version)}>
                        v{v.version}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <span className="text-muted-foreground">→</span>
              <div className="flex items-center gap-2">
                <span className="text-sm text-muted-foreground whitespace-nowrap">v</span>
                <Select
                  value={String(effectiveRight ?? "")}
                  onValueChange={(v) => setRightVersion(Number(v))}
                >
                  <SelectTrigger className="w-24">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {versions.map((v) => (
                      <SelectItem key={v.version} value={String(v.version)}>
                        v{v.version}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              {diff && (
                <div className="flex items-center gap-2 ml-auto">
                  <Badge variant="default" className="bg-emerald-500/15 text-emerald-600 border-emerald-500/20">
                    +{addedCount} {t.added}
                  </Badge>
                  <Badge variant="default" className="bg-red-500/15 text-red-600 border-red-500/20">
                    -{removedCount} {t.removed}
                  </Badge>
                </div>
              )}
            </div>

            {/* Diff view */}
            <ScrollArea className="flex-1 min-h-0 rounded-md border">
              {diff ? (
                addedCount === 0 && removedCount === 0 ? (
                  <p className="text-muted-foreground text-sm p-4">{t.noDifferences}</p>
                ) : (
                  <pre className="text-xs p-4 leading-relaxed">
                    {diff.map((line, i) => (
                      <div
                        key={i}
                        className={
                          line.type === "added"
                            ? "bg-emerald-500/10 text-emerald-700 dark:text-emerald-400"
                            : line.type === "removed"
                              ? "bg-red-500/10 text-red-700 dark:text-red-400"
                              : ""
                        }
                      >
                        <span className="inline-block w-5 text-right mr-2 text-muted-foreground select-none">
                          {line.type === "added" ? "+" : line.type === "removed" ? "-" : " "}
                        </span>
                        {line.text}
                      </div>
                    ))}
                  </pre>
                )
              ) : (
                <p className="text-muted-foreground text-sm p-4">{t.compareVersions}</p>
              )}
            </ScrollArea>

            {/* Version list */}
            <div className="border-t pt-3">
              <p className="text-xs text-muted-foreground mb-2">{t.versionHistory}</p>
              <div className="flex flex-wrap gap-2">
                {versions.map((v) => (
                  <Badge
                    key={v.version}
                    variant={v.version === currentVersion ? "default" : "secondary"}
                    className="cursor-default"
                  >
                    v{v.version} — {v.tasks.length} {t.tasksTab}
                    {v.description ? ` — ${v.description}` : ""}
                  </Badge>
                ))}
              </div>
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
