import { useState, useMemo, useCallback } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useSearchParams } from "react-router";
import { toast } from "sonner";
import { workflowApi, metadataApi, formatTs, type SearchParams } from "@/api/conductor";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Search, XCircle, Pause, Play, RotateCcw, ChevronLeft, ChevronRight } from "lucide-react";
import { CopyButton } from "@/components/CopyButton";

const PAGE_SIZE = 25;

function useFilterParams() {
  const [searchParams, setSearchParams] = useSearchParams();

  const params: SearchParams = useMemo(() => ({
    status: searchParams.get("status") || undefined,
    workflowType: searchParams.get("workflowType") || undefined,
    freeText: searchParams.get("freeText") || undefined,
    start: searchParams.has("start") ? Number(searchParams.get("start")) : 0,
    size: PAGE_SIZE,
  }), [searchParams]);

  const setParams = useCallback((updater: (prev: SearchParams) => SearchParams) => {
    setSearchParams((prev) => {
      const current: SearchParams = {
        status: prev.get("status") || undefined,
        workflowType: prev.get("workflowType") || undefined,
        freeText: prev.get("freeText") || undefined,
        start: prev.has("start") ? Number(prev.get("start")) : 0,
        size: PAGE_SIZE,
      };
      const next = updater(current);
      const qs = new URLSearchParams();
      if (next.status) qs.set("status", next.status);
      if (next.workflowType) qs.set("workflowType", next.workflowType);
      if (next.freeText) qs.set("freeText", next.freeText);
      if (next.start) qs.set("start", String(next.start));
      return qs;
    });
  }, [setSearchParams]);

  return { params, setParams };
}

export default function Workflows() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { params, setParams } = useFilterParams();
  const [search, setSearch] = useState(params.freeText ?? "");
  const [workflowName, setWorkflowName] = useState(params.workflowType ?? "");

  const { data: workflowDefs } = useQuery({
    queryKey: ["workflow-defs-list"],
    queryFn: () => metadataApi.listWorkflowDefs(),
  });

  const { data, isLoading } = useQuery({
    queryKey: ["workflow-search", params],
    queryFn: () => workflowApi.search(params),
  });

  const terminateMut = useMutation({
    mutationFn: (id: string) => workflowApi.terminate(id),
    onSuccess: () => {
      toast.success("Workflow terminated");
      queryClient.invalidateQueries({ queryKey: ["workflow-search"] });
    },
  });

  const pauseMut = useMutation({
    mutationFn: (id: string) => workflowApi.pause(id),
    onSuccess: () => {
      toast.success("Workflow paused");
      queryClient.invalidateQueries({ queryKey: ["workflow-search"] });
    },
  });

  const resumeMut = useMutation({
    mutationFn: (id: string) => workflowApi.resume(id),
    onSuccess: () => {
      toast.success("Workflow resumed");
      queryClient.invalidateQueries({ queryKey: ["workflow-search"] });
    },
  });

  const restartMut = useMutation({
    mutationFn: (id: string) => workflowApi.restart(id),
    onSuccess: () => {
      toast.success("Workflow restarted");
      queryClient.invalidateQueries({ queryKey: ["workflow-search"] });
    },
  });

  const handleSearch = () => {
    setParams((p) => ({
      ...p,
      freeText: search || undefined,
      workflowType: workflowName || undefined,
      start: 0,
    }));
  };

  const statuses = ["", "RUNNING", "COMPLETED", "FAILED", "TERMINATED", "PAUSED", "TIMED_OUT"];

  const uniqueNames = workflowDefs
    ? [...new Set(workflowDefs.map((d) => d.name))].sort()
    : [];

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-bold tracking-tight">Workflow Executions</h2>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm font-medium">Search & Filter</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex gap-2">
            <select
              value={workflowName}
              onChange={(e) => {
                setWorkflowName(e.target.value);
                setParams((p) => ({ ...p, workflowType: e.target.value || undefined, start: 0 }));
              }}
              className="h-9 rounded-md border border-input bg-background px-3 text-sm ring-offset-background focus:outline-none focus:ring-2 focus:ring-ring"
            >
              <option value="">All Workflow Types</option>
              {uniqueNames.map((n) => (
                <option key={n} value={n}>{n}</option>
              ))}
            </select>
            <Input
              placeholder="Search by name, ID, or correlation ID..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleSearch()}
              className="max-w-sm"
            />
            <Button variant="outline" size="sm" onClick={handleSearch}>
              <Search className="h-4 w-4" />
            </Button>
          </div>
          <div className="flex gap-1">
            {statuses.map((s) => (
              <Button
                key={s || "all"}
                variant={params.status === (s || undefined) ? "default" : "outline"}
                size="sm"
                onClick={() => setParams((p) => ({ ...p, status: s || undefined, start: 0 }))}
              >
                {s || "All"}
              </Button>
            ))}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="pt-6">
          {isLoading ? (
            <p className="text-muted-foreground text-sm">Loading...</p>
          ) : (
            <>
              <p className="text-xs text-muted-foreground mb-3">
                {data?.totalHits ?? 0} results
                {(data?.totalHits ?? 0) > 0 && (
                  <span className="ml-2">
                    (showing {(params.start ?? 0) + 1}–{Math.min((params.start ?? 0) + PAGE_SIZE, data?.totalHits ?? 0)})
                  </span>
                )}
              </p>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Workflow</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Started</TableHead>
                    <TableHead>Ended</TableHead>
                    <TableHead className="text-right">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {(data?.results ?? []).map((wf) => (
                    <TableRow
                      key={wf.workflowId}
                      className="cursor-pointer"
                      onClick={() => navigate(`/executions/${wf.workflowId}`)}
                    >
                      <TableCell>
                        <div>
                          <span className="font-medium">{wf.workflowType}</span>
                          <span className="text-muted-foreground text-xs ml-1">v{wf.version}</span>
                        </div>
                        <div className="text-xs text-muted-foreground truncate max-w-xs flex items-center gap-1">{wf.workflowId}<CopyButton value={wf.workflowId} /></div>
                      </TableCell>
                      <TableCell>
                        <StatusBadge status={wf.status} />
                      </TableCell>
                      <TableCell className="text-xs">{formatTs(wf.startTime)}</TableCell>
                      <TableCell className="text-xs">{formatTs(wf.endTime)}</TableCell>
                      <TableCell className="text-right" onClick={(e) => e.stopPropagation()}>
                        <div className="flex gap-1 justify-end">
                          {wf.status === "RUNNING" && (
                            <>
                              <Button variant="ghost" size="icon" onClick={() => pauseMut.mutate(wf.workflowId)} title="Pause">
                                <Pause className="h-3 w-3" />
                              </Button>
                              <Button variant="ghost" size="icon" onClick={() => terminateMut.mutate(wf.workflowId)} title="Terminate">
                                <XCircle className="h-3 w-3" />
                              </Button>
                            </>
                          )}
                          {wf.status === "PAUSED" && (
                            <Button variant="ghost" size="icon" onClick={() => resumeMut.mutate(wf.workflowId)} title="Resume">
                              <Play className="h-3 w-3" />
                            </Button>
                          )}
                          {(wf.status === "FAILED" || wf.status === "TERMINATED" || wf.status === "COMPLETED") && (
                            <Button variant="ghost" size="icon" onClick={() => restartMut.mutate(wf.workflowId)} title="Restart">
                              <RotateCcw className="h-3 w-3" />
                            </Button>
                          )}
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
              {/* Pagination */}
              {(data?.totalHits ?? 0) > PAGE_SIZE && (
                <div className="flex items-center justify-between mt-4">
                  <p className="text-xs text-muted-foreground">
                    Page {Math.floor((params.start ?? 0) / PAGE_SIZE) + 1} of{" "}
                    {Math.ceil((data?.totalHits ?? 0) / PAGE_SIZE)}
                  </p>
                  <div className="flex gap-1">
                    <Button
                      variant="outline"
                      size="sm"
                      disabled={(params.start ?? 0) === 0}
                      onClick={() => setParams((p) => ({ ...p, start: Math.max((p.start ?? 0) - PAGE_SIZE, 0) }))}
                    >
                      <ChevronLeft className="h-4 w-4 mr-1" />Prev
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      disabled={(params.start ?? 0) + PAGE_SIZE >= (data?.totalHits ?? 0)}
                      onClick={() => setParams((p) => ({ ...p, start: (p.start ?? 0) + PAGE_SIZE }))}
                    >
                      Next<ChevronRight className="h-4 w-4 ml-1" />
                    </Button>
                  </div>
                </div>
              )}
            </>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const variant =
    status === "COMPLETED" ? "success"
      : status === "RUNNING" ? "default"
      : status === "FAILED" || status === "TIMED_OUT" ? "destructive"
      : status === "PAUSED" ? "warning"
      : "secondary";
  return <Badge variant={variant as "default"}>{status}</Badge>;
}
