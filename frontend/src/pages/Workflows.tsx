import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { workflowApi, formatTs, type SearchParams } from "@/api/conductor";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Search, XCircle, Pause, Play, RotateCcw, ChevronLeft, ChevronRight } from "lucide-react";

const PAGE_SIZE = 25;

export default function Workflows() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [params, setParams] = useState<SearchParams>({ size: PAGE_SIZE, start: 0 });
  const [search, setSearch] = useState("");

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
    setParams((p) => ({ ...p, freeText: search || undefined, start: 0 }));
  };

  const statuses = ["", "RUNNING", "COMPLETED", "FAILED", "TERMINATED", "PAUSED", "TIMED_OUT"];

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-bold tracking-tight">Workflow Executions</h2>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm font-medium">Search & Filter</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex gap-2">
            <Input
              placeholder="Search workflows..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleSearch()}
              className="max-w-sm"
            />
            <Button variant="outline" size="sm" onClick={handleSearch}>
              <Search className="h-4 w-4" />
            </Button>
            <div className="flex gap-1 ml-4">
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
                        <div className="text-xs text-muted-foreground truncate max-w-xs">{wf.workflowId}</div>
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
