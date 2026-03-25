import { cn } from "@/lib/utils";

interface ProgressBarProps {
  value: number;
  max: number;
  className?: string;
  color?: "blue" | "emerald" | "amber" | "red";
  showLabel?: boolean;
}

const colorMap = {
  blue: "bg-blue-500",
  emerald: "bg-emerald-500",
  amber: "bg-amber-500",
  red: "bg-red-500",
};

/**
 * Animated progress/utilization bar with optional label.
 */
export function ProgressBar({ value, max, className, color = "blue", showLabel }: ProgressBarProps) {
  const pct = max > 0 ? Math.min((value / max) * 100, 100) : 0;
  const autoColor = pct > 90 ? "red" : pct > 70 ? "amber" : color;

  return (
    <div className={cn("flex items-center gap-2", className)}>
      <div className="flex-1 h-2 rounded-full bg-muted overflow-hidden">
        <div
          className={cn("h-full rounded-full transition-all duration-500 ease-out", colorMap[autoColor])}
          style={{ width: `${pct}%` }}
        />
      </div>
      {showLabel && (
        <span className="text-xs text-muted-foreground tabular-nums w-10 text-right">
          {Math.round(pct)}%
        </span>
      )}
    </div>
  );
}
