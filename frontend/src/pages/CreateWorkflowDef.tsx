import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useSearchParams } from "react-router";
import { toast } from "sonner";
import { metadataApi, type WorkflowDef } from "@/api/conductor";
import WorkflowBuilder from "@/components/workflow-builder/WorkflowBuilder";
import { useThemeText } from "@/components/ThemeContext";

/**
 * Full-page workflow definition creator / editor.
 * When `?clone=<name>&version=<v>` params are present, pre-fills the builder
 * with the cloned definition and auto-increments version.
 */
export default function CreateWorkflowDef() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const t = useThemeText();
  const [params] = useSearchParams();

  const cloneName = params.get("clone");
  const cloneVersion = params.get("version");

  // If cloning, build initialDef from URL params (data is passed via query cache)
  const allDefs: WorkflowDef[] | undefined = queryClient.getQueryData(["workflow-defs"]);
  const initialDef = cloneName && cloneVersion
    ? allDefs?.find((d) => d.name === cloneName && d.version === Number(cloneVersion))
    : undefined;

  const createMut = useMutation({
    mutationFn: (def: WorkflowDef) => metadataApi.registerWorkflowDef(def),
    onSuccess: () => {
      toast.success("Workflow definition created");
      queryClient.invalidateQueries({ queryKey: ["workflow-defs"] });
      navigate("/definitions");
    },
    onError: (e) => toast.error(e.message),
  });

  return (
    <div className="space-y-4 max-w-4xl">
      <WorkflowBuilder
        initialDef={initialDef}
        onSubmit={(def) => createMut.mutate(def)}
        isPending={createMut.isPending}
        submitLabel={initialDef ? "Save as New Version" : (t.createWorkflow ?? "Create Workflow")}
      />
    </div>
  );
}
