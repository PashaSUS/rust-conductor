import { useState, useEffect } from "react";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";

function getRelativeTime(ts: number): string {
  const now = Date.now();
  const diff = now - ts;

  if (diff < 0) return "just now";
  if (diff < 5000) return "just now";
  if (diff < 60000) return `${Math.floor(diff / 1000)}s ago`;
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
  if (diff < 604800000) return `${Math.floor(diff / 86400000)}d ago`;
  return new Date(ts).toLocaleDateString();
}

/** Parse a timestamp that may be epoch millis (number or numeric string) or an ISO string. */
function parseTs(v: string | number | null | undefined): number | null {
  if (v == null) return null;
  const n = typeof v === "number" ? v : Number(v);
  if (Number.isFinite(n) && n > 1e12) return n;
  const d = new Date(v);
  return Number.isNaN(d.getTime()) ? null : d.getTime();
}

interface RelativeTimeProps {
  value: string | number | null | undefined;
  live?: boolean;
  className?: string;
}

export function RelativeTime({ value, live = true, className }: RelativeTimeProps) {
  const [, setTick] = useState(0);
  const ts = parseTs(value);

  useEffect(() => {
    if (!live || ts === null) return;
    const interval = setInterval(() => setTick((t) => t + 1), 10000);
    return () => clearInterval(interval);
  }, [live, ts]);

  if (ts === null) return <span className={className}>—</span>;

  const absolute = new Date(ts).toLocaleString();
  const relative = getRelativeTime(ts);

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <span className={className} title={absolute}>
            {relative}
          </span>
        </TooltipTrigger>
        <TooltipContent>
          <p className="text-xs">{absolute}</p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
