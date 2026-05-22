import { useState, useEffect, useCallback, useRef } from "react";
import type { LucideIcon } from "lucide-react";
import type { ThemeText } from "@/components/ThemeContext";
import {
  LayoutDashboard, Play, FileCode2, ListChecks, Layers, Info,
  GitBranch, Calendar, GitCompare, Diff, BookTemplate, PencilRuler,
  BarChart3, Flame, ShieldCheck, Radio, Settings as SettingsIcon,
} from "lucide-react";
import { isLiteMode } from "@/lib/config";

export interface NavItem {
  to: string;
  label: string;
  icon: LucideIcon;
  shortcut: string;
}

export function getNavItems(t: ThemeText): NavItem[] {
  if (isLiteMode()) {
    return [
      { to: "/executions", label: t.executions, icon: Play, shortcut: "2" },
      { to: "/definitions", label: t.workflowDefs, icon: FileCode2, shortcut: "3" },
      { to: "/taskdefs", label: t.taskDefs, icon: ListChecks, shortcut: "4" },
      { to: "/queues", label: t.taskQueues, icon: Layers, shortcut: "5" },
      { to: "---", label: "divider", icon: null as never, shortcut: "" },
      { to: "/settings", label: "Settings", icon: SettingsIcon, shortcut: "," },
      { to: "/about", label: t.about, icon: Info, shortcut: "a" },
    ];
  }
  return [
    { to: "/", label: t.dashboard, icon: LayoutDashboard, shortcut: "1" },
    { to: "/executions", label: t.executions, icon: Play, shortcut: "2" },
    { to: "/definitions", label: t.workflowDefs, icon: FileCode2, shortcut: "3" },
    { to: "/taskdefs", label: t.taskDefs, icon: ListChecks, shortcut: "4" },
    { to: "/queues", label: t.taskQueues, icon: Layers, shortcut: "5" },
    { to: "/schedules", label: t.schedules, icon: Calendar, shortcut: "6" },
    { to: "/metrics", label: "Metrics", icon: BarChart3, shortcut: "7" },
    { to: "---", label: "divider", icon: null as never, shortcut: "" },
    { to: "/dependencies", label: t.dependencyGraph, icon: GitBranch, shortcut: "8" },
    { to: "/compare", label: "Compare", icon: GitCompare, shortcut: "9" },
    { to: "/diff", label: "Diff", icon: Diff, shortcut: "0" },
    { to: "---2", label: "divider", icon: null as never, shortcut: "" },
    { to: "/designer", label: "Designer", icon: PencilRuler, shortcut: "d" },
    { to: "/stresser", label: "Stresser", icon: Flame, shortcut: "s" },
    { to: "/templates", label: "Templates", icon: BookTemplate, shortcut: "t" },
    { to: "/validate", label: "Validate", icon: ShieldCheck, shortcut: "v" },
    { to: "/signals", label: "Signals", icon: Radio, shortcut: "g" },
    { to: "---3", label: "divider", icon: null as never, shortcut: "" },
    { to: "/settings", label: "Settings", icon: SettingsIcon, shortcut: "," },
    { to: "/about", label: t.about, icon: Info, shortcut: "a" },
  ];
}

export function useKeyboardShortcuts(navigate: (path: string) => void, setStartOpen: (open: boolean) => void, setCollapsed: React.Dispatch<React.SetStateAction<boolean>>) {
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement).tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

      if (e.altKey) {
        switch (e.key) {
          case "1": e.preventDefault(); navigate("/"); break;
          case "2": e.preventDefault(); navigate("/executions"); break;
          case "3": e.preventDefault(); navigate("/definitions"); break;
          case "4": e.preventDefault(); navigate("/taskdefs"); break;
          case "5": e.preventDefault(); navigate("/queues"); break;
          case "6": e.preventDefault(); navigate("/schedules"); break;
          case "7": e.preventDefault(); navigate("/metrics"); break;
          case "8": e.preventDefault(); navigate("/dependencies"); break;
          case "9": e.preventDefault(); navigate("/compare"); break;
          case "0": e.preventDefault(); navigate("/diff"); break;
          case "d": e.preventDefault(); navigate("/designer"); break;
          case "s": e.preventDefault(); navigate("/stresser"); break;
          case "t": e.preventDefault(); navigate("/templates"); break;
          case "a": e.preventDefault(); navigate("/about"); break;
          case "n": e.preventDefault(); setStartOpen(true); break;
          case "b": e.preventDefault(); setCollapsed((c) => !c); break;
        }
      }
    },
    [navigate, setStartOpen, setCollapsed],
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);
}

export function PageTransition({ locationKey, children }: { locationKey: string; children: React.ReactNode }) {
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
