import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { metadataApi, type WorkflowDef } from "@/api/conductor";
import { usePagination } from "@/hooks/usePagination";
import { PaginationControls } from "@/components/PaginationControls";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Plus, Trash2, Eye, Play, Copy, Search, History, Pencil } from "lucide-react";
import WorkflowBuilder from "@/components/WorkflowBuilder";
import { StartWorkflowDialog } from "@/components/StartWorkflowDialog";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { CopyButton } from "@/components/CopyButton";
import { VersionHistoryDialog } from "@/components/VersionHistoryDialog";
import { useThemeText } from "@/components/ThemeContext";

export default function WorkflowDefs() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const t = useThemeText();
  const [viewDef, setViewDef] = useState<WorkflowDef | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [editDef, setEditDef] = useState<WorkflowDef | null>(null);
  const [startDef, setStartDef] = useState<WorkflowDef | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<{ name: string; version: number } | null>(null);
  const [searchTerm, setSearchTerm] = useState("");
  const [historyTarget, setHistoryTarget] = useState<{ name: string; version: number } | null>(null);
  const [searchAllVersions, setSearchAllVersions] = useState(false);

  const { data: defs, isLoading } = useQuery({
    queryKey: ["workflow-defs"],
    queryFn: metadataApi.listWorkflowDefs,
  });

  const createMut = useMutation({
    mutationFn: (def: WorkflowDef) => metadataApi.registerWorkflowDef(def),
    onSuccess: () => {
      toast.success(t.toastWorkflowDefSaved);
      queryClient.invalidateQueries({ queryKey: ["workflow-defs"] });
      setCreateOpen(false);
      setEditDef(null);
    },
    onError: (e) => toast.error(e.message),
  });

  const deleteMut = useMutation({
    mutationFn: ({ name, version }: { name: string; version: number }) =>
      metadataApi.deleteWorkflowDef(name, version),
    onSuccess: () => {
      toast.success(t.toastDeleted);
      queryClient.invalidateQueries({ queryKey: ["workflow-defs"] });
    },
  });

  const cloneDef = (def: WorkflowDef) => {
    const cloned: WorkflowDef = {
      ...JSON.parse(JSON.stringify(def)),
      name: `${def.name}_copy`,
      version: 1,
    };
    setEditDef(cloned);
  };

  /** Edit workflow — navigates to full-page editor, saves as next version */
  const editAsNextVersion = (def: WorkflowDef) => {
    navigate(`/definitions/create?clone=${encodeURIComponent(def.name)}&version=${def.version}`);
  };

  const filteredDefs = (defs ?? []).filter(
    (d) =>
      !searchTerm ||
      d.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      d.description?.toLowerCase().includes(searchTerm.toLowerCase())
  );

  // When not searching all versions, group by name and show only the latest version
  const displayDefs = searchAllVersions
    ? filteredDefs
    : Object.values(
        filteredDefs.reduce<Record<string, typeof filteredDefs[0]>>((acc, d) => {
          if (!acc[d.name] || d.version > acc[d.name].version) acc[d.name] = d;
          return acc;
        }, {})
      );

  const [pagedDefs, pagination] = usePagination(displayDefs);

  return (
    <div className="flex flex-col gap-4 h-[calc(100vh-8rem)]">
      <div className="flex items-center justify-between shrink-0">
        <h2 className="text-2xl font-bold tracking-tight">{t.workflowDefsTitle}</h2>
        <Button size="sm" onClick={() => navigate("/definitions/create")}>
          <Plus className="h-4 w-4 mr-1" />
          {t.newDefinition}
        </Button>
      </div>

      {/* Search bar */}
      <div className="flex items-center gap-2 shrink-0">
        <Search className="h-4 w-4 text-muted-foreground" />
        <Input
          placeholder={t.filterDefinitions}
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="max-w-xs"
        />
        <label className="flex items-center gap-1.5 text-xs text-muted-foreground cursor-pointer select-none">
          <input
            type="checkbox"
            checked={searchAllVersions}
            onChange={(e) => setSearchAllVersions(e.target.checked)}
            className="rounded border-muted-foreground"
          />
          {t.searchAllVersions}
        </label>
        {searchTerm && (
          <span className="text-xs text-muted-foreground">
            {displayDefs.length} {t.of} {defs?.length ?? 0}
          </span>
        )}
      </div>

      <Card className="flex-1 min-h-0 flex flex-col">
        <CardContent className="pt-6 flex-1 overflow-auto">
          {isLoading ? (
            <p className="text-muted-foreground text-sm">{t.loading}</p>
          ) : filteredDefs.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <p className="text-sm">
                {defs?.length === 0
                  ? t.noWorkflowDefs
                  : t.noDefsMatch}
              </p>
            </div>
          ) : (
            <>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{t.name}</TableHead>
                  <TableHead>{t.version}</TableHead>
                  <TableHead>{t.tasks}</TableHead>
                  <TableHead>{t.description}</TableHead>
                  <TableHead>{t.timeout}</TableHead>
                  <TableHead className="text-right">{t.actions}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {pagedDefs.map((def) => (
                  <TableRow key={`${def.name}-${def.version}`}>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <span className="font-medium">{def.name}</span>
                        <CopyButton value={def.name} />
                      </div>
                    </TableCell>
                    <TableCell><Badge variant="secondary">v{def.version}</Badge></TableCell>
                    <TableCell>{def.tasks.length} {t.tasksTab}</TableCell>
                    <TableCell className="text-muted-foreground text-xs max-w-48 truncate">
                      {def.description || "—"}
                    </TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {def.timeoutSeconds ? `${def.timeoutSeconds}s` : t.none}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex gap-1 justify-end">
                        <Button variant="ghost" size="icon" onClick={() => setStartDef(def)} title={t.startExecution}>
                          <Play className="h-3 w-3" />
                        </Button>
                        <Button variant="ghost" size="icon" onClick={() => setViewDef(def)} title={t.viewJson}>
                          <Eye className="h-3 w-3" />
                        </Button>
                        <Button variant="ghost" size="icon" onClick={() => editAsNextVersion(def)} title="Edit (save as v+1)">
                          <Pencil className="h-3 w-3" />
                        </Button>
                        <Button variant="ghost" size="icon" onClick={() => setHistoryTarget({ name: def.name, version: def.version })} title={t.versionHistory}>
                          <History className="h-3 w-3" />
                        </Button>
                        <Button variant="ghost" size="icon" onClick={() => cloneDef(def)} title={t.cloneAndEdit}>
                          <Copy className="h-3 w-3" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={() => setDeleteTarget({ name: def.name, version: def.version })}
                          title={t.delete}
                        >
                          <Trash2 className="h-3 w-3" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
            </>
          )}
        </CardContent>
        <div className="border-t px-6 py-2 shrink-0">
          <PaginationControls
            page={pagination.page}
            totalPages={pagination.totalPages}
            canPrev={pagination.canPrev}
            canNext={pagination.canNext}
            onPrev={pagination.prev}
            onNext={pagination.next}
            rangeLabel={`${pagination.startIndex + 1}–${pagination.endIndex}`}
            totalItems={pagination.totalItems}
          />
        </div>
      </Card>

      {/* Create workflow dialog (full-page builder) */}
      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent className="max-w-6xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{t.createWorkflowDef}</DialogTitle>
          </DialogHeader>
          <WorkflowBuilder
            onSubmit={(def) => createMut.mutate(def)}
            isPending={createMut.isPending}
            submitLabel={t.createWorkflow}
          />
        </DialogContent>
      </Dialog>

      {/* Edit/clone workflow dialog */}
      <Dialog open={!!editDef} onOpenChange={() => setEditDef(null)}>
        <DialogContent className="max-w-6xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{t.editWorkflowDef}</DialogTitle>
          </DialogHeader>
          {editDef && (
            <WorkflowBuilder
              initialDef={editDef}
              onSubmit={(def) => createMut.mutate(def)}
              isPending={createMut.isPending}
              submitLabel={t.saveWorkflow}
            />
          )}
        </DialogContent>
      </Dialog>

      {/* Start workflow dialog */}
      <StartWorkflowDialog
        open={!!startDef}
        onOpenChange={(open) => { if (!open) setStartDef(null); }}
        preselectedDef={startDef ?? undefined}
      />

      {/* View dialog */}
      <Dialog open={!!viewDef} onOpenChange={() => setViewDef(null)}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>{viewDef?.name} v{viewDef?.version}</DialogTitle>
          </DialogHeader>
          <pre className="text-xs rounded-lg bg-muted p-4 overflow-auto max-h-[60vh]">
            {JSON.stringify(viewDef, null, 2)}
          </pre>
          <div className="flex justify-end gap-2">
            <Button variant="outline" size="sm" onClick={() => { if (viewDef) { setEditDef(JSON.parse(JSON.stringify(viewDef))); setViewDef(null); } }}>
              {t.edit}
            </Button>
            <CopyButton value={JSON.stringify(viewDef, null, 2)} />
          </div>
        </DialogContent>
      </Dialog>

      {/* Delete confirmation */}
      <ConfirmDialog
        open={!!deleteTarget}
        onOpenChange={(open) => { if (!open) setDeleteTarget(null); }}
        title={t.deleteWorkflowDef}
        description={`"${deleteTarget?.name}" v${deleteTarget?.version} — ${t.deleteConfirmSuffix}`}
        confirmLabel={t.delete}
        variant="destructive"
        onConfirm={async () => {
          if (deleteTarget) {
            await deleteMut.mutateAsync(deleteTarget);
          }
        }}
      />

      {/* Version history dialog */}
      {historyTarget && (
        <VersionHistoryDialog
          workflowName={historyTarget.name}
          currentVersion={historyTarget.version}
          open={!!historyTarget}
          onOpenChange={(open) => { if (!open) setHistoryTarget(null); }}
        />
      )}
    </div>
  );
}
