import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { metadataApi, type WorkflowDef, type ValidationResult } from "@/api/conductor";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { CheckCircle, XCircle, AlertTriangle, Search } from "lucide-react";

export default function WorkflowValidation() {
  const [selected, setSelected] = useState<string>("");
  const [version, setVersion] = useState<number>(1);
  const [result, setResult] = useState<ValidationResult | null>(null);
  const [validating, setValidating] = useState(false);
  const [jsonInput, setJsonInput] = useState("");
  const [mode, setMode] = useState<"select" | "json">("select");

  const { data: defs } = useQuery({
    queryKey: ["workflowDefs"],
    queryFn: () => metadataApi.listWorkflowDefs(),
  });

  const handleValidate = async () => {
    setValidating(true);
    setResult(null);
    try {
      let def: WorkflowDef;
      if (mode === "json") {
        def = JSON.parse(jsonInput) as WorkflowDef;
      } else {
        def = await metadataApi.getWorkflowDef(selected, version);
      }
      const res = await metadataApi.validateWorkflow(def);
      setResult(res);
    } catch (e) {
      setResult({
        valid: false,
        errors: [e instanceof Error ? e.message : String(e)],
        warnings: [],
      });
    } finally {
      setValidating(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold tracking-tight">Workflow Validation</h1>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">Validate Workflow Definition</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex gap-2">
            <Button
              variant={mode === "select" ? "default" : "outline"}
              size="sm"
              onClick={() => setMode("select")}
            >
              Select Definition
            </Button>
            <Button
              variant={mode === "json" ? "default" : "outline"}
              size="sm"
              onClick={() => setMode("json")}
            >
              Paste JSON
            </Button>
          </div>

          {mode === "select" ? (
            <div className="flex gap-2 items-end">
              <div className="flex-1">
                <label className="text-sm font-medium text-muted-foreground block mb-1">
                  Workflow
                </label>
                <select
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                  value={selected}
                  onChange={(e) => {
                    setSelected(e.target.value);
                    setResult(null);
                  }}
                >
                  <option value="">Select a workflow...</option>
                  {defs?.map((d) => (
                    <option key={`${d.name}:${d.version}`} value={d.name}>
                      {d.name} (v{d.version})
                    </option>
                  ))}
                </select>
              </div>
              <div className="w-24">
                <label className="text-sm font-medium text-muted-foreground block mb-1">
                  Version
                </label>
                <input
                  type="number"
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                  value={version}
                  onChange={(e) => setVersion(Number(e.target.value))}
                  min={1}
                />
              </div>
              <Button onClick={handleValidate} disabled={!selected || validating}>
                <Search className="h-4 w-4 mr-1" />
                {validating ? "Validating..." : "Validate"}
              </Button>
            </div>
          ) : (
            <div className="space-y-2">
              <textarea
                className="w-full h-48 rounded-md border border-input bg-background px-3 py-2 text-sm font-mono"
                placeholder="Paste workflow definition JSON..."
                value={jsonInput}
                onChange={(e) => {
                  setJsonInput(e.target.value);
                  setResult(null);
                }}
              />
              <Button onClick={handleValidate} disabled={!jsonInput.trim() || validating}>
                <Search className="h-4 w-4 mr-1" />
                {validating ? "Validating..." : "Validate"}
              </Button>
            </div>
          )}
        </CardContent>
      </Card>

      {result && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              {result.valid ? (
                <>
                  <CheckCircle className="h-5 w-5 text-green-500" />
                  <span className="text-green-700 dark:text-green-400">Valid</span>
                </>
              ) : (
                <>
                  <XCircle className="h-5 w-5 text-red-500" />
                  <span className="text-red-700 dark:text-red-400">Invalid</span>
                </>
              )}
              <Badge variant={result.valid ? "default" : "destructive"} className="ml-2">
                {result.errors.length} error{result.errors.length !== 1 ? "s" : ""}
              </Badge>
              {result.warnings.length > 0 && (
                <Badge variant="secondary">
                  {result.warnings.length} warning{result.warnings.length !== 1 ? "s" : ""}
                </Badge>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {result.errors.length > 0 && (
              <div className="space-y-2">
                <h4 className="text-sm font-semibold text-red-600 dark:text-red-400">Errors</h4>
                {result.errors.map((err, i) => (
                  <div
                    key={i}
                    className="flex items-start gap-2 text-sm bg-red-50 dark:bg-red-950/20 rounded-md p-2"
                  >
                    <XCircle className="h-4 w-4 text-red-500 mt-0.5 shrink-0" />
                    <span>{err}</span>
                  </div>
                ))}
              </div>
            )}
            {result.warnings.length > 0 && (
              <div className="space-y-2">
                <h4 className="text-sm font-semibold text-yellow-600 dark:text-yellow-400">
                  Warnings
                </h4>
                {result.warnings.map((warn, i) => (
                  <div
                    key={i}
                    className="flex items-start gap-2 text-sm bg-yellow-50 dark:bg-yellow-950/20 rounded-md p-2"
                  >
                    <AlertTriangle className="h-4 w-4 text-yellow-500 mt-0.5 shrink-0" />
                    <span>{warn}</span>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
}
