import { useState, useMemo, useCallback } from "react";
import { cn } from "@/lib/utils";
import { CopyButton } from "@/components/CopyButton";
import { ChevronRight, ChevronDown } from "lucide-react";

interface JsonViewProps {
  data: unknown;
  className?: string;
  maxHeight?: string;
  copyable?: boolean;
  defaultExpanded?: number; // depth to auto-expand, default 2
}

// VS Code-style color classes
const COLORS = {
  key: "text-blue-600 dark:text-blue-400",
  string: "text-emerald-600 dark:text-emerald-400",
  number: "text-amber-600 dark:text-amber-400",
  boolean: "text-purple-600 dark:text-purple-400",
  null: "text-red-500 dark:text-red-400",
  brace: "text-muted-foreground",
  count: "text-muted-foreground",
};

function CollapsibleNode({
  keyName,
  value,
  depth,
  defaultExpanded,
  isLast,
}: {
  keyName?: string;
  value: unknown;
  depth: number;
  defaultExpanded: number;
  isLast: boolean;
}) {
  const isObject = value !== null && typeof value === "object" && !Array.isArray(value);
  const isArray = Array.isArray(value);
  const isExpandable = isObject || isArray;
  const [expanded, setExpanded] = useState(depth < defaultExpanded);
  const indent = depth * 16;

  const toggle = useCallback(() => setExpanded((e) => !e), []);
  const comma = isLast ? "" : ",";

  if (!isExpandable) {
    // Primitive value
    return (
      <div className="flex items-start" style={{ paddingLeft: indent }}>
        <span className="w-4 shrink-0" />
        {keyName !== undefined && (
          <>
            <span className={COLORS.key}>"{keyName}"</span>
            <span className={COLORS.brace}>:&nbsp;</span>
          </>
        )}
        <PrimitiveValue value={value} />
        <span className={COLORS.brace}>{comma}</span>
      </div>
    );
  }

  const entries = isArray
    ? (value as unknown[]).map((v, i) => ({ key: String(i), value: v }))
    : Object.entries(value as Record<string, unknown>).map(([k, v]) => ({ key: k, value: v }));

  const openBrace = isArray ? "[" : "{";
  const closeBrace = isArray ? "]" : "}";
  const countLabel = isArray
    ? `${entries.length} item${entries.length !== 1 ? "s" : ""}`
    : `${entries.length} key${entries.length !== 1 ? "s" : ""}`;

  if (!expanded) {
    return (
      <div
        className="flex items-center cursor-pointer hover:bg-muted rounded-sm"
        style={{ paddingLeft: indent }}
        onClick={toggle}
      >
        <ChevronRight className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
        {keyName !== undefined && (
          <>
            <span className={COLORS.key}>"{keyName}"</span>
            <span className={COLORS.brace}>:&nbsp;</span>
          </>
        )}
        <span className={COLORS.brace}>{openBrace}</span>
        <span className={cn(COLORS.count, "text-[10px] mx-1 italic")}>{countLabel}</span>
        <span className={COLORS.brace}>{closeBrace}</span>
        <span className={COLORS.brace}>{comma}</span>
      </div>
    );
  }

  return (
    <div>
      <div
        className="flex items-center cursor-pointer hover:bg-muted rounded-sm"
        style={{ paddingLeft: indent }}
        onClick={toggle}
      >
        <ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
        {keyName !== undefined && (
          <>
            <span className={COLORS.key}>"{keyName}"</span>
            <span className={COLORS.brace}>:&nbsp;</span>
          </>
        )}
        <span className={COLORS.brace}>{openBrace}</span>
      </div>
      {entries.map((entry, i) => (
        <CollapsibleNode
          key={entry.key}
          keyName={isArray ? undefined : entry.key}
          value={entry.value}
          depth={depth + 1}
          defaultExpanded={defaultExpanded}
          isLast={i === entries.length - 1}
        />
      ))}
      <div style={{ paddingLeft: indent }}>
        <span className="w-4 inline-block" />
        <span className={COLORS.brace}>{closeBrace}</span>
        <span className={COLORS.brace}>{comma}</span>
      </div>
    </div>
  );
}

function PrimitiveValue({ value }: { value: unknown }) {
  if (value === null) return <span className={COLORS.null}>null</span>;
  if (typeof value === "boolean") return <span className={COLORS.boolean}>{String(value)}</span>;
  if (typeof value === "number") return <span className={COLORS.number}>{String(value)}</span>;
  if (typeof value === "string") {
    // Truncate very long strings in the display
    const display = value.length > 200 ? value.slice(0, 200) + "..." : value;
    return <span className={COLORS.string}>"{display}"</span>;
  }
  return <span>{String(value)}</span>;
}

export function JsonView({ data, className, maxHeight = "24rem", copyable = true, defaultExpanded = 2 }: JsonViewProps) {
  const jsonStr = useMemo(() => {
    try {
      return JSON.stringify(data, null, 2);
    } catch {
      return String(data);
    }
  }, [data]);

  if (data === undefined || data === null) {
    return <span className="text-xs text-muted-foreground italic">null</span>;
  }

  return (
    <div className={cn("relative group rounded-lg bg-muted", className)}>
      {copyable && (
        <div className="absolute top-2 right-2 z-10 opacity-0 group-hover:opacity-100 transition-opacity">
          <CopyButton value={jsonStr} />
        </div>
      )}
      <div
        className="text-xs p-3 overflow-auto font-mono leading-relaxed select-text"
        style={{ maxHeight }}
      >
        <CollapsibleNode
          value={data}
          depth={0}
          defaultExpanded={defaultExpanded}
          isLast
        />
      </div>
    </div>
  );
}
