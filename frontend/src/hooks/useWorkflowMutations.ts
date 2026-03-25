import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { workflowApi, bulkApi } from "@/api/conductor";
import { useThemeText } from "@/components/ThemeContext";

/**
 * Reusable hook for common workflow lifecycle mutations
 * (pause, resume, terminate, restart, retry).
 *
 * @param invalidateKeys — additional query keys to invalidate on success
 */
export function useWorkflowMutations(invalidateKeys: string[][] = []) {
  const queryClient = useQueryClient();
  const t = useThemeText();

  const invalidate = () => {
    for (const key of invalidateKeys) {
      queryClient.invalidateQueries({ queryKey: key });
    }
  };

  const pauseMut = useMutation({
    mutationFn: (id: string) => workflowApi.pause(id),
    onSuccess: () => { toast.success(t.toastWorkflowPaused); invalidate(); },
  });

  const resumeMut = useMutation({
    mutationFn: (id: string) => workflowApi.resume(id),
    onSuccess: () => { toast.success(t.toastWorkflowResumed); invalidate(); },
  });

  const terminateMut = useMutation({
    mutationFn: (id: string) => workflowApi.terminate(id),
    onSuccess: () => { toast.success(t.toastWorkflowTerminated); invalidate(); },
  });

  const restartMut = useMutation({
    mutationFn: (id: string) => workflowApi.restart(id),
    onSuccess: () => { toast.success(t.toastWorkflowRestarted); invalidate(); },
  });

  const retryMut = useMutation({
    mutationFn: (id: string) => workflowApi.retry(id),
    onSuccess: () => { toast.success(t.toastWorkflowRetried); invalidate(); },
  });

  return { pauseMut, resumeMut, terminateMut, restartMut, retryMut };
}

/**
 * Reusable hook for bulk workflow operations.
 */
export function useBulkWorkflowMutations(
  invalidateKeys: string[][] = [],
  onSuccess?: () => void,
) {
  const queryClient = useQueryClient();
  const t = useThemeText();

  const invalidate = () => {
    for (const key of invalidateKeys) {
      queryClient.invalidateQueries({ queryKey: key });
    }
    onSuccess?.();
  };

  const bulkPauseMut = useMutation({
    mutationFn: (ids: string[]) => bulkApi.pause(ids),
    onSuccess: () => { toast.success(t.bulkSuccess); invalidate(); },
  });
  const bulkResumeMut = useMutation({
    mutationFn: (ids: string[]) => bulkApi.resume(ids),
    onSuccess: () => { toast.success(t.bulkSuccess); invalidate(); },
  });
  const bulkRetryMut = useMutation({
    mutationFn: (ids: string[]) => bulkApi.retry(ids),
    onSuccess: () => { toast.success(t.bulkSuccess); invalidate(); },
  });
  const bulkRestartMut = useMutation({
    mutationFn: (ids: string[]) => bulkApi.restart(ids),
    onSuccess: () => { toast.success(t.bulkSuccess); invalidate(); },
  });
  const bulkTerminateMut = useMutation({
    mutationFn: (ids: string[]) => bulkApi.terminate(ids),
    onSuccess: () => { toast.success(t.bulkSuccess); invalidate(); },
  });

  return { bulkPauseMut, bulkResumeMut, bulkRetryMut, bulkRestartMut, bulkTerminateMut };
}
