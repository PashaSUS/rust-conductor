import { useState, useEffect, useCallback } from "react";
import { Outlet, Link, useNavigate, useLocation } from "react-router";
import {
  LayoutDashboard,
  Play,
  FileCode2,
  ListChecks,
  Layers,
  Zap,
  Rocket,
  Search,
  PanelLeftClose,
  PanelLeftOpen,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { StartWorkflowDialog } from "@/components/StartWorkflowDialog";
import { ThemeToggle } from "@/components/ThemeToggle";
import { CommandPalette } from "@/components/CommandPalette";
import { Breadcrumbs } from "@/components/Breadcrumbs";

const navItems = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard, shortcut: "1" },
  { to: "/executions", label: "Executions", icon: Play, shortcut: "2" },
  {
    to: "/definitions",
    label: "Workflow Defs",
    icon: FileCode2,
    shortcut: "3",
  },
  { to: "/taskdefs", label: "Task Defs", icon: ListChecks, shortcut: "4" },
  { to: "/queues", label: "Task Queues", icon: Layers, shortcut: "5" },
];

export default function Layout() {
  const [startOpen, setStartOpen] = useState(false);
  const [collapsed, setCollapsed] = useState(
    () => localStorage.getItem("sidebar-collapsed") === "true",
  );
  const navigate = useNavigate();
  const { pathname } = useLocation();

  useEffect(() => {
    localStorage.setItem("sidebar-collapsed", String(collapsed));
  }, [collapsed]);

  // Global keyboard shortcuts
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      // Don't trigger when typing in inputs
      const tag = (e.target as HTMLElement).tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

      if (e.altKey) {
        switch (e.key) {
          case "1":
            e.preventDefault();
            navigate("/");
            break;
          case "2":
            e.preventDefault();
            navigate("/executions");
            break;
          case "3":
            e.preventDefault();
            navigate("/definitions");
            break;
          case "4":
            e.preventDefault();
            navigate("/taskdefs");
            break;
          case "5":
            e.preventDefault();
            navigate("/queues");
            break;
          case "n":
            e.preventDefault();
            setStartOpen(true);
            break;
          case "b":
            e.preventDefault();
            setCollapsed((c) => !c);
            break;
        }
      }
    },
    [navigate],
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  return (
    <TooltipProvider delayDuration={0}>
      <div className="flex h-screen bg-background">
        {/* Sidebar */}
        <aside
          className={cn(
            "border-r bg-sidebar flex flex-col transition-all duration-200",
            collapsed ? "w-16" : "w-64",
          )}
        >
          <div
            className={cn(
              "flex items-center gap-2 border-b",
              collapsed ? "px-3 py-5 justify-center" : "px-6 py-5",
            )}
          >
            <Zap className="h-6 w-6 text-chart-1 shrink-0" />
            {!collapsed && (
              <h1 className="font-bold text-lg text-sidebar-foreground">
                Rust Conductor
              </h1>
            )}
          </div>
          <nav className={cn("py-3 space-y-0.5", collapsed ? "px-2" : "px-3")}>
            {navItems.map(({ to, label, icon: Icon, shortcut }) => {
              const isActive = to === "/" ? pathname === "/" : pathname.startsWith(to);
              return (
                <Tooltip key={to}>
                  <TooltipTrigger asChild>
                    <Link
                      to={to}
                      className={cn(
                        "group relative flex h-9 items-center rounded-md text-sm font-medium transition-colors",
                        collapsed ? "justify-center w-full" : "px-3",
                        isActive
                          ? "bg-sidebar-accent text-sidebar-accent-foreground"
                          : "text-sidebar-foreground/70 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground",
                      )}
                    >
                      <span className={cn("flex items-center justify-center", collapsed ? "w-full" : "w-5 mr-3")}>
                        <Icon className="h-4 w-4" />
                      </span>
                      {!collapsed && (
                        <>
                          <span className="flex-1 truncate">{label}</span>
                          <kbd className="ml-auto hidden sm:inline-flex h-5 min-w-10 items-center justify-center rounded border bg-muted/50 px-1 font-mono text-[10px] font-medium text-muted-foreground/60">
                            Alt+{shortcut}
                          </kbd>
                        </>
                      )}
                    </Link>
                  </TooltipTrigger>
                  {collapsed && (
                    <TooltipContent side="right">
                      <p>
                        {label}{" "}
                        <span className="text-muted-foreground ml-1">
                          Alt+{shortcut}
                        </span>
                      </p>
                    </TooltipContent>
                  )}
                </Tooltip>
              );
            })}
          </nav>
          <div className="flex-1" />
          <div
            className={cn(
              "border-t space-y-2 py-3",
              collapsed ? "px-2" : "px-3",
            )}
          >
            {collapsed ? (
              <>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="outline"
                      size="icon"
                      className="w-full"
                      onClick={() =>
                        document.dispatchEvent(
                          new KeyboardEvent("keydown", {
                            key: "k",
                            ctrlKey: true,
                          }),
                        )
                      }
                    >
                      <Search className="h-4 w-4" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent side="right">
                    <p>Search (Ctrl+K)</p>
                  </TooltipContent>
                </Tooltip>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="outline"
                      size="icon"
                      className="w-full"
                      onClick={() => setStartOpen(true)}
                    >
                      <Rocket className="h-4 w-4" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent side="right">
                    <p>Start Workflow (Alt+N)</p>
                  </TooltipContent>
                </Tooltip>
              </>
            ) : (
              <>
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full justify-between text-muted-foreground"
                  onClick={() =>
                    document.dispatchEvent(
                      new KeyboardEvent("keydown", { key: "k", ctrlKey: true }),
                    )
                  }
                >
                  <span className="flex items-center gap-2">
                    <Search className="h-4 w-4" />
                    Search…
                  </span>
                  <kbd className="pointer-events-none hidden sm:inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground">
                    Ctrl+K
                  </kbd>
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full justify-start gap-2"
                  onClick={() => setStartOpen(true)}
                >
                  <Rocket className="h-4 w-4" />
                  Start Workflow
                </Button>
              </>
            )}
          </div>
          <div
            className={cn(
              "border-t py-3 text-xs text-muted-foreground flex items-center",
              collapsed
                ? "px-2 justify-center flex-col gap-2"
                : "px-6 justify-between",
            )}
          >
            {!collapsed && <span>Conductor-compliant engine</span>}
            <div
              className={cn(
                "flex items-center",
                collapsed ? "flex-col gap-1" : "gap-1",
              )}
            >
              <ThemeToggle />
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8"
                    onClick={() => setCollapsed((c) => !c)}
                  >
                    {collapsed ? (
                      <PanelLeftOpen className="h-4 w-4" />
                    ) : (
                      <PanelLeftClose className="h-4 w-4" />
                    )}
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="right">
                  <p>{collapsed ? "Expand" : "Collapse"} sidebar (Alt+B)</p>
                </TooltipContent>
              </Tooltip>
            </div>
          </div>
        </aside>

        {/* Main content */}
        <main className="flex-1 overflow-y-auto">
          <div className="p-6">
            <Breadcrumbs />
            <Outlet />
          </div>
        </main>

        <StartWorkflowDialog open={startOpen} onOpenChange={setStartOpen} />
        <CommandPalette />
      </div>
    </TooltipProvider>
  );
}
