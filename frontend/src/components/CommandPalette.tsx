import { useState, useEffect, useCallback, useRef } from "react";
import { useNavigate } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { metadataApi, workflowApi } from "@/api/conductor";
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  FileCode2,
  ListChecks,
  Play,
  LayoutDashboard,
  Layers,
  Search,
  Calendar,
} from "lucide-react";
import { useThemeText } from "@/components/ThemeContext";

interface PaletteItem {
  id: string;
  label: string;
  sublabel?: string;
  icon: React.ComponentType<{ className?: string }>;
  action: () => void;
  category: string;
}

export function CommandPalette() {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const navigate = useNavigate();
  const t = useThemeText();
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Fetch data for search
  const { data: wfDefs } = useQuery({
    queryKey: ["wf-defs"],
    queryFn: metadataApi.listWorkflowDefs,
    enabled: open,
  });
  const { data: taskDefs } = useQuery({
    queryKey: ["task-defs"],
    queryFn: metadataApi.listTaskDefs,
    enabled: open,
  });
  const { data: recentExecs } = useQuery({
    queryKey: ["workflows-recent"],
    queryFn: () => workflowApi.search({ size: 20 }),
    enabled: open,
  });

  // Global Ctrl+K listener
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        setOpen((prev) => !prev);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  // Reset on open
  useEffect(() => {
    if (open) {
      setQuery("");
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  const go = useCallback(
    (path: string) => {
      setOpen(false);
      navigate(path);
    },
    [navigate]
  );

  // Build items list
  const items: PaletteItem[] = [];

  // Pages
  const pages: PaletteItem[] = [
    { id: "p-dash", label: t.dashboard, icon: LayoutDashboard, action: () => go("/"), category: t.pages },
    { id: "p-exec", label: t.executions, icon: Play, action: () => go("/executions"), category: t.pages },
    { id: "p-wfdef", label: t.workflowDefs, icon: FileCode2, action: () => go("/definitions"), category: t.pages },
    { id: "p-tdef", label: t.taskDefs, icon: ListChecks, action: () => go("/taskdefs"), category: t.pages },
    { id: "p-queues", label: t.taskQueues, icon: Layers, action: () => go("/queues"), category: t.pages },
    { id: "p-schedules", label: t.schedules, icon: Calendar, action: () => go("/schedules"), category: t.pages },
  ];
  items.push(...pages);

  // Workflow definitions
  if (wfDefs) {
    for (const def of wfDefs) {
      items.push({
        id: `wdef-${def.name}-${def.version}`,
        label: def.name,
        sublabel: `v${def.version} · ${t.workflowDefs}`,
        icon: FileCode2,
        action: () => go("/definitions"),
        category: t.workflowDefinitions,
      });
    }
  }

  // Task definitions
  if (taskDefs) {
    for (const def of taskDefs) {
      items.push({
        id: `tdef-${def.name}`,
        label: def.name,
        sublabel: t.taskDefs,
        icon: ListChecks,
        action: () => go("/taskdefs"),
        category: t.taskDefinitions,
      });
    }
  }

  // Recent executions
  if (recentExecs?.results) {
    for (const exec of recentExecs.results) {
      items.push({
        id: `exec-${exec.workflowId}`,
        label: exec.workflowType,
        sublabel: `${exec.status} · ${exec.workflowId.slice(0, 8)}…`,
        icon: Play,
        action: () => go(`/executions/${exec.workflowId}`),
        category: t.recentExecutions,
      });
    }
  }

  // Filter
  const q = query.toLowerCase().trim();
  const filtered = q
    ? items.filter(
        (item) =>
          item.label.toLowerCase().includes(q) ||
          (item.sublabel && item.sublabel.toLowerCase().includes(q))
      )
    : items;

  // Group by category
  const grouped: { category: string; items: PaletteItem[] }[] = [];
  for (const item of filtered) {
    const existing = grouped.find((g) => g.category === item.category);
    if (existing) existing.items.push(item);
    else grouped.push({ category: item.category, items: [item] });
  }

  const flatFiltered = grouped.flatMap((g) => g.items);

  // Keyboard navigation
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, flatFiltered.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter" && flatFiltered[selectedIndex]) {
      e.preventDefault();
      flatFiltered[selectedIndex].action();
    } else if (e.key === "Escape") {
      setOpen(false);
    }
  };

  // Scroll selected item into view
  useEffect(() => {
    const el = listRef.current?.querySelector(`[data-idx="${selectedIndex}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [selectedIndex]);

  // Reset selection when query changes
  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  let flatIndex = -1;

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent className="p-0 gap-0 max-w-lg [&>button]:hidden overflow-hidden">
        <DialogTitle className="sr-only">{t.commandPalette}</DialogTitle>
        <div className="flex items-center border-b px-3">
          <Search className="h-4 w-4 text-muted-foreground shrink-0" />
          <Input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={t.searchAll}
            className="border-0 focus-visible:ring-0 focus-visible:ring-offset-0 h-12 text-sm"
          />
          <kbd className="pointer-events-none hidden sm:inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground">
            ESC
          </kbd>
        </div>
        <div ref={listRef} className="max-h-80 overflow-y-auto p-2">
          {grouped.length === 0 && (
            <p className="text-center text-sm text-muted-foreground py-6">
              {t.noResults}
            </p>
          )}
          {grouped.map((group) => (
            <div key={group.category}>
              <p className="text-xs font-medium text-muted-foreground px-2 py-1.5">
                {group.category}
              </p>
              {group.items.map((item) => {
                flatIndex++;
                const idx = flatIndex;
                const Icon = item.icon;
                return (
                  <button
                    key={item.id}
                    data-idx={idx}
                    className={`flex items-center gap-3 w-full rounded-md px-2 py-2 text-sm text-left transition-colors ${
                      idx === selectedIndex
                        ? "bg-accent text-accent-foreground"
                        : "hover:bg-accent/50"
                    }`}
                    onClick={item.action}
                    onMouseEnter={() => setSelectedIndex(idx)}
                  >
                    <Icon className="h-4 w-4 text-muted-foreground shrink-0" />
                    <div className="flex-1 min-w-0">
                      <span className="truncate block">{item.label}</span>
                      {item.sublabel && (
                        <span className="text-xs text-muted-foreground truncate block">
                          {item.sublabel}
                        </span>
                      )}
                    </div>
                  </button>
                );
              })}
            </div>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  );
}
