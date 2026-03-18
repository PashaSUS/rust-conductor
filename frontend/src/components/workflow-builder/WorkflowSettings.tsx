import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { TagInput } from "@/components/TagInput";
import { Settings2 } from "lucide-react";

interface WorkflowSettingsProps {
  name: string;
  version: number;
  description: string;
  timeoutSeconds: number;
  ownerEmail: string;
  failureWorkflow: string;
  inputParams: string[];
  onUpdate: (field: string, value: string | number) => void;
  onUpdateInputParams: (params: string[]) => void;
}

export function WorkflowSettings({
  name,
  version,
  description,
  timeoutSeconds,
  ownerEmail,
  failureWorkflow,
  inputParams,
  onUpdate,
  onUpdateInputParams,
}: WorkflowSettingsProps) {
  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-sm font-medium flex items-center gap-2">
          <Settings2 className="h-4 w-4" />
          Workflow Settings
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label htmlFor="wf-name">Name *</Label>
            <Input
              id="wf-name"
              placeholder="my_workflow"
              value={name}
              onChange={(e) => onUpdate("name", e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="wf-version">Version</Label>
            <Input
              id="wf-version"
              type="number"
              min={1}
              value={version}
              onChange={(e) => onUpdate("version", Number(e.target.value) || 1)}
            />
          </div>
        </div>
        <div className="space-y-2">
          <Label htmlFor="wf-desc">Description</Label>
          <Input
            id="wf-desc"
            placeholder="What does this workflow do?"
            value={description}
            onChange={(e) => onUpdate("description", e.target.value)}
          />
        </div>
        <div className="grid grid-cols-3 gap-4">
          <div className="space-y-2">
            <Label htmlFor="wf-timeout">Timeout (seconds)</Label>
            <Input
              id="wf-timeout"
              type="number"
              min={0}
              value={timeoutSeconds}
              onChange={(e) => onUpdate("timeoutSeconds", Number(e.target.value) || 0)}
              placeholder="0 = no timeout"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="wf-owner">Owner Email</Label>
            <Input
              id="wf-owner"
              type="email"
              value={ownerEmail}
              onChange={(e) => onUpdate("ownerEmail", e.target.value)}
              placeholder="team@example.com"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="wf-failure">Failure Workflow</Label>
            <Input
              id="wf-failure"
              value={failureWorkflow}
              onChange={(e) => onUpdate("failureWorkflow", e.target.value)}
              placeholder="Optional"
            />
          </div>
        </div>
        <div className="space-y-2">
          <Label htmlFor="wf-inputs">Input Parameters</Label>
          <TagInput
            id="wf-inputs"
            tags={inputParams}
            onChange={onUpdateInputParams}
            placeholder="Type a parameter name and press Enter"
          />
          <p className="text-[10px] text-muted-foreground">
            These become available as sources for task input mapping.
          </p>
        </div>
      </CardContent>
    </Card>
  );
}
