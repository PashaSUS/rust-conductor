import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { metadataApi, type WorkflowTask } from "@/api/conductor";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { useThemeText } from "@/components/ThemeContext";
import { GitBranch, ArrowRight, Inbox } from "lucide-react";

/** Recursively collect all sub-workflow references from a task list */
function collectSubWorkflows(tasks: WorkflowTask[]): { name: string; version?: number }[] {
  const refs: { name: string; version?: number }[] = [];
  for (const task of tasks) {
    if (task.subWorkflowParam?.name) {
      refs.push({ name: task.subWorkflowParam.name, version: task.subWorkflowParam.version });
    }
    if (task.forkTasks) {
      for (const branch of task.forkTasks) {
        refs.push(...collectSubWorkflows(branch));
      }
    }
    if (task.decisionCases) {
      for (const caseKey of Object.keys(task.decisionCases)) {
        refs.push(...collectSubWorkflows(task.decisionCases[caseKey]));
      }
    }
    if (task.defaultCase) {
      refs.push(...collectSubWorkflows(task.defaultCase));
    }
    if (task.loopOver) {
      refs.push(...collectSubWorkflows(task.loopOver));
    }
  }
  return refs;
}

interface DependencyEdge {
  from: string;
  fromVersion: number;
  to: string;
  toVersion?: number;
}

export default function WorkflowDependencyGraph() {
  const t = useThemeText();

  const { data: defs, isLoading } = useQuery({
    queryKey: ["workflow-defs"],
    queryFn: metadataApi.listWorkflowDefs,
  });

  const { edges, nodeSet, inDegree, outDegree } = useMemo(() => {
    if (!defs) return { edges: [], nodeSet: new Set<string>(), inDegree: {} as Record<string, number>, outDegree: {} as Record<string, number> };

    const edges: DependencyEdge[] = [];
    const nodeSet = new Set<string>();
    const inDegree: Record<string, number> = {};
    const outDegree: Record<string, number> = {};

    for (const def of defs) {
      nodeSet.add(def.name);
      if (!outDegree[def.name]) outDegree[def.name] = 0;
      if (!inDegree[def.name]) inDegree[def.name] = 0;

      const refs = collectSubWorkflows(def.tasks);
      const seen = new Set<string>();
      for (const ref of refs) {
        const key = `${def.name}->${ref.name}`;
        if (seen.has(key)) continue;
        seen.add(key);

        edges.push({ from: def.name, fromVersion: def.version, to: ref.name, toVersion: ref.version });
        nodeSet.add(ref.name);
        outDegree[def.name] = (outDegree[def.name] ?? 0) + 1;
        inDegree[ref.name] = (inDegree[ref.name] ?? 0) + 1;
        if (!inDegree[def.name]) inDegree[def.name] = 0;
        if (!outDegree[ref.name]) outDegree[ref.name] = outDegree[ref.name] ?? 0;
      }
    }

    return { edges, nodeSet, inDegree, outDegree };
  }, [defs]);

  // Group edges by source for display
  const edgesBySource = useMemo(() => {
    const map: Record<string, DependencyEdge[]> = {};
    for (const edge of edges) {
      if (!map[edge.from]) map[edge.from] = [];
      map[edge.from].push(edge);
    }
    return map;
  }, [edges]);

  // Find root nodes (no incoming edges) and leaf nodes (no outgoing edges)
  const roots = [...nodeSet].filter((n) => (inDegree[n] ?? 0) === 0 && (outDegree[n] ?? 0) > 0);
  const leaves = [...nodeSet].filter((n) => (outDegree[n] ?? 0) === 0 && (inDegree[n] ?? 0) > 0);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">{t.dependencyGraph}</h2>
          <p className="text-muted-foreground text-sm">{t.dependencyGraphDesc}</p>
        </div>
        <div className="flex items-center gap-2">
          <Badge variant="secondary" className="gap-1">
            <GitBranch className="h-3 w-3" />
            {edges.length} dependencies
          </Badge>
        </div>
      </div>

      {isLoading ? (
        <p className="text-muted-foreground text-sm">{t.loading}</p>
      ) : edges.length === 0 ? (
        <Card>
          <CardContent className="py-8 text-center">
            <Inbox className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
            <p className="text-muted-foreground text-sm">{t.noDependencies}</p>
          </CardContent>
        </Card>
      ) : (
        <>
          {/* Visual dependency graph */}
          <Card>
            <CardHeader>
              <CardTitle className="text-sm font-medium">{t.dependencyGraph}</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {/* Root workflows */}
                {roots.length > 0 && (
                  <div>
                    <p className="text-xs font-medium text-muted-foreground mb-2">Root Workflows (no parent)</p>
                    <div className="flex flex-wrap gap-2">
                      {roots.map((name) => (
                        <Badge key={name} variant="default" className="bg-blue-500/15 text-blue-600 border-blue-500/20">
                          {name}
                        </Badge>
                      ))}
                    </div>
                  </div>
                )}

                {/* Dependency tree */}
                <div className="space-y-2">
                  {Object.entries(edgesBySource)
                    .sort(([a], [b]) => a.localeCompare(b))
                    .map(([source, deps]) => (
                      <div key={source} className="rounded-lg border p-3">
                        <div className="flex items-start gap-3">
                          <div className="shrink-0">
                            <Badge variant="secondary" className="font-mono text-xs">
                              {source}
                            </Badge>
                          </div>
                          <ArrowRight className="h-4 w-4 text-muted-foreground mt-0.5 shrink-0" />
                          <div className="flex flex-wrap gap-2">
                            {deps.map((dep) => (
                              <Badge
                                key={`${dep.from}-${dep.to}`}
                                variant="outline"
                                className="font-mono text-xs"
                              >
                                {dep.to}
                                {dep.toVersion !== undefined && (
                                  <span className="text-muted-foreground ml-1">v{dep.toVersion}</span>
                                )}
                              </Badge>
                            ))}
                          </div>
                        </div>
                      </div>
                    ))}
                </div>

                {/* Leaf workflows */}
                {leaves.length > 0 && (
                  <div>
                    <p className="text-xs font-medium text-muted-foreground mb-2">Leaf Workflows (called but don't call others)</p>
                    <div className="flex flex-wrap gap-2">
                      {leaves.map((name) => (
                        <Badge key={name} variant="default" className="bg-emerald-500/15 text-emerald-600 border-emerald-500/20">
                          {name}
                        </Badge>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </CardContent>
          </Card>

          {/* Summary table */}
          <Card>
            <CardHeader>
              <CardTitle className="text-sm font-medium">Workflow Connectivity</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-2">
                {[...nodeSet]
                  .filter((n) => (inDegree[n] ?? 0) > 0 || (outDegree[n] ?? 0) > 0)
                  .sort((a, b) => a.localeCompare(b))
                  .map((name) => (
                    <div key={name} className="rounded-lg border p-2 text-xs">
                      <span className="font-medium block truncate">{name}</span>
                      <div className="flex gap-3 mt-1 text-muted-foreground">
                        <span>↗ {outDegree[name] ?? 0} calls</span>
                        <span>↙ {inDegree[name] ?? 0} called by</span>
                      </div>
                    </div>
                  ))}
              </div>
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}
