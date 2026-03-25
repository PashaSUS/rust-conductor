import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { metadataApi, type TaskDef } from "@/api/conductor";
import { TaskDefForm } from "@/components/TaskDefForm";
import { useThemeText } from "@/components/ThemeContext";

/**
 * Full-page task definition creator.
 */
export default function CreateTaskDef() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const t = useThemeText();

  const createMut = useMutation({
    mutationFn: (defs: TaskDef[]) => metadataApi.registerTaskDefs(defs),
    onSuccess: () => {
      toast.success(t.toastTaskDefCreated ?? "Task definition created");
      queryClient.invalidateQueries({ queryKey: ["task-defs"] });
      navigate("/taskdefs");
    },
    onError: (e) => toast.error(e.message),
  });

  return (
    <div className="space-y-4 max-w-4xl">
      <TaskDefForm onSubmit={(defs) => createMut.mutate(defs)} isPending={createMut.isPending} />
    </div>
  );
}
