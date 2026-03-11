import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { metadataApi, type WorkflowDef } from "@/api/conductor";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { Plus, Trash2, Eye } from "lucide-react";

export default function WorkflowDefs() {
  const queryClient = useQueryClient();
  const [jsonInput, setJsonInput] = useState("");
  const [viewDef, setViewDef] = useState<WorkflowDef | null>(null);
  const [createOpen, setCreateOpen] = useState(false);

  const { data: defs, isLoading } = useQuery({
    queryKey: ["workflow-defs"],
    queryFn: metadataApi.listWorkflowDefs,
  });

  const createMut = useMutation({
    mutationFn: (def: WorkflowDef) => metadataApi.registerWorkflowDef(def),
    onSuccess: () => {
      toast.success("Workflow definition created");
      queryClient.invalidateQueries({ queryKey: ["workflow-defs"] });
      setCreateOpen(false);
      setJsonInput("");
    },
    onError: (e) => toast.error(e.message),
  });

  const deleteMut = useMutation({
    mutationFn: ({ name, version }: { name: string; version: number }) =>
      metadataApi.deleteWorkflowDef(name, version),
    onSuccess: () => {
      toast.success("Deleted");
      queryClient.invalidateQueries({ queryKey: ["workflow-defs"] });
    },
  });

  const handleCreate = () => {
    try {
      const def = JSON.parse(jsonInput) as WorkflowDef;
      createMut.mutate(def);
    } catch {
      toast.error("Invalid JSON");
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold tracking-tight">Workflow Definitions</h2>
        <Dialog open={createOpen} onOpenChange={setCreateOpen}>
          <DialogTrigger asChild>
            <Button size="sm"><Plus className="h-4 w-4 mr-1" />New Definition</Button>
          </DialogTrigger>
          <DialogContent className="max-w-2xl">
            <DialogHeader>
              <DialogTitle>Create Workflow Definition</DialogTitle>
            </DialogHeader>
            <Textarea
              rows={16}
              placeholder='{"name": "my_workflow", "version": 1, "tasks": [...]}'
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
                  <TableHead>Version</TableHead>
                  <TableHead>Tasks</TableHead>
                  <TableHead>Timeout</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {(defs ?? []).map((def) => (
                  <TableRow key={`${def.name}-${def.version}`}>
                    <TableCell className="font-medium">{def.name}</TableCell>
                    <TableCell><Badge variant="secondary">v{def.version}</Badge></TableCell>
                    <TableCell>{def.tasks.length} tasks</TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {def.timeoutSeconds ? `${def.timeoutSeconds}s` : "none"}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex gap-1 justify-end">
                        <Button variant="ghost" size="icon" onClick={() => setViewDef(def)} title="View">
                          <Eye className="h-3 w-3" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={() => deleteMut.mutate({ name: def.name, version: def.version })}
                          title="Delete"
                        >
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

      {/* View dialog */}
      <Dialog open={!!viewDef} onOpenChange={() => setViewDef(null)}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>{viewDef?.name} v{viewDef?.version}</DialogTitle>
          </DialogHeader>
          <pre className="text-xs rounded-lg bg-muted p-4 overflow-auto max-h-[60vh]">
            {JSON.stringify(viewDef, null, 2)}
          </pre>
        </DialogContent>
      </Dialog>
    </div>
  );
}
