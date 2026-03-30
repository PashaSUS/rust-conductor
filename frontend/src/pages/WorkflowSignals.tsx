import { useState } from "react";
import { workflowApi, type SignalResponse } from "@/api/conductor";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Send, CheckCircle } from "lucide-react";

export default function WorkflowSignals() {
  const [signalName, setSignalName] = useState("");
  const [payloadText, setPayloadText] = useState("{}");
  const [sending, setSending] = useState(false);
  const [lastResult, setLastResult] = useState<SignalResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleSend = async () => {
    setSending(true);
    setError(null);
    setLastResult(null);
    try {
      let payload: unknown;
      try {
        payload = JSON.parse(payloadText);
      } catch {
        throw new Error("Invalid JSON payload");
      }
      const result = await workflowApi.sendSignal({ signalName, payload });
      setLastResult(result);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSending(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold tracking-tight">Workflow Signals</h1>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">Send Signal</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-sm text-muted-foreground">
            Send a signal to all workflows with WAIT_FOR_SIGNAL tasks matching the signal name.
          </p>
          <div>
            <label className="text-sm font-medium text-muted-foreground block mb-1">
              Signal Name
            </label>
            <input
              className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
              placeholder="e.g. order-approved, payment-received"
              value={signalName}
              onChange={(e) => setSignalName(e.target.value)}
            />
          </div>
          <div>
            <label className="text-sm font-medium text-muted-foreground block mb-1">
              Payload (JSON)
            </label>
            <textarea
              className="w-full h-32 rounded-md border border-input bg-background px-3 py-2 text-sm font-mono"
              value={payloadText}
              onChange={(e) => setPayloadText(e.target.value)}
            />
          </div>
          <Button
            onClick={handleSend}
            disabled={!signalName.trim() || sending}
          >
            <Send className="h-4 w-4 mr-1" />
            {sending ? "Sending..." : "Send Signal"}
          </Button>

          {lastResult && (
            <div className="flex items-center gap-2 text-sm bg-green-50 dark:bg-green-950/20 rounded-md p-3">
              <CheckCircle className="h-4 w-4 text-green-500" />
              <span>
                Signal <strong>{lastResult.signalName}</strong> delivered to{" "}
                <Badge variant="secondary">{lastResult.deliveredTo}</Badge> waiting workflow
                {lastResult.deliveredTo !== 1 ? "s" : ""}.
              </span>
            </div>
          )}

          {error && (
            <div className="text-sm bg-red-50 dark:bg-red-950/20 rounded-md p-3 text-red-600 dark:text-red-400">
              {error}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
