import { useState, useRef } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { metadataApi, workflowApi, type TaskDef } from "@/api/conductor";
import { buildInputFromFields } from "@/lib/field-parser";
import { usePagination } from "@/hooks/usePagination";
import { PaginationControls } from "@/components/PaginationControls";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Plus, Trash2, Eye, Play, Search, Code, FormInput } from "lucide-react";
import { TaskDefForm } from "@/components/TaskDefForm";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { CopyButton } from "@/components/CopyButton";
import { useThemeText } from "@/components/ThemeContext";

export default function TaskDefs() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const t = useThemeText();
  const [viewDef, setViewDef] = useState<TaskDef | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [testRunDef, setTestRunDef] = useState<TaskDef | null>(null);
  const [testInput, setTestInput] = useState("{}");
  const [testFieldValues, setTestFieldValues] = useState<Record<string, string>>({});
  const [testInputMode, setTestInputMode] = useState<"fields" | "json">("fields");
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState("");
  const jsonDraftRef = useRef<TaskDef[] | null>(null);

  const { data: defs, isLoading } = useQuery({
    queryKey: ["task-defs"],
    queryFn: metadataApi.listTaskDefs,
  });

  const createMut = useMutation({
    mutationFn: (defs: TaskDef[]) => metadataApi.registerTaskDefs(defs),
    onSuccess: () => {
      toast.success(t.toastTaskDefCreated);
      queryClient.invalidateQueries({ queryKey: ["task-defs"] });
      setCreateOpen(false);
    },
    onError: (e) => toast.error(e.message),
  });

  const deleteMut = useMutation({
    mutationFn: (name: string) => metadataApi.deleteTaskDef(name),
    onSuccess: () => {
      toast.success(t.toastDeleted);
      queryClient.invalidateQueries({ queryKey: ["task-defs"] });
    },
  });

  const testRunMut = useMutation({
    mutationFn: async ({ taskName, input }: { taskName: string; input: Record<string, unknown> }) => {
      const wfName = "__test_run_task";

      // Register the single dynamic wrapper workflow once (reused for all task types).
      let needsRegister = false;
      try {
        await metadataApi.getWorkflowDef(wfName, 1);
      } catch {
        needsRegister = true;
      }

      if (needsRegister) {
        await metadataApi.registerWorkflowDef({
          name: wfName,
          version: 1,
          description: "Dynamic test-run wrapper — runs any task type via DYNAMIC dispatch",
          tasks: [
            {
              name: "__dynamic_test",
              taskReferenceName: "dynamic_test",
              type: "DYNAMIC",
              dynamicTaskNameParam: "taskToExecute",
              inputParameters: {
                taskToExecute: "${workflow.input.taskToExecute}",
              },
            },
          ],
        });
      }

      const workflowId = await workflowApi.start({
        name: wfName,
        version: 1,
        input: { taskToExecute: taskName, ...input },
      });
      return workflowId;
    },
    onSuccess: (workflowId) => {
      toast.success(t.toastWorkflowStarted);
      setTestRunDef(null);
      setTestInput("{}");
      navigate(`/executions/${workflowId}`);
    },
    onError: (e) => toast.error(e.message),
  });

  const openTestRun = (def: TaskDef) => {
    setTestRunDef(def);
    setTestInput("{}");
    const fields: Record<string, string> = {};
    for (const k of def.inputKeys ?? []) fields[k] = "";
    setTestFieldValues(fields);
    setTestInputMode((def.inputKeys?.length ?? 0) > 0 ? "fields" : "json");
  };

  const submitTestRun = () => {
    try {
      const input =
        testInputMode === "fields" && (testRunDef?.inputKeys?.length ?? 0) > 0
          ? buildInputFromFields(testFieldValues)
          : JSON.parse(testInput);
      testRunMut.mutate({ taskName: testRunDef!.name, input });
    } catch {
      toast.error(t.toastInvalidJson);
    }
  };

  const filteredDefs = (defs ?? []).filter(
    (d) =>
      !searchTerm ||
      d.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      d.description?.toLowerCase().includes(searchTerm.toLowerCase()) ||
      d.ownerEmail?.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const [pagedDefs, pagination] = usePagination(filteredDefs);

  return (
    <div className="flex flex-col gap-4 h-[calc(100vh-8rem)]">
      <div className="flex items-center justify-between shrink-0">
        <h2 className="text-2xl font-bold tracking-tight">{t.taskDefsTitle}</h2>
        <Button size="sm" onClick={() => navigate("/taskdefs/create")}>
          <Plus className="h-4 w-4 mr-1" />
          {t.newTaskDef}
        </Button>
      </div>

      {/* Search bar */}
      <div className="flex items-center gap-2 shrink-0">
        <Search className="h-4 w-4 text-muted-foreground" />
        <Input
          placeholder={t.filterTaskDefs}
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="max-w-xs"
        />
        {searchTerm && (
          <span className="text-xs text-muted-foreground">
            {filteredDefs.length} {t.of} {defs?.length ?? 0}
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
                  ? t.noTaskDefs
                  : t.noDefsMatch}
              </p>
            </div>
          ) : (
            <>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{t.name}</TableHead>
                  <TableHead>{t.retryCount}</TableHead>
                  <TableHead>{t.timeout}</TableHead>
                  <TableHead>{t.responseTimeout}</TableHead>
                  <TableHead>{t.owner}</TableHead>
                  <TableHead className="text-right">{t.actions}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {pagedDefs.map((def) => (
                  <TableRow key={def.name}>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <span className="font-medium">{def.name}</span>
                        <CopyButton value={def.name} />
                      </div>
                    </TableCell>
                    <TableCell><Badge variant="secondary">{def.retryCount ?? 3}</Badge></TableCell>
                    <TableCell className="text-muted-foreground text-xs">{def.timeoutSeconds ?? 3600}s</TableCell>
                    <TableCell className="text-muted-foreground text-xs">{def.responseTimeoutSeconds ?? 600}s</TableCell>
                    <TableCell className="text-muted-foreground text-xs">{def.ownerEmail ?? "—"}</TableCell>
                    <TableCell className="text-right">
                      <div className="flex gap-1 justify-end">
                        <Button variant="ghost" size="icon" onClick={() => openTestRun(def)} title={t.testRun}>
                          <Play className="h-3 w-3" />
                        </Button>
                        <Button variant="ghost" size="icon" onClick={() => setViewDef(def)} title={t.view}>
                          <Eye className="h-3 w-3" />
                        </Button>
                        <Button variant="ghost" size="icon" onClick={() => setDeleteTarget(def.name)} title={t.delete}>
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

      {/* Create dialog with form/JSON tabs */}
      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{t.createTaskDef}</DialogTitle>
          </DialogHeader>
          <Tabs defaultValue="form">
            <TabsList>
              <TabsTrigger value="form">{t.form}</TabsTrigger>
              <TabsTrigger value="json">{t.json}</TabsTrigger>
            </TabsList>
            <TabsContent value="form">
              <TaskDefForm
                onSubmit={(defs) => createMut.mutate(defs)}
                isPending={createMut.isPending}
              />
            </TabsContent>
            <TabsContent value="json" className="space-y-3">
              <Textarea
                rows={12}
                placeholder='[{"name": "my_task", "retryCount": 3, "timeoutSeconds": 3600}]'
                className="font-mono text-xs"
                onChange={(e) => {
                  try {
                    const parsed = JSON.parse(e.target.value);
                    jsonDraftRef.current = Array.isArray(parsed) ? parsed : [parsed];
                  } catch {
                    jsonDraftRef.current = null;
                  }
                }}
              />
              <Button
                onClick={() => {
                  if (jsonDraftRef.current) {
                    createMut.mutate(jsonDraftRef.current);
                  } else {
                    toast.error(t.toastInvalidJson);
                  }
                }}
                disabled={createMut.isPending}
              >
                {createMut.isPending ? t.creating : t.createFromJson}
              </Button>
            </TabsContent>
          </Tabs>
        </DialogContent>
      </Dialog>

      {/* View dialog */}
      <Dialog open={!!viewDef} onOpenChange={() => setViewDef(null)}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>{viewDef?.name}</DialogTitle>
          </DialogHeader>
          <pre className="text-xs rounded-lg bg-muted p-4 overflow-auto max-h-[60vh]">
            {JSON.stringify(viewDef, null, 2)}
          </pre>
          <div className="flex justify-end">
            <CopyButton value={JSON.stringify(viewDef, null, 2)} />
          </div>
        </DialogContent>
      </Dialog>

      {/* Test run dialog */}
      <Dialog open={!!testRunDef} onOpenChange={() => setTestRunDef(null)}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>{t.testRun}: {testRunDef?.name}</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            {t.testRunDescription}
          </p>

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="text-sm font-medium">{t.taskInput}</Label>
              {(testRunDef?.inputKeys?.length ?? 0) > 0 && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-7 text-xs gap-1"
                  onClick={() => {
                    if (testInputMode === "fields") {
                      setTestInput(JSON.stringify(buildInputFromFields(testFieldValues), null, 2));
                      setTestInputMode("json");
                    } else {
                      try {
                        const parsed = JSON.parse(testInput);
                        if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
                          const fields: Record<string, string> = {};
                          for (const k of testRunDef?.inputKeys ?? []) {
                            const val = parsed[k];
                            fields[k] = val === undefined || val === null ? "" : typeof val === "object" ? JSON.stringify(val) : String(val);
                          }
                          setTestFieldValues(fields);
                        }
                      } catch { /* ignore */ }
                      setTestInputMode("fields");
                    }
                  }}
                >
                  {testInputMode === "fields" ? (
                    <><Code className="h-3 w-3" /> {t.json}</>
                  ) : (
                    <><FormInput className="h-3 w-3" /> {t.fields}</>
                  )}
                </Button>
              )}
            </div>

            {testInputMode === "fields" && (testRunDef?.inputKeys?.length ?? 0) > 0 ? (
              <div className="space-y-3 rounded-lg border p-3 max-h-[40vh] overflow-auto">
                {(testRunDef?.inputKeys ?? []).map((k) => (
                  <div key={k} className="space-y-1">
                    <Label className="text-xs font-mono">{k}</Label>
                    <Input
                      value={testFieldValues[k] ?? ""}
                      onChange={(e) =>
                        setTestFieldValues((prev) => ({ ...prev, [k]: e.target.value }))
                      }
                      placeholder={`${t.valueFor} ${k}`}
                      className="font-mono text-xs"
                    />
                  </div>
                ))}
                <p className="text-[10px] text-muted-foreground">
                  {t.autoParseHint}
                </p>
              </div>
            ) : (
              <Textarea
                rows={8}
                placeholder="{}"
                value={testInput}
                onChange={(e) => setTestInput(e.target.value)}
                className="font-mono text-xs"
              />
            )}
          </div>

          <Button
            onClick={submitTestRun}
            disabled={testRunMut.isPending}
          >
            {testRunMut.isPending ? t.startingWorkflow : t.runTest}
          </Button>
        </DialogContent>
      </Dialog>

      {/* Delete confirmation */}
      <ConfirmDialog
        open={!!deleteTarget}
        onOpenChange={(open) => { if (!open) setDeleteTarget(null); }}
        title={t.deleteTaskDef}
        description={`"${deleteTarget}" — ${t.deleteConfirmSuffix}`}
        confirmLabel={t.delete}
        variant="destructive"
        onConfirm={async () => {
          if (deleteTarget) {
            await deleteMut.mutateAsync(deleteTarget);
          }
        }}
      />
    </div>
  );
}
