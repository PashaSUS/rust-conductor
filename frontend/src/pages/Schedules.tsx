import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { scheduleApi, metadataApi } from "@/api/conductor";
import { usePagination } from "@/hooks/usePagination";
import { PaginationControls } from "@/components/PaginationControls";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { RelativeTime } from "@/components/RelativeTime";
import { Plus, Trash2, Calendar, AlertTriangle } from "lucide-react";
import { useThemeText } from "@/components/ThemeContext";
import { CreateScheduleDialog } from "./CreateScheduleDialog";

export default function Schedules() {
  const t = useThemeText();
  const queryClient = useQueryClient();
  const [createOpen, setCreateOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);

  const { data: schedules, isLoading } = useQuery({
    queryKey: ["schedules"],
    queryFn: scheduleApi.list,
  });

  const { data: workflowDefs } = useQuery({
    queryKey: ["workflow-defs-list"],
    queryFn: () => metadataApi.listWorkflowDefs(),
  });

  const enableMut = useMutation({
    mutationFn: (id: string) => scheduleApi.enable(id),
    onSuccess: () => {
      toast.success(t.scheduleEnabled);
      queryClient.invalidateQueries({ queryKey: ["schedules"] });
    },
  });

  const disableMut = useMutation({
    mutationFn: (id: string) => scheduleApi.disable(id),
    onSuccess: () => {
      toast.success(t.scheduleDisabled);
      queryClient.invalidateQueries({ queryKey: ["schedules"] });
    },
  });

  const deleteMut = useMutation({
    mutationFn: (id: string) => scheduleApi.delete(id),
    onSuccess: () => {
      toast.success(t.scheduleDeleted);
      queryClient.invalidateQueries({ queryKey: ["schedules"] });
    },
  });

  const [pagedSchedules, pagination] = usePagination(schedules ?? []);

  return (
    <div className="flex flex-col gap-4 h-[calc(100vh-8rem)]">
      <div className="flex items-center justify-between shrink-0">
        <h2 className="text-2xl font-bold tracking-tight">{t.schedulesTitle}</h2>
        <Button size="sm" onClick={() => setCreateOpen(true)}>
          <Plus className="h-4 w-4 mr-1" />
          {t.createSchedule}
        </Button>
      </div>

      <Card className="flex-1 min-h-0 flex flex-col">
        <CardContent className="pt-6 flex-1 min-h-0 flex flex-col">
          {isLoading ? (
            <p className="text-muted-foreground text-sm">{t.loading}</p>
          ) : !schedules?.length ? (
            <div className="text-center py-8 text-muted-foreground">
              <Calendar className="h-8 w-8 mx-auto mb-2 opacity-50" />
              <p className="text-sm">{t.noSchedules}</p>
            </div>
          ) : (
            <>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{t.name}</TableHead>
                  <TableHead>{t.cronExpression}</TableHead>
                  <TableHead>{t.timezoneLabel}</TableHead>
                  <TableHead>{t.workflow}</TableHead>
                  <TableHead>{t.status}</TableHead>
                  <TableHead>{t.lastRun}</TableHead>
                  <TableHead>{t.nextRun}</TableHead>
                  <TableHead>{t.scheduleLastError}</TableHead>
                  <TableHead className="text-right">{t.actions}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {pagedSchedules.map((s) => (
                  <TableRow key={s.scheduleId}>
                    <TableCell className="font-medium">{s.name}</TableCell>
                    <TableCell className="font-mono text-xs">{s.cronExpression}</TableCell>
                    <TableCell className="text-xs">{s.timezone}</TableCell>
                    <TableCell>
                      <span className="text-sm">{s.workflowName}</span>
                      <span className="text-xs text-muted-foreground ml-1">v{s.workflowVersion}</span>
                    </TableCell>
                    <TableCell>
                      <Badge
                        variant={s.enabled ? "default" : "secondary"}
                        className="cursor-pointer"
                        onClick={() => s.enabled ? disableMut.mutate(s.scheduleId) : enableMut.mutate(s.scheduleId)}
                      >
                        {s.enabled ? t.enabled : t.disabled}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-xs"><RelativeTime value={s.lastRunAt} /></TableCell>
                    <TableCell className="text-xs"><RelativeTime value={s.nextRunAt} /></TableCell>
                    <TableCell className="text-xs max-w-50">
                      {s.lastError ? (
                        <span className="flex items-center gap-1 text-destructive" title={s.lastError}>
                          <AlertTriangle className="h-3 w-3 shrink-0" />
                          <span className="truncate">{s.lastError}</span>
                        </span>
                      ) : (
                        <span className="text-muted-foreground">—</span>
                      )}
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => setDeleteTarget(s.scheduleId)}
                        title={t.deleteSchedule}
                      >
                        <Trash2 className="h-3 w-3" />
                      </Button>
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

      <CreateScheduleDialog
        open={createOpen}
        onOpenChange={setCreateOpen}
        workflowDefs={workflowDefs ?? []}
      />

      <ConfirmDialog
        open={!!deleteTarget}
        onOpenChange={(open) => { if (!open) setDeleteTarget(null); }}
        title={t.deleteSchedule}
        description={t.bulkConfirm}
        confirmLabel={t.deleteSchedule}
        variant="destructive"
        onConfirm={async () => {
          if (deleteTarget) await deleteMut.mutateAsync(deleteTarget);
        }}
      />
    </div>
  );
}
