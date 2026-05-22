import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { Settings as SettingsIcon, Server, Power } from "lucide-react";
import { toast } from "sonner";
import {
  getApiBase,
  setApiBase,
  isLiteMode,
  setLiteMode,
} from "@/lib/config";

export default function Settings() {
  const [url, setUrl] = useState(() => getApiBase());
  const [lite, setLite] = useState(() => isLiteMode());

  const save = () => {
    setApiBase(url || null);
    setLiteMode(lite);
    toast.success("Settings saved. Reloading…");
    setTimeout(() => window.location.reload(), 500);
  };

  const reset = () => {
    setApiBase(null);
    setLiteMode(false);
    toast.success("Settings reset. Reloading…");
    setTimeout(() => window.location.reload(), 500);
  };

  return (
    <div className="flex flex-col gap-4 max-w-3xl">
      <div className="flex items-center gap-2">
        <SettingsIcon className="h-6 w-6" />
        <h2 className="text-2xl font-bold tracking-tight">Settings</h2>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <Server className="h-4 w-4" />
            Conductor API
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <label className="text-sm font-medium block mb-1">
              API Base URL
            </label>
            <Input
              type="url"
              placeholder="https://conductor.example.com (leave blank for same origin)"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
            />
            <p className="text-xs text-muted-foreground mt-1">
              The base URL (no trailing <code>/api</code>) of the Conductor
              server. Leave blank to use the default (same origin /{" "}
              <code>VITE_API_BASE</code>).
            </p>
          </div>

          <div className="flex items-start gap-3 rounded-md border p-3">
            <Checkbox
              checked={lite}
              onCheckedChange={(v) => setLite(v === true)}
              className="mt-0.5"
            />
            <div className="flex-1">
              <div className="text-sm font-medium flex items-center gap-2">
                <Power className="h-3.5 w-3.5" />
                Lite mode (base Conductor compatibility)
              </div>
              <p className="text-xs text-muted-foreground mt-1">
                Hides rust-conductor-specific features (signals, checkpoints,
                designer, stresser, templates, diff, compare, validation,
                dependency graph, metrics, schedules, dashboard) so the UI
                works against a stock Netflix Conductor OSS REST API.
              </p>
            </div>
          </div>

          <div className="flex gap-2">
            <Button onClick={save}>Save & Reload</Button>
            <Button variant="outline" onClick={reset}>
              Reset to defaults
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
