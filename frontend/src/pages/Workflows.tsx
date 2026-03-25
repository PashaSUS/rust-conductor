import { useState, useMemo, useCallback } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate, useSearchParams } from "react-router";
import { workflowApi, metadataApi, type SearchParams } from "@/api/conductor";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { SearchableSelect } from "@/components/SearchableSelect";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Search, XCircle, Pause, Play, RotateCcw, Plus, RefreshCw } from "lucide-react";
import { CopyButton } from "@/components/CopyButton";
import { StartWorkflowDialog } from "@/components/StartWorkflowDialog";
import { StatusBadge } from "@/components/StatusBadge";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { RelativeTime } from "@/components/RelativeTime";
import { PaginationControls } from "@/components/PaginationControls";
import { useThemeText } from "@/components/ThemeContext";
import { useWorkflowMutations, useBulkWorkflowMutations } from "@/hooks/useWorkflowMutations";

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
  const t = useThemeText();
  const { params, setParams } = useFilterParams();
  const [search, setSearch] = useState(params.freeText ?? "");
  const [workflowName, setWorkflowName] = useState(params.workflowType ?? "");
  const [autoRefresh, setAutoRefresh] = useState(false);
  const [startOpen, setStartOpen] = useState(false);
  const [terminateTarget, setTerminateTarget] = useState<string | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  const { pauseMut, resumeMut, terminateMut, restartMut } = useWorkflowMutations([["workflow-search"]]);
  const { bulkPauseMut, bulkResumeMut, bulkRetryMut, bulkRestartMut, bulkTerminateMut } =
    useBulkWorkflowMutations([["workflow-search"]], () => setSelectedIds(new Set()));

  const { data: workflowDefs } = useQuery({
    queryKey: ["workflow-defs-list"],
    queryFn: () => metadataApi.listWorkflowDefs(),
  });

  const { data, isLoading } = useQuery({
    queryKey: ["workflow-search", params],
    queryFn: () => workflowApi.search(params),
    refetchInterval: autoRefresh ? 5000 : false,
  });

  const toggleSelect = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };
  const toggleAll = () => {
    const ids = (data?.results ?? []).map((w) => w.workflowId);
    setSelectedIds((prev) => prev.size === ids.length ? new Set() : new Set(ids));
  };

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
    <div className="flex flex-col gap-4 h-[calc(100vh-8rem)]">
      <div className="flex items-center justify-between shrink-0">
        <h2 className="text-2xl font-bold tracking-tight">{t.executionsTitle}</h2>
        <div className="flex items-center gap-2">
          <Button
            variant={autoRefresh ? "default" : "outline"}
            size="sm"
            onClick={() => setAutoRefresh(!autoRefresh)}
            title={autoRefresh ? t.autoRefreshOn : t.autoRefreshOff}
          >
            <RefreshCw className={`h-3 w-3 mr-1 ${autoRefresh ? "animate-spin" : ""}`} />
            {autoRefresh ? t.live : t.autoRefresh}
          </Button>
          <Button size="sm" onClick={() => setStartOpen(true)}>
            <Plus className="h-4 w-4 mr-1" />
            {t.startWorkflow}
          </Button>
        </div>
      </div>

      <Card className="shrink-0">
        <CardHeader className="pb-3">
          <CardTitle className="text-sm font-medium">{t.searchAndFilter}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex gap-2">
            <SearchableSelect
              options={uniqueNames.map((n) => ({ label: n, value: n }))}
              value={workflowName}
              onChange={(v) => {
                setWorkflowName(v);
                setParams((p) => ({ ...p, workflowType: v || undefined, start: 0 }));
              }}
              placeholder={t.allWorkflowTypes}
              searchPlaceholder={t.searchPlaceholder}
              className="w-55"
            />
            <Input
              placeholder={t.searchPlaceholder}
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
                {s || t.allStatuses}
              </Button>
            ))}
          </div>
        </CardContent>
      </Card>

      <Card className="flex-1 min-h-0 flex flex-col">
        <CardContent className="pt-6 flex-1 overflow-auto">
          {isLoading ? (
            <p className="text-muted-foreground text-sm">{t.loading}</p>
          ) : (
            <>
              <p className="text-xs text-muted-foreground mb-3">
                {data?.totalHits ?? 0} {t.results}
                {(data?.totalHits ?? 0) > 0 && (
                  <span className="ml-2">
                    ({t.showing} {(params.start ?? 0) + 1}–{Math.min((params.start ?? 0) + PAGE_SIZE, data?.totalHits ?? 0)})
                  </span>
                )}
              </p>
              {selectedIds.size > 0 && (
                <div className="flex items-center gap-2 mb-3 p-2 rounded-md bg-muted">
                  <span className="text-xs font-medium">{selectedIds.size} {t.selectedCount}</span>
                  <Button size="sm" variant="outline" onClick={() => bulkPauseMut.mutate([...selectedIds])}>{t.bulkPause}</Button>
                  <Button size="sm" variant="outline" onClick={() => bulkResumeMut.mutate([...selectedIds])}>{t.bulkResume}</Button>
                  <Button size="sm" variant="outline" onClick={() => bulkRetryMut.mutate([...selectedIds])}>{t.bulkRetry}</Button>
                  <Button size="sm" variant="outline" onClick={() => bulkRestartMut.mutate([...selectedIds])}>{t.bulkRestart}</Button>
                  <Button size="sm" variant="destructive" onClick={() => bulkTerminateMut.mutate([...selectedIds])}>{t.bulkTerminate}</Button>
                </div>
              )}
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-8">
                      <Checkbox
                        checked={selectedIds.size > 0 && selectedIds.size === (data?.results ?? []).length}
                        onCheckedChange={toggleAll}
                      />
                    </TableHead>
                    <TableHead>{t.workflow}</TableHead>
                    <TableHead>{t.status}</TableHead>
                    <TableHead>{t.started}</TableHead>
                    <TableHead>{t.ended}</TableHead>
                    <TableHead className="text-right">{t.actions}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {(data?.results ?? []).map((wf) => (
                    <TableRow
                      key={wf.workflowId}
                      className="cursor-pointer"
                      onClick={() => navigate(`/executions/${wf.workflowId}`)}
                    >
                      <TableCell onClick={(e) => e.stopPropagation()}>
                        <Checkbox
                          checked={selectedIds.has(wf.workflowId)}
                          onCheckedChange={() => toggleSelect(wf.workflowId)}
                        />
                      </TableCell>
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
                      <TableCell className="text-xs"><RelativeTime value={wf.startTime} /></TableCell>
                      <TableCell className="text-xs"><RelativeTime value={wf.endTime} /></TableCell>
                      <TableCell className="text-right" onClick={(e) => e.stopPropagation()}>
                        <div className="flex gap-1 justify-end">
                          {wf.status === "RUNNING" && (
                            <>
                              <Button variant="ghost" size="icon" onClick={() => pauseMut.mutate(wf.workflowId)} title={t.pause}>
                                <Pause className="h-3 w-3" />
                              </Button>
                              <Button variant="ghost" size="icon" onClick={() => setTerminateTarget(wf.workflowId)} title={t.terminate}>
                                <XCircle className="h-3 w-3" />
                              </Button>
                            </>
                          )}
                          {wf.status === "PAUSED" && (
                            <Button variant="ghost" size="icon" onClick={() => resumeMut.mutate(wf.workflowId)} title={t.resume}>
                              <Play className="h-3 w-3" />
                            </Button>
                          )}
                          {(wf.status === "FAILED" || wf.status === "TERMINATED" || wf.status === "COMPLETED") && (
                            <Button variant="ghost" size="icon" onClick={() => restartMut.mutate(wf.workflowId)} title={t.restart}>
                              <RotateCcw className="h-3 w-3" />
                            </Button>
                          )}
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </>
          )}
        </CardContent>
        {/* Pagination footer — outside scroll area */}
        {!isLoading && (data?.totalHits ?? 0) > PAGE_SIZE && (
          <div className="border-t px-6 py-2 shrink-0">
            <PaginationControls
              page={Math.floor((params.start ?? 0) / PAGE_SIZE)}
              totalPages={Math.ceil((data?.totalHits ?? 0) / PAGE_SIZE)}
              canPrev={(params.start ?? 0) > 0}
              canNext={(params.start ?? 0) + PAGE_SIZE < (data?.totalHits ?? 0)}
              onPrev={() => setParams((p) => ({ ...p, start: Math.max((p.start ?? 0) - PAGE_SIZE, 0) }))}
              onNext={() => setParams((p) => ({ ...p, start: (p.start ?? 0) + PAGE_SIZE }))}
              rangeLabel={`${(params.start ?? 0) + 1}\u2013${Math.min((params.start ?? 0) + PAGE_SIZE, data?.totalHits ?? 0)}`}
              totalItems={data?.totalHits}
            />
          </div>
        )}
      </Card>

      {/* Start workflow dialog */}
      <StartWorkflowDialog
        open={startOpen}
        onOpenChange={setStartOpen}
      />

      {/* Terminate confirmation */}
      <ConfirmDialog
        open={!!terminateTarget}
        onOpenChange={(open) => { if (!open) setTerminateTarget(null); }}
        title={t.terminateWorkflow}
        description={t.terminateConfirm}
        confirmLabel={t.terminate}
        variant="destructive"
        onConfirm={async () => {
          if (terminateTarget) {
            await terminateMut.mutateAsync(terminateTarget);
          }
        }}
      />
    </div>
  );
}
