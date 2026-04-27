import { useState, useEffect } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { AnimatedCounter } from "@/components/AnimatedCounter";
import { LineChart, Line, ResponsiveContainer } from "recharts";

const SPARKLINE_KEY = "dashboard-sparkline-history";
const SPARKLINE_POINTS = 12;

function loadSparklineHistory(): Record<string, number[]> {
  try {
    const stored = localStorage.getItem(SPARKLINE_KEY);
    if (stored) return JSON.parse(stored);
  } catch { /* ignore */ }
  return {};
}

function pushSparklineValue(key: string, value: number) {
  const history = loadSparklineHistory();
  const arr = history[key] ?? [];
  arr.push(value);
  if (arr.length > SPARKLINE_POINTS) arr.splice(0, arr.length - SPARKLINE_POINTS);
  history[key] = arr;
  localStorage.setItem(SPARKLINE_KEY, JSON.stringify(history));
  return arr;
}

function useSparkline(key: string, value: number) {
  const [data, setData] = useState<{ v: number }[]>([]);
  useEffect(() => {
    const arr = pushSparklineValue(key, value);
    setData(arr.map((v) => ({ v })));
  }, [key, value]);
  return data;
}

export function StatCard({
  icon: Icon,
  label,
  value,
  color,
  barColor,
  sparklineKey,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: number;
  color: string;
  barColor?: string;
  sparklineKey: string;
}) {
  const sparkData = useSparkline(sparklineKey, value);
  const strokeColor = color.replace("text-", "").includes("blue") ? "#3b82f6" : color.includes("emerald") ? "#10b981" : color.includes("red") ? "#ef4444" : "#f59e0b";

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-sm font-medium">{label}</CardTitle>
        <Icon className={`h-4 w-4 ${color}`} />
      </CardHeader>
      <CardContent>
        <div className="flex items-end justify-between gap-2">
          <div className="text-2xl font-bold">
            <AnimatedCounter value={value} />
          </div>
          {sparkData.length > 1 && (
            <div className="h-8 w-20">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={sparkData}>
                  <Line type="monotone" dataKey="v" stroke={strokeColor} strokeWidth={1.5} dot={false} />
                </LineChart>
              </ResponsiveContainer>
            </div>
          )}
        </div>
        {barColor && value > 0 && (
          <div className="mt-2 h-1 rounded-full bg-muted overflow-hidden">
            <div className={`h-full ${barColor} rounded-full animate-in fade-in-0 slide-in-from-left-1/2`} style={{ width: "100%" }} />
          </div>
        )}
      </CardContent>
    </Card>
  );
}
