import { Badge } from "@/components/ui/badge";

function statusVariant(status: string): string {
  switch (status) {
    case "COMPLETED":
      return "success";
    case "RUNNING":
    case "IN_PROGRESS":
      return "default";
    case "FAILED":
    case "FAILED_WITH_TERMINAL_ERROR":
    case "TIMED_OUT":
      return "destructive";
    case "PAUSED":
    case "SCHEDULED":
      return "warning";
    default:
      return "secondary";
  }
}

/** Workflow-level status badge (RUNNING, COMPLETED, FAILED, etc.) */
export function StatusBadge({
  status,
  className,
}: {
  status: string;
  className?: string;
}) {
  return (
    <Badge variant={statusVariant(status) as "default"} className={className}>
      {status}
    </Badge>
  );
}

/** Task-level status badge (IN_PROGRESS, SCHEDULED, COMPLETED, etc.) */
export function TaskStatusBadge({
  status,
  className,
}: {
  status: string;
  className?: string;
}) {
  return (
    <Badge variant={statusVariant(status) as "default"} className={className}>
      {status}
    </Badge>
  );
}
