import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { TagInput } from "@/components/TagInput";
import { Settings2 } from "lucide-react";
import { useThemeText } from "@/components/ThemeContext";

interface WorkflowSettingsProps {
  name: string;
  version: number;
  description: string;
  timeoutSeconds: number;
  ownerEmail: string;
  failureWorkflow: string;
  inputParams: string[];
  onCompleteWebhook: string;
  onFailureWebhook: string;
  slaDeadlineSeconds: number;
  tags: string[];
  onUpdate: (field: string, value: string | number) => void;
  onUpdateInputParams: (params: string[]) => void;
  onUpdateTags: (tags: string[]) => void;
}

export function WorkflowSettings({
  name,
  version,
  description,
  timeoutSeconds,
  ownerEmail,
  failureWorkflow,
  inputParams,
  onCompleteWebhook,
  onFailureWebhook,
  slaDeadlineSeconds,
  tags,
  onUpdate,
  onUpdateInputParams,
  onUpdateTags,
}: WorkflowSettingsProps) {
  const t = useThemeText();
  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-sm font-medium flex items-center gap-2">
          <Settings2 className="h-4 w-4" />
          {t.workflowSettings}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label htmlFor="wf-name">{t.nameRequired}</Label>
            <Input
              id="wf-name"
              placeholder="my_workflow"
              value={name}
              onChange={(e) => onUpdate("name", e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="wf-version">{t.versionLabel}</Label>
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
          <Label htmlFor="wf-desc">{t.description}</Label>
          <Input
            id="wf-desc"
            placeholder={t.descriptionPlaceholder}
            value={description}
            onChange={(e) => onUpdate("description", e.target.value)}
          />
        </div>
        <div className="grid grid-cols-3 gap-4">
          <div className="space-y-2">
            <Label htmlFor="wf-timeout">{t.timeoutSeconds}</Label>
            <Input
              id="wf-timeout"
              type="number"
              min={0}
              value={timeoutSeconds}
              onChange={(e) => onUpdate("timeoutSeconds", Number(e.target.value) || 0)}
              placeholder={t.noTimeoutHint}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="wf-owner">{t.ownerEmail}</Label>
            <Input
              id="wf-owner"
              type="email"
              value={ownerEmail}
              onChange={(e) => onUpdate("ownerEmail", e.target.value)}
              placeholder="team@example.com"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="wf-failure">{t.failureWorkflow}</Label>
            <Input
              id="wf-failure"
              value={failureWorkflow}
              onChange={(e) => onUpdate("failureWorkflow", e.target.value)}
              placeholder={t.optional}
            />
          </div>
        </div>
        <div className="space-y-2">
          <Label htmlFor="wf-inputs">{t.inputParameters}</Label>
          <TagInput
            id="wf-inputs"
            tags={inputParams}
            onChange={onUpdateInputParams}
            placeholder={t.inputParamPlaceholder}
          />
          <p className="text-[10px] text-muted-foreground">
            {t.inputParamHint}
          </p>
        </div>
        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label htmlFor="wf-complete-webhook">{t.onCompleteWebhook}</Label>
            <Input
              id="wf-complete-webhook"
              value={onCompleteWebhook}
              onChange={(e) => onUpdate("onCompleteWebhook", e.target.value)}
              placeholder="https://example.com/hook"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="wf-failure-webhook">{t.onFailureWebhook}</Label>
            <Input
              id="wf-failure-webhook"
              value={onFailureWebhook}
              onChange={(e) => onUpdate("onFailureWebhook", e.target.value)}
              placeholder="https://example.com/hook"
            />
          </div>
        </div>
        <div className="space-y-2">
          <Label htmlFor="wf-sla">{t.slaDeadline}</Label>
          <Input
            id="wf-sla"
            type="number"
            min={0}
            value={slaDeadlineSeconds}
            onChange={(e) => onUpdate("slaDeadlineSeconds", Number(e.target.value) || 0)}
            placeholder={t.optional}
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="wf-tags">{t.tagsLabel}</Label>
          <TagInput
            id="wf-tags"
            tags={tags}
            onChange={onUpdateTags}
            placeholder={t.tagsPlaceholder}
          />
        </div>
      </CardContent>
    </Card>
  );
}
