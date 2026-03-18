import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { metadataApi, workflowApi, type TaskDef } from "@/api/conductor";
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

/** Try to parse a string value into a typed value */
function parseFieldValue(raw: string): unknown {
  const trimmed = raw.trim();
  if (trimmed === "") return "";
  if (trimmed === "true") return true;
  if (trimmed === "false") return false;
  if (trimmed === "null") return null;
  const num = Number(trimmed);
  if (!isNaN(num) && trimmed !== "") return num;
  if (
    (trimmed.startsWith("{") && trimmed.endsWith("}")) ||
    (trimmed.startsWith("[") && trimmed.endsWith("]"))
  ) {
    try { return JSON.parse(trimmed); } catch { /* fall through */ }
  }
  return raw;
}

function buildInputFromFields(fields: Record<string, string>): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  for (const [key, val] of Object.entries(fields)) {
    result[key] = parseFieldValue(val);
  }
  return result;
}

export default function TaskDefs() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const [viewDef, setViewDef] = useState<TaskDef | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [testRunDef, setTestRunDef] = useState<TaskDef | null>(null);
  const [testInput, setTestInput] = useState("{}");
  const [testFieldValues, setTestFieldValues] = useState<Record<string, string>>({});
  const [testInputMode, setTestInputMode] = useState<"fields" | "json">("fields");
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState("");

  const { data: defs, isLoading } = useQuery({
    queryKey: ["task-defs"],
    queryFn: metadataApi.listTaskDefs,
  });

  const createMut = useMutation({
    mutationFn: (defs: TaskDef[]) => metadataApi.registerTaskDefs(defs),
    onSuccess: () => {
      toast.success("Task definition(s) created");
      queryClient.invalidateQueries({ queryKey: ["task-defs"] });
      setCreateOpen(false);
    },
    onError: (e) => toast.error(e.message),
  });

  const deleteMut = useMutation({
    mutationFn: (name: string) => metadataApi.deleteTaskDef(name),
    onSuccess: () => {
      toast.success("Deleted");
      queryClient.invalidateQueries({ queryKey: ["task-defs"] });
    },
  });

  const testRunMut = useMutation({
    mutationFn: async ({ taskName, input }: { taskName: string; input: Record<string, unknown> }) => {
      const wfName = `__test_run_${taskName}`;
      const taskRef = `test_${taskName}`;
      await metadataApi.registerWorkflowDef({
        name: wfName,
        version: 1,
        description: `Auto-generated test workflow for task: ${taskName}`,
        tasks: [
          {
            name: "fork_test",
            taskReferenceName: "fork_test",
            type: "FORK_JOIN",
            forkTasks: [
              [
                {
                  name: taskName,
                  taskReferenceName: taskRef,
                  type: "SIMPLE",
                  inputParameters: input,
                },
              ],
            ],
          },
          {
            name: "join_test",
            taskReferenceName: "join_test",
            type: "JOIN",
            joinOn: [taskRef],
          },
        ],
      });
      const workflowId = await workflowApi.start({ name: wfName, version: 1, input });
      return workflowId;
    },
    onSuccess: (workflowId) => {
      toast.success("Test workflow started");
      setTestRunDef(null);
      setTestInput("{}");
      navigate(`/executions/${workflowId}`);
    },
    onError: (e) => toast.error(e.message),
  });

  const filteredDefs = (defs ?? []).filter(
    (d) =>
      !searchTerm ||
      d.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      d.description?.toLowerCase().includes(searchTerm.toLowerCase()) ||
      d.ownerEmail?.toLowerCase().includes(searchTerm.toLowerCase())
  );

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold tracking-tight">Task Definitions</h2>
        <Button size="sm" onClick={() => setCreateOpen(true)}>
          <Plus className="h-4 w-4 mr-1" />
          New Task Def
        </Button>
      </div>

      {/* Search bar */}
      <div className="flex items-center gap-2">
        <Search className="h-4 w-4 text-muted-foreground" />
        <Input
          placeholder="Filter task definitions..."
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="max-w-xs"
        />
        {searchTerm && (
          <span className="text-xs text-muted-foreground">
            {filteredDefs.length} of {defs?.length ?? 0}
          </span>
        )}
      </div>

      <Card>
        <CardContent className="pt-6">
          {isLoading ? (
            <p className="text-muted-foreground text-sm">Loading...</p>
          ) : filteredDefs.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <p className="text-sm">
                {defs?.length === 0
                  ? "No task definitions yet. Create your first one!"
                  : "No definitions match your search."}
              </p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Retry Count</TableHead>
                  <TableHead>Timeout</TableHead>
                  <TableHead>Response Timeout</TableHead>
                  <TableHead>Owner</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredDefs.map((def) => (
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
                        <Button variant="ghost" size="icon" onClick={() => {
                          setTestRunDef(def);
                          setTestInput("{}");
                          const fields: Record<string, string> = {};
                          for (const k of def.inputKeys ?? []) fields[k] = "";
                          setTestFieldValues(fields);
                          setTestInputMode((def.inputKeys?.length ?? 0) > 0 ? "fields" : "json");
                        }} title="Test Run">
                          <Play className="h-3 w-3" />
                        </Button>
                        <Button variant="ghost" size="icon" onClick={() => setViewDef(def)} title="View">
                          <Eye className="h-3 w-3" />
                        </Button>
                        <Button variant="ghost" size="icon" onClick={() => setDeleteTarget(def.name)} title="Delete">
                          <Trash2 className="h-3 w-3" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Create dialog with form/JSON tabs */}
      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Create Task Definition</DialogTitle>
          </DialogHeader>
          <Tabs defaultValue="form">
            <TabsList>
              <TabsTrigger value="form">Form</TabsTrigger>
              <TabsTrigger value="json">JSON</TabsTrigger>
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
                    const arr = Array.isArray(parsed) ? parsed : [parsed];
                    // Store for submit — keep a ref so the button can use it
                    (window as unknown as Record<string, unknown>).__taskJsonDraft = arr;
                  } catch {
                    // ignore
                  }
                }}
              />
              <Button
                onClick={() => {
                  const draft = (window as unknown as Record<string, unknown>).__taskJsonDraft as TaskDef[] | undefined;
                  if (draft) {
                    createMut.mutate(draft);
                  } else {
                    toast.error("Invalid JSON");
                  }
                }}
                disabled={createMut.isPending}
              >
                {createMut.isPending ? "Creating..." : "Create from JSON"}
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
            <DialogTitle>Test Run: {testRunDef?.name}</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            This will create a workflow with a FORK → <strong>{testRunDef?.name}</strong> → JOIN structure and start it.
            The task will appear in its queue, ready to be polled and executed.
          </p>

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="text-sm font-medium">Task Input</Label>
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
                    <><Code className="h-3 w-3" /> JSON</>
                  ) : (
                    <><FormInput className="h-3 w-3" /> Fields</>
                  )}
                </Button>
              )}
            </div>

            {testInputMode === "fields" && (testRunDef?.inputKeys?.length ?? 0) > 0 ? (
              <div className="space-y-3 rounded-lg border p-3">
                {(testRunDef?.inputKeys ?? []).map((k) => (
                  <div key={k} className="space-y-1">
                    <Label className="text-xs font-mono">{k}</Label>
                    <Input
                      value={testFieldValues[k] ?? ""}
                      onChange={(e) =>
                        setTestFieldValues((prev) => ({ ...prev, [k]: e.target.value }))
                      }
                      placeholder={`Value for ${k}`}
                      className="font-mono text-xs"
                    />
                  </div>
                ))}
                <p className="text-[10px] text-muted-foreground">
                  Values are auto-parsed: numbers, booleans (true/false), JSON objects/arrays, or strings.
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
            onClick={() => {
              try {
                const input =
                  testInputMode === "fields" && (testRunDef?.inputKeys?.length ?? 0) > 0
                    ? buildInputFromFields(testFieldValues)
                    : JSON.parse(testInput);
                testRunMut.mutate({ taskName: testRunDef!.name, input });
              } catch {
                toast.error("Invalid JSON input");
              }
            }}
            disabled={testRunMut.isPending}
          >
            {testRunMut.isPending ? "Starting..." : "Run Test"}
          </Button>
        </DialogContent>
      </Dialog>

      {/* Delete confirmation */}
      <ConfirmDialog
        open={!!deleteTarget}
        onOpenChange={(open) => { if (!open) setDeleteTarget(null); }}
        title="Delete Task Definition"
        description={`Are you sure you want to delete "${deleteTarget}"? This action cannot be undone.`}
        confirmLabel="Delete"
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
