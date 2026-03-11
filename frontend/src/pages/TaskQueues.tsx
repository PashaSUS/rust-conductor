import { useQuery } from "@tanstack/react-query";
import { taskApi } from "@/api/conductor";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Layers } from "lucide-react";

export default function TaskQueues() {
  const { data: sizes, isLoading } = useQuery({
    queryKey: ["queue-sizes"],
    queryFn: taskApi.queueSizes,
    refetchInterval: 5000,
  });

  const entries = Object.entries(sizes ?? {}).sort((a, b) => b[1] - a[1]);
  const total = entries.reduce((sum, [, v]) => sum + v, 0);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold tracking-tight">Task Queues</h2>
        <Badge variant="secondary" className="gap-1">
          <Layers className="h-3 w-3" />
          {total} total queued
        </Badge>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium">Active Queues</CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <p className="text-muted-foreground text-sm">Loading...</p>
          ) : entries.length === 0 ? (
            <p className="text-muted-foreground text-sm">No tasks in queue</p>
          ) : (
            <div className="space-y-3">
              {entries.map(([name, count]) => (
                <div key={name} className="flex items-center justify-between rounded-lg border p-3">
                  <div>
                    <span className="font-medium text-sm">{name}</span>
                  </div>
                  <div className="flex items-center gap-3">
                    <div className="w-32 bg-muted rounded-full h-2">
                      <div
                        className="bg-chart-1 h-2 rounded-full transition-all"
                        style={{ width: `${Math.min(100, (count / Math.max(total, 1)) * 100)}%` }}
                      />
                    </div>
                    <Badge variant="secondary">{count}</Badge>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
