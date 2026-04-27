import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Settings2, Plus, AlertTriangle } from "lucide-react";

interface WorkflowSettingsPanelProps {
  workflowName: string;
  setWorkflowName: (v: string) => void;
  workflowVersion: number;
  setWorkflowVersion: (v: number) => void;
  workflowDesc: string;
  setWorkflowDesc: (v: string) => void;
  workflowInputKeys: string[];
  inputKeyDraft: string;
  setInputKeyDraft: (v: string) => void;
  addInputKey: () => void;
  removeInputKey: (k: string) => void;
  workflowOutputParams: Record<string, string>;
  validationErrors: string[];
  onClose: () => void;
}

export function WorkflowSettingsPanel({
  workflowName, setWorkflowName, workflowVersion, setWorkflowVersion,
  workflowDesc, setWorkflowDesc, workflowInputKeys, inputKeyDraft,
  setInputKeyDraft, addInputKey, removeInputKey, workflowOutputParams,
  validationErrors, onClose,
}: WorkflowSettingsPanelProps) {
  return (
    <div className="p-4 space-y-3">
      <div className="flex items-center justify-between">
        <h4 className="font-semibold text-sm flex items-center gap-1.5"><Settings2 className="h-4 w-4" /> Workflow Settings</h4>
        <Button variant="ghost" size="sm" className="h-6 px-1" onClick={onClose}>×</Button>
      </div>
      <div><label className="text-xs text-muted-foreground mb-1 block">Name</label><Input value={workflowName} onChange={(e) => setWorkflowName(e.target.value)} className="h-8 text-xs" /></div>
      <div className="grid grid-cols-2 gap-2">
        <div><label className="text-xs text-muted-foreground mb-1 block">Version</label><Input type="number" min={1} value={workflowVersion} onChange={(e) => setWorkflowVersion(Number(e.target.value))} className="h-8 text-xs" /></div>
        <div className="col-span-1" />
      </div>
      <div><label className="text-xs text-muted-foreground mb-1 block">Description</label><Textarea value={workflowDesc} onChange={(e) => setWorkflowDesc(e.target.value)} placeholder="Describe what this workflow does..." className="text-xs min-h-16 resize-y" /></div>
      <div className="border-t pt-2">
        <label className="text-xs font-medium mb-1.5 block">Input Parameters</label>
        <div className="flex gap-1 mb-2">
          <Input value={inputKeyDraft} onChange={(e) => setInputKeyDraft(e.target.value)} onKeyDown={(e) => e.key === "Enter" && addInputKey()} placeholder="Add input key..." className="h-7 text-xs flex-1" />
          <Button variant="outline" size="sm" className="h-7 px-2" onClick={addInputKey}><Plus className="h-3 w-3" /></Button>
        </div>
        <div className="flex flex-wrap gap-1">
          {workflowInputKeys.map((k) => (
            <Badge key={k} variant="secondary" className="text-[10px] gap-0.5 h-5">{k}<button onClick={() => removeInputKey(k)} className="hover:text-destructive ml-1">×</button></Badge>
          ))}
          {workflowInputKeys.length === 0 && <p className="text-[10px] text-muted-foreground">No input parameters defined</p>}
        </div>
      </div>
      <div className="border-t pt-2">
        <label className="text-xs font-medium mb-1.5 block">Output Parameters</label>
        <div className="bg-muted/50 rounded p-2">
          <p className="text-[10px] text-muted-foreground">
            {Object.keys(workflowOutputParams).length > 0 ? Object.keys(workflowOutputParams).join(", ") : "Connect task outputs to END node to define workflow outputs"}
          </p>
        </div>
      </div>
      {validationErrors.length > 0 && (
        <div className="bg-destructive/10 rounded p-2 space-y-1 border-t pt-2">
          <p className="text-xs font-medium text-destructive mb-1">Validation Errors</p>
          {validationErrors.map((e, i) => (
            <p key={i} className="text-[10px] text-destructive flex items-center gap-1"><AlertTriangle className="h-2.5 w-2.5 shrink-0" /> {e}</p>
          ))}
        </div>
      )}
    </div>
  );
}
