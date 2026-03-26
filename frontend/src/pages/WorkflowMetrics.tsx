import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { workflowApi, metadataApi } from "@/api/conductor";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { SearchableSelect } from "@/components/SearchableSelect";
import { useChartColors } from "@/hooks/useChartColors";
import {
  BarChart,
  Bar,
  PieChart,
  Pie,
  Cell,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from "recharts";
import { Activity, Clock, CheckCircle, XCircle, Timer, TrendingUp } from "lucide-react";

function formatDuration(ms: number | undefined): string {
  if (ms == null) return "—";
  if (ms < 1000) return `${ms}ms`;
  const secs = ms / 1000;
  if (secs < 60) return `${secs.toFixed(1)}s`;
  const mins = secs / 60;
  if (mins < 60) return `${mins.toFixed(1)}m`;
  return `${(mins / 60).toFixed(1)}h`;
}

const STATUS_COLORS: Record<string, string> = {
  COMPLETED: "#10b981",
  RUNNING: "#3b82f6",
  FAILED: "#ef4444",
  TIMED_OUT: "#f97316",
  TERMINATED: "#6b7280",
  PAUSED: "#8b5cf6",
};

export default function WorkflowMetrics() {
  const [selectedName, setSelectedName] = useState("");
  const colors = useChartColors();

  const { data: defs } = useQuery({
    queryKey: ["workflow-defs-list"],
    queryFn: () => metadataApi.listWorkflowDefs(),
  });

  const { data: metrics, isLoading } = useQuery({
    queryKey: ["workflow-metrics", selectedName],
    queryFn: () => workflowApi.metrics(selectedName),
    enabled: !!selectedName,
  });

  const uniqueNames = defs
    ? [...new Set(defs.map((d) => d.name))].sort()
    : [];

  const statusData = metrics
    ? Object.entries(metrics.statusDistribution).map(([status, count]) => ({
        name: status,
        value: count,
        color: STATUS_COLORS[status] ?? "#9ca3af",
      }))
    : [];

  const durationData = metrics
    ? [
        { name: "Min", value: metrics.minDurationMs ?? 0 },
        { name: "P50", value: metrics.p50DurationMs ?? 0 },
        { name: "Avg", value: metrics.avgDurationMs ?? 0 },
        { name: "P95", value: metrics.p95DurationMs ?? 0 },
        { name: "Max", value: metrics.maxDurationMs ?? 0 },
      ]
    : [];

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold tracking-tight">Workflow Metrics</h2>
      </div>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm font-medium">Select Workflow Definition</CardTitle>
        </CardHeader>
        <CardContent>
          <SearchableSelect
            options={uniqueNames.map((n) => ({ label: n, value: n }))}
            value={selectedName}
            onChange={setSelectedName}
            placeholder="Choose a workflow type..."
            searchPlaceholder="Search workflow types..."
            className="max-w-md"
          />
        </CardContent>
      </Card>

      {selectedName && isLoading && (
        <p className="text-sm text-muted-foreground">Loading metrics...</p>
      )}

      {metrics && metrics.sampleSize > 0 && (
        <>
          {/* Summary cards */}
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
            <MetricCard
              icon={<Activity className="h-4 w-4 text-blue-500" />}
              label="Sample Size"
              value={String(metrics.sampleSize)}
              subtitle="last workflows"
            />
            <MetricCard
              icon={<CheckCircle className="h-4 w-4 text-emerald-500" />}
              label="Success Rate"
              value={`${metrics.successRate.toFixed(1)}%`}
              subtitle="completed"
            />
            <MetricCard
              icon={<XCircle className="h-4 w-4 text-red-500" />}
              label="Failure Rate"
              value={`${metrics.failureRate.toFixed(1)}%`}
              subtitle="failed + timed out"
            />
            <MetricCard
              icon={<Clock className="h-4 w-4 text-amber-500" />}
              label="Avg Duration"
              value={formatDuration(metrics.avgDurationMs)}
              subtitle="mean"
            />
            <MetricCard
              icon={<Timer className="h-4 w-4 text-purple-500" />}
              label="P50 Duration"
              value={formatDuration(metrics.p50DurationMs)}
              subtitle="median"
            />
            <MetricCard
              icon={<TrendingUp className="h-4 w-4 text-orange-500" />}
              label="P95 Duration"
              value={formatDuration(metrics.p95DurationMs)}
              subtitle="95th percentile"
            />
          </div>

          {/* Charts */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {/* Status Distribution Pie */}
            <Card>
              <CardHeader>
                <CardTitle className="text-sm font-medium">Status Distribution</CardTitle>
              </CardHeader>
              <CardContent>
                <ResponsiveContainer width="100%" height={300}>
                  <PieChart>
                    <Pie
                      data={statusData}
                      cx="50%"
                      cy="50%"
                      innerRadius={60}
                      outerRadius={100}
                      paddingAngle={2}
                      dataKey="value"
                      label={({ name, value }) => `${name}: ${value}`}
                    >
                      {statusData.map((entry) => (
                        <Cell key={entry.name} fill={entry.color} />
                      ))}
                    </Pie>
                    <Tooltip
                      contentStyle={{
                        backgroundColor: colors.tooltip.bg,
                        borderColor: colors.tooltip.border,
                        borderRadius: "8px",
                      }}
                    />
                    <Legend />
                  </PieChart>
                </ResponsiveContainer>
              </CardContent>
            </Card>

            {/* Duration Distribution Bar */}
            <Card>
              <CardHeader>
                <CardTitle className="text-sm font-medium">Duration Percentiles</CardTitle>
              </CardHeader>
              <CardContent>
                <ResponsiveContainer width="100%" height={300}>
                  <BarChart data={durationData}>
                    <CartesianGrid strokeDasharray="3 3" stroke={colors.grid} />
                    <XAxis dataKey="name" stroke={colors.axis} tick={{ fill: colors.text, fontSize: 12 }} />
                    <YAxis
                      stroke={colors.axis}
                      tick={{ fill: colors.text, fontSize: 12 }}
                      tickFormatter={(v) => formatDuration(v)}
                    />
                    <Tooltip
                      formatter={(value) => [formatDuration(Number(value)), "Duration"]}
                      contentStyle={{
                        backgroundColor: colors.tooltip.bg,
                        borderColor: colors.tooltip.border,
                        borderRadius: "8px",
                      }}
                    />
                    <Bar dataKey="value" fill={colors.running} radius={[4, 4, 0, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              </CardContent>
            </Card>
          </div>

          {/* Duration details table */}
          <Card>
            <CardHeader>
              <CardTitle className="text-sm font-medium">Duration Summary</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-5 gap-4 text-center">
                {[
                  { label: "Minimum", value: metrics.minDurationMs },
                  { label: "P50 (Median)", value: metrics.p50DurationMs },
                  { label: "Average", value: metrics.avgDurationMs },
                  { label: "P95", value: metrics.p95DurationMs },
                  { label: "Maximum", value: metrics.maxDurationMs },
                ].map((item) => (
                  <div key={item.label} className="space-y-1">
                    <p className="text-xs text-muted-foreground">{item.label}</p>
                    <p className="text-lg font-semibold font-mono">{formatDuration(item.value)}</p>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </>
      )}

      {metrics && metrics.sampleSize === 0 && (
        <Card>
          <CardContent className="py-12 text-center">
            <p className="text-muted-foreground">No workflow executions found for <strong>{selectedName}</strong>.</p>
          </CardContent>
        </Card>
      )}

      {!selectedName && (
        <Card>
          <CardContent className="py-12 text-center">
            <Activity className="h-12 w-12 text-muted-foreground/30 mx-auto mb-4" />
            <p className="text-muted-foreground">Select a workflow definition above to view its metrics.</p>
            <p className="text-xs text-muted-foreground mt-1">Metrics are computed from the last 100 executions.</p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

function MetricCard({
  icon,
  label,
  value,
  subtitle,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  subtitle: string;
}) {
  return (
    <Card>
      <CardContent className="pt-6">
        <div className="flex items-center gap-2 mb-2">
          {icon}
          <span className="text-xs text-muted-foreground">{label}</span>
        </div>
        <p className="text-2xl font-bold font-mono">{value}</p>
        <p className="text-xs text-muted-foreground mt-1">{subtitle}</p>
      </CardContent>
    </Card>
  );
}
