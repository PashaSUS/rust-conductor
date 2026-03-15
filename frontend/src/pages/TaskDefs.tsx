import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { metadataApi, workflowApi, type TaskDef } from "@/api/conductor";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { Plus, Trash2, Eye, Play } from "lucide-react";

export default function TaskDefs() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const [jsonInput, setJsonInput] = useState("");
  const [viewDef, setViewDef] = useState<TaskDef | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [testRunDef, setTestRunDef] = useState<TaskDef | null>(null);
  const [testInput, setTestInput] = useState("{}");

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
      setJsonInput("");
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
      // Register a fork-join workflow wrapping the single task
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
      // Start the workflow
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

  const handleCreate = () => {
    try {
      const parsed = JSON.parse(jsonInput);
      const arr = Array.isArray(parsed) ? parsed : [parsed];
      createMut.mutate(arr);
    } catch {
      toast.error("Invalid JSON");
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold tracking-tight">Task Definitions</h2>
        <Dialog open={createOpen} onOpenChange={setCreateOpen}>
          <DialogTrigger asChild>
            <Button size="sm"><Plus className="h-4 w-4 mr-1" />New Task Def</Button>
          </DialogTrigger>
          <DialogContent className="max-w-2xl">
            <DialogHeader>
              <DialogTitle>Create Task Definition(s)</DialogTitle>
            </DialogHeader>
            <Textarea
              rows={12}
              placeholder='[{"name": "my_task", "retryCount": 3, "timeoutSeconds": 3600}]'
              value={jsonInput}
              onChange={(e) => setJsonInput(e.target.value)}
              className="font-mono text-xs"
            />
            <Button onClick={handleCreate} disabled={createMut.isPending}>
              {createMut.isPending ? "Creating..." : "Create"}
            </Button>
          </DialogContent>
        </Dialog>
      </div>

      <Card>
        <CardContent className="pt-6">
          {isLoading ? (
            <p className="text-muted-foreground text-sm">Loading...</p>
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
                {(defs ?? []).map((def) => (
                  <TableRow key={def.name}>
                    <TableCell className="font-medium">{def.name}</TableCell>
                    <TableCell><Badge variant="secondary">{def.retryCount ?? 3}</Badge></TableCell>
                    <TableCell className="text-muted-foreground text-xs">{def.timeoutSeconds ?? 3600}s</TableCell>
                    <TableCell className="text-muted-foreground text-xs">{def.responseTimeoutSeconds ?? 600}s</TableCell>
                    <TableCell className="text-muted-foreground text-xs">{def.ownerEmail ?? "—"}</TableCell>
                    <TableCell className="text-right">
                      <div className="flex gap-1 justify-end">
                        <Button variant="ghost" size="icon" onClick={() => { setTestRunDef(def); setTestInput("{}"); }} title="Test Run">
                          <Play className="h-3 w-3" />
                        </Button>
                        <Button variant="ghost" size="icon" onClick={() => setViewDef(def)} title="View">
                          <Eye className="h-3 w-3" />
                        </Button>
                        <Button variant="ghost" size="icon" onClick={() => deleteMut.mutate(def.name)} title="Delete">
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

      <Dialog open={!!viewDef} onOpenChange={() => setViewDef(null)}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>{viewDef?.name}</DialogTitle>
          </DialogHeader>
          <pre className="text-xs rounded-lg bg-muted p-4 overflow-auto max-h-[60vh]">
            {JSON.stringify(viewDef, null, 2)}
          </pre>
        </DialogContent>
      </Dialog>

      <Dialog open={!!testRunDef} onOpenChange={() => setTestRunDef(null)}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Test Run: {testRunDef?.name}</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            This will create a workflow with a FORK → <strong>{testRunDef?.name}</strong> → JOIN structure and start it.
            The task will appear in its queue, ready to be polled and executed.
          </p>
          <label className="text-sm font-medium">Task Input (JSON)</label>
          <Textarea
            rows={8}
            placeholder="{}"
            value={testInput}
            onChange={(e) => setTestInput(e.target.value)}
            className="font-mono text-xs"
          />
          <Button
            onClick={() => {
              try {
                const parsed = JSON.parse(testInput);
                testRunMut.mutate({ taskName: testRunDef!.name, input: parsed });
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
    </div>
  );
}
