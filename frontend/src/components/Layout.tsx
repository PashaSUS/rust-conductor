import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { Outlet, Link, useNavigate, useLocation } from "react-router";
import {
  LayoutDashboard,
  Play,
  FileCode2,
  ListChecks,
  Layers,
  Info,
  Zap,
  Rocket,
  Search,
  PanelLeftClose,
  PanelLeftOpen,
  GitBranch,
  Calendar,
  GitCompare,
  Diff,
  BookTemplate,
  PencilRuler,
  Menu,
  X,
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
import { ThemeSwitcher } from "@/components/ThemeSwitcher";
import { CommandPalette } from "@/components/CommandPalette";
import { Breadcrumbs } from "@/components/Breadcrumbs";
import { useThemeText } from "@/components/ThemeContext";
import { NotificationBell } from "@/components/NotificationBell";

export default function Layout() {
  const t = useThemeText();
  const [startOpen, setStartOpen] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);
  const [collapsed, setCollapsed] = useState(
    () => localStorage.getItem("sidebar-collapsed") === "true",
  );
  const navigate = useNavigate();
  const { pathname } = useLocation();

  // Close mobile drawer on route change
  useEffect(() => { setMobileOpen(false); }, [pathname]);

  const navItems = useMemo(() => [
    // Core
    { to: "/", label: t.dashboard, icon: LayoutDashboard, shortcut: "1" },
    { to: "/executions", label: t.executions, icon: Play, shortcut: "2" },
    { to: "/definitions", label: t.workflowDefs, icon: FileCode2, shortcut: "3" },
    { to: "/taskdefs", label: t.taskDefs, icon: ListChecks, shortcut: "4" },
    { to: "/queues", label: t.taskQueues, icon: Layers, shortcut: "5" },
    { to: "/schedules", label: t.schedules, icon: Calendar, shortcut: "6" },
    // Analysis & Visualization
    { to: "---", label: "divider", icon: null as never, shortcut: "" },
    { to: "/dependencies", label: t.dependencyGraph, icon: GitBranch, shortcut: "7" },
    { to: "/compare", label: "Compare", icon: GitCompare, shortcut: "" },
    { to: "/diff", label: "Diff", icon: Diff, shortcut: "" },
    // Tools
    { to: "---2", label: "divider", icon: null as never, shortcut: "" },
    { to: "/designer", label: "Designer", icon: PencilRuler, shortcut: "" },
    { to: "/templates", label: "Templates", icon: BookTemplate, shortcut: "" },
    { to: "/about", label: t.about, icon: Info, shortcut: "8" },
  ], [t]);

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
          case "6":
            e.preventDefault();
            navigate("/schedules");
            break;
          case "7":
            e.preventDefault();
            navigate("/dependencies");
            break;
          case "8":
            e.preventDefault();
            navigate("/about");
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
        {/* Mobile hamburger */}
        <div className="md:hidden fixed top-0 left-0 right-0 z-40 flex items-center gap-2 border-b bg-sidebar px-4 py-3">
          <Button variant="ghost" size="icon" onClick={() => setMobileOpen(true)}>
            <Menu className="h-5 w-5" />
          </Button>
          <Zap className="h-5 w-5 text-chart-1" />
          <span className="font-bold text-sm">{t.appName}</span>
        </div>

        {/* Mobile drawer overlay */}
        {mobileOpen && (
          <div className="md:hidden fixed inset-0 z-50 flex">
            <div className="fixed inset-0 bg-black/80" onClick={() => setMobileOpen(false)} />
            <aside className="relative w-64 bg-sidebar flex flex-col animate-in slide-in-from-left duration-200 max-h-screen overflow-y-auto">
              <div className="flex items-center justify-between px-4 py-4 border-b">
                <div className="flex items-center gap-2">
                  <Zap className="h-5 w-5 text-chart-1" />
                  <span className="font-bold">{t.appName}</span>
                </div>
                <Button variant="ghost" size="icon" onClick={() => setMobileOpen(false)}>
                  <X className="h-4 w-4" />
                </Button>
              </div>
              <nav className="py-3 px-3 space-y-0.5 flex-1">
                {navItems.map(({ to, label, icon: Icon }) => {
                  if (to.startsWith("---")) return <div key={to} className="my-2 border-t border-border" />;
                  const isActive = to === "/" ? pathname === "/" : pathname.startsWith(to);
                  return (
                    <Link
                      key={to}
                      to={to}
                      className={cn(
                        "flex h-9 items-center rounded-md text-sm font-medium px-3 transition-colors",
                        isActive ? "bg-sidebar-accent text-sidebar-accent-foreground" : "text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-foreground",
                      )}
                    >
                      <Icon className="h-4 w-4 mr-3" />
                      <span className="truncate">{label}</span>
                    </Link>
                  );
                })}
              </nav>
            </aside>
          </div>
        )}

        {/* Sidebar */}
        <aside
          className={cn(
            "hidden md:flex border-r bg-sidebar flex-col transition-all duration-200",
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
                {t.appName}
              </h1>
            )}
          </div>
          <nav className={cn("py-3 space-y-0.5 overflow-y-auto", collapsed ? "px-2" : "px-3")}>
            {navItems.map(({ to, label, icon: Icon, shortcut }) => {
              // Divider separator between groups
              if (to.startsWith("---")) {
                return <div key={to} className="my-2 border-t border-border" />;
              }

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
                          : "text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-foreground",
                      )}
                    >
                      <span className={cn("flex items-center justify-center", collapsed ? "w-full" : "w-5 mr-3")}>
                        <Icon className="h-4 w-4" />
                      </span>
                      {!collapsed && (
                        <>
                          <span className="flex-1 truncate">{label}</span>
                          {shortcut && (
                            <kbd className="ml-auto hidden sm:inline-flex h-5 min-w-10 items-center justify-center rounded border bg-muted px-1 font-mono text-[10px] font-medium text-muted-foreground">
                              Alt+{shortcut}
                          </kbd>
                          )}
                        </>
                      )}
                    </Link>
                  </TooltipTrigger>
                  {collapsed && (
                    <TooltipContent side="right">
                      <p>
                        {label}
                        {shortcut && (
                          <span className="text-muted-foreground ml-1">
                            Alt+{shortcut}
                          </span>
                        )}
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
                    <p>{t.search} (Ctrl+K)</p>
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
                    <p>{t.startWorkflow} (Alt+N)</p>
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
                    {t.search}
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
                  {t.startWorkflow}
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
            {!collapsed && <span>{t.tagline}</span>}
            <div
              className={cn(
                "flex items-center",
                collapsed ? "flex-col gap-1" : "gap-1",
              )}
            >
              <ThemeSwitcher />
              <NotificationBell />
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
                  <p>{collapsed ? t.expandSidebar : t.collapseSidebar} (Alt+B)</p>
                </TooltipContent>
              </Tooltip>
            </div>
          </div>
        </aside>

        {/* Main content */}
        <main className="flex-1 overflow-y-auto pt-14 md:pt-0">
          <div className="p-6">
            <Breadcrumbs />
            <PageTransition locationKey={pathname}>
              <Outlet />
            </PageTransition>
          </div>
        </main>

        <StartWorkflowDialog open={startOpen} onOpenChange={setStartOpen} />
        <CommandPalette />
      </div>
    </TooltipProvider>
  );
}

function PageTransition({ locationKey, children }: { locationKey: string; children: React.ReactNode }) {
  const [displayChildren, setDisplayChildren] = useState(children);
  const [transitioning, setTransitioning] = useState(false);
  const prevKey = useRef(locationKey);

  useEffect(() => {
    if (locationKey !== prevKey.current) {
      prevKey.current = locationKey;
      setTransitioning(true);
      const timer = setTimeout(() => {
        setDisplayChildren(children);
        setTransitioning(false);
      }, 150);
      return () => clearTimeout(timer);
    } else {
      setDisplayChildren(children);
    }
  }, [locationKey, children]);

  return (
    <div
      className="transition-all duration-150 ease-in-out"
      style={{ opacity: transitioning ? 0 : 1, transform: transitioning ? "translateY(4px)" : "translateY(0)" }}
    >
      {displayChildren}
    </div>
  );
}
