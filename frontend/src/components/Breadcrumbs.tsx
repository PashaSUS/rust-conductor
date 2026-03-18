import { useLocation, Link } from "react-router";
import { ChevronRight, Home } from "lucide-react";

const ROUTE_LABELS: Record<string, string> = {
  "": "Dashboard",
  executions: "Executions",
  definitions: "Workflow Definitions",
  taskdefs: "Task Definitions",
  queues: "Task Queues",
};

export function Breadcrumbs() {
  const location = useLocation();
  const segments = location.pathname.split("/").filter(Boolean);

  if (segments.length === 0) return null;

  const crumbs: { label: string; path: string }[] = [];

  for (let i = 0; i < segments.length; i++) {
    const seg = segments[i];
    const path = "/" + segments.slice(0, i + 1).join("/");

    if (ROUTE_LABELS[seg]) {
      crumbs.push({ label: ROUTE_LABELS[seg], path });
    } else if (i > 0 && segments[i - 1] === "executions") {
      // Workflow ID — truncate for display
      crumbs.push({ label: seg.length > 12 ? seg.slice(0, 8) + "…" : seg, path });
    } else {
      crumbs.push({ label: seg, path });
    }
  }

  return (
    <nav className="flex items-center gap-1 text-xs text-muted-foreground mb-4">
      <Link
        to="/"
        className="hover:text-foreground transition-colors flex items-center gap-1"
      >
        <Home className="h-3 w-3" />
      </Link>
      {crumbs.map((crumb, i) => (
        <span key={crumb.path} className="flex items-center gap-1">
          <ChevronRight className="h-3 w-3" />
          {i === crumbs.length - 1 ? (
            <span className="text-foreground font-medium">{crumb.label}</span>
          ) : (
            <Link to={crumb.path} className="hover:text-foreground transition-colors">
              {crumb.label}
            </Link>
          )}
        </span>
      ))}
    </nav>
  );
}
