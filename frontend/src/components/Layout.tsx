import { Outlet, NavLink } from "react-router";
import {
  LayoutDashboard,
  Play,
  FileCode2,
  ListChecks,
  Layers,
  Zap,
} from "lucide-react";
import { cn } from "@/lib/utils";

const navItems = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard },
  { to: "/executions", label: "Executions", icon: Play },
  { to: "/definitions", label: "Workflow Defs", icon: FileCode2 },
  { to: "/taskdefs", label: "Task Defs", icon: ListChecks },
  { to: "/queues", label: "Task Queues", icon: Layers },
];

export default function Layout() {
  return (
    <div className="flex h-screen bg-background">
      {/* Sidebar */}
      <aside className="w-64 border-r bg-sidebar flex flex-col">
        <div className="flex items-center gap-2 px-6 py-5 border-b">
          <Zap className="h-6 w-6 text-chart-1" />
          <h1 className="font-bold text-lg text-sidebar-foreground">
            Rust Conductor
          </h1>
        </div>
        <nav className="flex-1 px-3 py-4 space-y-1">
          {navItems.map(({ to, label, icon: Icon }) => (
            <NavLink
              key={to}
              to={to}
              end={to === "/"}
              className={({ isActive }) =>
                cn(
                  "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
                  isActive
                    ? "bg-sidebar-accent text-sidebar-accent-foreground"
                    : "text-sidebar-foreground/70 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
                )
              }
            >
              <Icon className="h-4 w-4" />
              {label}
            </NavLink>
          ))}
        </nav>
        <div className="border-t px-6 py-3 text-xs text-muted-foreground">
          Conductor-compliant engine
        </div>
      </aside>

      {/* Main content */}
      <main className="flex-1 overflow-y-auto">
        <div className="p-6">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
