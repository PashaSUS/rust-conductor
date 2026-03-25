import { useState, useMemo, useCallback } from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { CopyButton } from "@/components/CopyButton";
import { ChevronRight, ChevronDown, Search, ArrowLeft } from "lucide-react";

interface Props {
  data: unknown;
  maxHeight?: string;
}

type PathSegment = string | number;

function getType(val: unknown): string {
  if (val === null) return "null";
  if (Array.isArray(val)) return "array";
  return typeof val;
}

function getPreview(val: unknown): string {
  if (val === null) return "null";
  if (typeof val === "string") return val.length > 60 ? `"${val.slice(0, 60)}…"` : `"${val}"`;
  if (typeof val === "number" || typeof val === "boolean") return String(val);
  if (Array.isArray(val)) return `Array(${val.length})`;
  if (typeof val === "object") return `{${Object.keys(val as object).length} keys}`;
  return String(val);
}

function getAtPath(data: unknown, path: PathSegment[]): unknown {
  let current = data;
  for (const seg of path) {
    if (current == null) return undefined;
    current = (current as Record<string, unknown>)[String(seg)];
  }
  return current;
}

/**
 * #175 — Task output data explorer.
 * Interactive, navigable JSON explorer with breadcrumb path,
 * type badges, search, and copy-to-clipboard.
 */
export function DataExplorer({ data, maxHeight = "28rem" }: Props) {
  const [path, setPath] = useState<PathSegment[]>([]);
  const [searchTerm, setSearchTerm] = useState("");
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  const currentValue = useMemo(() => getAtPath(data, path), [data, path]);
  const currentType = getType(currentValue);

  const navigateTo = useCallback((seg: PathSegment) => {
    setPath((prev) => [...prev, seg]);
    setExpanded(new Set());
  }, []);

  const navigateBack = useCallback(() => {
    setPath((prev) => prev.slice(0, -1));
    setExpanded(new Set());
  }, []);

  const navigateToIndex = useCallback((idx: number) => {
    setPath((prev) => prev.slice(0, idx));
    setExpanded(new Set());
  }, []);

  const toggleExpand = useCallback((key: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }, []);

  const entries = useMemo(() => {
    if (currentType !== "object" && currentType !== "array") return [];
    const obj = currentValue as Record<string, unknown>;
    let items = Object.entries(obj);
    if (searchTerm) {
      const lower = searchTerm.toLowerCase();
      items = items.filter(
        ([k, v]) =>
          k.toLowerCase().includes(lower) ||
          String(v).toLowerCase().includes(lower)
      );
    }
    return items;
  }, [currentValue, currentType, searchTerm]);

  return (
    <div className="space-y-2">
      {/* Breadcrumb path */}
      <div className="flex items-center gap-1 text-xs flex-wrap">
        {path.length > 0 && (
          <Button variant="ghost" size="icon" className="h-5 w-5" onClick={navigateBack}>
            <ArrowLeft className="h-3 w-3" />
          </Button>
        )}
        <button
          className="text-muted-foreground hover:text-foreground transition-colors"
          onClick={() => navigateToIndex(0)}
        >
          root
        </button>
        {path.map((seg, i) => (
          <span key={i} className="flex items-center gap-1">
            <span className="text-muted-foreground">/</span>
            <button
              className="text-muted-foreground hover:text-foreground transition-colors font-mono"
              onClick={() => navigateToIndex(i + 1)}
            >
              {String(seg)}
            </button>
          </span>
        ))}
        <Badge variant="secondary" className="ml-1 text-[10px]">{currentType}</Badge>
        <CopyButton value={JSON.stringify(currentValue, null, 2)} className="ml-auto" />
      </div>

      {/* Search for object/array */}
      {(currentType === "object" || currentType === "array") && entries.length > 5 && (
        <div className="relative">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground" />
          <Input
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            placeholder="Filter keys or values..."
            className="pl-7 h-7 text-xs"
          />
        </div>
      )}

      {/* Content */}
      <div className="overflow-auto rounded border bg-muted p-2" style={{ maxHeight }}>
        {currentType === "object" || currentType === "array" ? (
          <div className="space-y-0.5">
            {entries.length === 0 && (
              <p className="text-xs text-muted-foreground italic py-2">
                {searchTerm ? "No matching entries" : "Empty"}
              </p>
            )}
            {entries.map(([key, val]) => {
              const valType = getType(val);
              const isExpandable = valType === "object" || valType === "array";
              const isExpanded = expanded.has(key);

              return (
                <div key={key}>
                  <div className="flex items-center gap-1 py-0.5 hover:bg-accent rounded px-1 group">
                    {isExpandable ? (
                      <button onClick={() => toggleExpand(key)} className="h-4 w-4 shrink-0">
                        {isExpanded ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
                      </button>
                    ) : (
                      <span className="w-4 shrink-0" />
                    )}
                    <span
                      className={`font-mono text-xs ${isExpandable ? "text-blue-500 cursor-pointer hover:underline" : "text-foreground"}`}
                      onClick={() => isExpandable && navigateTo(currentType === "array" ? Number(key) : key)}
                    >
                      {key}
                    </span>
                    <span className="text-muted-foreground text-[10px]">:</span>
                    <Badge variant="outline" className="text-[9px] px-1 py-0">{valType}</Badge>
                    <span className="text-xs text-muted-foreground truncate flex-1 min-w-0">
                      {getPreview(val)}
                    </span>
                    <CopyButton value={JSON.stringify(val, null, 2)} className="opacity-0 group-hover:opacity-100 h-5 w-5" />
                  </div>
                  {isExpanded && (
                    <div className="ml-5 pl-2 border-l border-border">
                      {Object.entries(val as Record<string, unknown>).slice(0, 50).map(([subKey, subVal]) => (
                        <div key={subKey} className="flex items-center gap-1 py-0.5 text-xs">
                          <span className="w-4 shrink-0" />
                          <span className="font-mono text-muted-foreground">{subKey}</span>
                          <span className="text-muted-foreground text-[10px]">:</span>
                          <span className="text-xs truncate">{getPreview(subVal)}</span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        ) : (
          <div className="text-sm font-mono p-2 whitespace-pre-wrap break-all">
            {currentValue === null ? (
              <span className="text-muted-foreground italic">null</span>
            ) : typeof currentValue === "string" ? (
              <span className="text-green-600 dark:text-green-400">"{currentValue}"</span>
            ) : typeof currentValue === "number" ? (
              <span className="text-blue-600 dark:text-blue-400">{currentValue}</span>
            ) : typeof currentValue === "boolean" ? (
              <span className="text-purple-600 dark:text-purple-400">{String(currentValue)}</span>
            ) : (
              String(currentValue)
            )}
          </div>
        )}
      </div>
    </div>
  );
}
