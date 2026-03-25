import { useState, useMemo } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { metadataApi, type WorkflowDef } from "@/api/conductor";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { JsonView } from "@/components/JsonView";
import { Search, Download, Tag, FileJson } from "lucide-react";
import { toast } from "sonner";

interface Template {
  id: string;
  name: string;
  description: string;
  category: string;
  tags: string[];
  definition: WorkflowDef;
}

const BUILTIN_TEMPLATES: Template[] = [
  {
    id: "http-poll",
    name: "HTTP Polling Workflow",
    description: "Periodically polls an HTTP endpoint and processes the response. Uses DO_WHILE loop with an HTTP task.",
    category: "Integration",
    tags: ["http", "polling", "loop"],
    definition: {
      name: "http_polling_workflow",
      version: 1,
      description: "Polls an HTTP endpoint periodically",
      tasks: [
        {
          name: "poll_loop",
          taskReferenceName: "poll_loop",
          type: "DO_WHILE",
          loopCondition: "if ($.poll_loop['iteration'] < $.maxIterations) { true; } else { false; }",
          loopOver: [
            {
              name: "http_poll",
              taskReferenceName: "http_poll_task",
              type: "HTTP",
              inputParameters: {
                http_request: {
                  uri: "${workflow.input.url}",
                  method: "GET",
                },
              },
            },
            {
              name: "wait_interval",
              taskReferenceName: "wait_task",
              type: "WAIT",
              inputParameters: { duration: "${workflow.input.intervalSeconds}s" },
            },
          ],
          inputParameters: { maxIterations: "${workflow.input.maxIterations}" },
        },
      ],
      inputParameters: ["url", "intervalSeconds", "maxIterations"],
    },
  },
  {
    id: "fan-out",
    name: "Fan-out / Fan-in",
    description: "Fork multiple parallel tasks and join their results. Classic scatter-gather pattern.",
    category: "Patterns",
    tags: ["fork", "join", "parallel"],
    definition: {
      name: "fan_out_fan_in",
      version: 1,
      description: "Parallel task execution with result aggregation",
      tasks: [
        {
          name: "fork_tasks",
          taskReferenceName: "fork_step",
          type: "FORK_JOIN",
          forkTasks: [
            [{ name: "branch_a", taskReferenceName: "branch_a", type: "SIMPLE", inputParameters: { data: "${workflow.input.data}" } }],
            [{ name: "branch_b", taskReferenceName: "branch_b", type: "SIMPLE", inputParameters: { data: "${workflow.input.data}" } }],
            [{ name: "branch_c", taskReferenceName: "branch_c", type: "SIMPLE", inputParameters: { data: "${workflow.input.data}" } }],
          ],
        },
        {
          name: "join_results",
          taskReferenceName: "join_step",
          type: "JOIN",
          joinOn: ["branch_a", "branch_b", "branch_c"],
        },
        {
          name: "aggregate",
          taskReferenceName: "aggregate_step",
          type: "SIMPLE",
          inputParameters: {
            resultA: "${branch_a.output}",
            resultB: "${branch_b.output}",
            resultC: "${branch_c.output}",
          },
        },
      ],
      inputParameters: ["data"],
    },
  },
  {
    id: "retry-with-backoff",
    name: "Retry with Exponential Backoff",
    description: "Retries a task with increasing delays on failure. Uses DO_WHILE with SWITCH for error handling.",
    category: "Resilience",
    tags: ["retry", "backoff", "error-handling"],
    definition: {
      name: "retry_backoff_workflow",
      version: 1,
      description: "Retries a task with exponential backoff",
      tasks: [
        {
          name: "retry_loop",
          taskReferenceName: "retry_loop",
          type: "DO_WHILE",
          loopCondition: "if ($.retry_loop['iteration'] < $.maxRetries) { true; } else { false; }",
          loopOver: [
            {
              name: "attempt_task",
              taskReferenceName: "attempt",
              type: "SIMPLE",
              optional: true,
              inputParameters: { payload: "${workflow.input.payload}" },
            },
            {
              name: "check_result",
              taskReferenceName: "check",
              type: "SWITCH",
              caseExpression: "$.attempt.status === 'COMPLETED' ? 'success' : 'retry'",
              decisionCases: {
                success: [
                  { name: "noop_success", taskReferenceName: "success_marker", type: "TERMINATE", inputParameters: { terminationStatus: "COMPLETED" } },
                ],
              },
              defaultCase: [
                { name: "backoff_wait", taskReferenceName: "backoff", type: "WAIT", inputParameters: { duration: "30s" } },
              ],
            },
          ],
          inputParameters: { maxRetries: "${workflow.input.maxRetries}" },
        },
      ],
      inputParameters: ["payload", "maxRetries"],
    },
  },
  {
    id: "etl-pipeline",
    name: "ETL Pipeline",
    description: "Extract-Transform-Load pipeline with staged processing and error notification.",
    category: "Data",
    tags: ["etl", "pipeline", "data"],
    definition: {
      name: "etl_pipeline",
      version: 1,
      description: "Standard ETL pipeline",
      tasks: [
        { name: "extract", taskReferenceName: "extract_step", type: "SIMPLE", inputParameters: { source: "${workflow.input.source}" } },
        { name: "transform", taskReferenceName: "transform_step", type: "SIMPLE", inputParameters: { rawData: "${extract_step.output.data}", rules: "${workflow.input.transformRules}" } },
        { name: "validate", taskReferenceName: "validate_step", type: "SIMPLE", inputParameters: { data: "${transform_step.output.data}" } },
        { name: "load", taskReferenceName: "load_step", type: "SIMPLE", inputParameters: { data: "${validate_step.output.data}", destination: "${workflow.input.destination}" } },
      ],
      inputParameters: ["source", "destination", "transformRules"],
      failureWorkflow: "etl_failure_handler",
    },
  },
  {
    id: "approval-workflow",
    name: "Human Approval Workflow",
    description: "Waits for human approval via WAIT task before proceeding. Includes timeout handling.",
    category: "Human-in-Loop",
    tags: ["approval", "human", "wait"],
    definition: {
      name: "approval_workflow",
      version: 1,
      description: "Human approval workflow with timeout",
      tasks: [
        { name: "prepare_request", taskReferenceName: "prepare", type: "SIMPLE", inputParameters: { request: "${workflow.input.request}" } },
        { name: "notify_approver", taskReferenceName: "notify", type: "EVENT", inputParameters: { sink: "conductor:approval_requests", approver: "${workflow.input.approver}" } },
        { name: "wait_for_approval", taskReferenceName: "approval_wait", type: "WAIT", inputParameters: { duration: "${workflow.input.timeoutMinutes}m" } },
        {
          name: "check_approval",
          taskReferenceName: "check_approval",
          type: "SWITCH",
          caseExpression: "$.approval_wait.output.approved === true ? 'approved' : 'rejected'",
          decisionCases: {
            approved: [{ name: "execute_action", taskReferenceName: "execute", type: "SIMPLE", inputParameters: { action: "${workflow.input.action}" } }],
            rejected: [{ name: "notify_rejection", taskReferenceName: "reject_notify", type: "EVENT", inputParameters: { sink: "conductor:rejections" } }],
          },
          defaultCase: [],
        },
      ],
      inputParameters: ["request", "approver", "action", "timeoutMinutes"],
    },
  },
  {
    id: "sub-workflow-chain",
    name: "Sub-workflow Chain",
    description: "Chains multiple sub-workflows sequentially, passing outputs as inputs to the next.",
    category: "Patterns",
    tags: ["sub-workflow", "chain", "composition"],
    definition: {
      name: "sub_workflow_chain",
      version: 1,
      description: "Sequential sub-workflow execution",
      tasks: [
        {
          name: "step_1",
          taskReferenceName: "sub_wf_1",
          type: "SUB_WORKFLOW",
          subWorkflowParam: { name: "${workflow.input.workflow1Name}", version: 1 },
          inputParameters: { data: "${workflow.input.initialData}" },
        },
        {
          name: "step_2",
          taskReferenceName: "sub_wf_2",
          type: "SUB_WORKFLOW",
          subWorkflowParam: { name: "${workflow.input.workflow2Name}", version: 1 },
          inputParameters: { data: "${sub_wf_1.output}" },
        },
        {
          name: "step_3",
          taskReferenceName: "sub_wf_3",
          type: "SUB_WORKFLOW",
          subWorkflowParam: { name: "${workflow.input.workflow3Name}", version: 1 },
          inputParameters: { data: "${sub_wf_2.output}" },
        },
      ],
      inputParameters: ["workflow1Name", "workflow2Name", "workflow3Name", "initialData"],
    },
  },
];

/**
 * #179 — Workflow template marketplace.
 * Browse, preview, and import built-in workflow templates.
 */
export default function TemplateMarketplace() {
  const [search, setSearch] = useState("");
  const [categoryFilter, setCategoryFilter] = useState<string>("all");
  const [previewTemplate, setPreviewTemplate] = useState<Template | null>(null);
  const queryClient = useQueryClient();

  const categories = useMemo(() => {
    const set = new Set(BUILTIN_TEMPLATES.map((t) => t.category));
    return Array.from(set).sort();
  }, []);

  const filtered = useMemo(() => {
    let result = BUILTIN_TEMPLATES;
    if (categoryFilter !== "all") {
      result = result.filter((t) => t.category === categoryFilter);
    }
    if (search) {
      const lower = search.toLowerCase();
      result = result.filter(
        (t) =>
          t.name.toLowerCase().includes(lower) ||
          t.description.toLowerCase().includes(lower) ||
          t.tags.some((tag) => tag.toLowerCase().includes(lower))
      );
    }
    return result;
  }, [search, categoryFilter]);

  const importMut = useMutation({
    mutationFn: (def: WorkflowDef) => metadataApi.registerWorkflowDef(def),
    onSuccess: () => {
      toast.success("Template imported successfully");
      queryClient.invalidateQueries({ queryKey: ["workflowDefs"] });
    },
    onError: (err: Error) => toast.error(`Import failed: ${err.message}`),
  });

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold tracking-tight">Workflow Templates</h2>
        <Badge variant="secondary">{BUILTIN_TEMPLATES.length} templates</Badge>
      </div>

      {/* Filters */}
      <div className="flex gap-2">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search templates..."
            className="pl-9"
          />
        </div>
        <div className="flex gap-1">
          <Button
            variant={categoryFilter === "all" ? "default" : "outline"}
            size="sm"
            onClick={() => setCategoryFilter("all")}
          >
            All
          </Button>
          {categories.map((cat) => (
            <Button
              key={cat}
              variant={categoryFilter === cat ? "default" : "outline"}
              size="sm"
              onClick={() => setCategoryFilter(cat)}
            >
              {cat}
            </Button>
          ))}
        </div>
      </div>

      {/* Template grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {filtered.map((tmpl) => (
          <Card key={tmpl.id} className="hover:shadow-md transition-shadow">
            <CardContent className="pt-4 space-y-3">
              <div className="flex items-start justify-between">
                <div>
                  <h3 className="font-semibold text-sm">{tmpl.name}</h3>
                  <Badge variant="secondary" className="text-[10px] mt-1">{tmpl.category}</Badge>
                </div>
                <FileJson className="h-5 w-5 text-muted-foreground" />
              </div>
              <p className="text-xs text-muted-foreground">{tmpl.description}</p>
              <div className="flex flex-wrap gap-1">
                {tmpl.tags.map((tag) => (
                  <span key={tag} className="flex items-center gap-0.5 text-[10px] text-muted-foreground">
                    <Tag className="h-2.5 w-2.5" />{tag}
                  </span>
                ))}
              </div>
              <div className="text-xs text-muted-foreground">
                {tmpl.definition.tasks.length} task{tmpl.definition.tasks.length !== 1 ? "s" : ""}
                {tmpl.definition.inputParameters && ` · ${tmpl.definition.inputParameters.length} inputs`}
              </div>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  className="flex-1"
                  onClick={() => setPreviewTemplate(previewTemplate?.id === tmpl.id ? null : tmpl)}
                >
                  Preview
                </Button>
                <Button
                  size="sm"
                  className="flex-1"
                  onClick={() => importMut.mutate(tmpl.definition)}
                  disabled={importMut.isPending}
                >
                  <Download className="h-3 w-3 mr-1" />
                  Import
                </Button>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      {filtered.length === 0 && (
        <p className="text-center text-muted-foreground py-8">No templates match your search.</p>
      )}

      {/* Preview panel */}
      {previewTemplate && (
        <Card>
          <CardContent className="pt-4">
            <div className="flex items-center justify-between mb-3">
              <h3 className="font-semibold">Preview: {previewTemplate.name}</h3>
              <Button variant="ghost" size="sm" onClick={() => setPreviewTemplate(null)}>Close</Button>
            </div>
            <JsonView data={previewTemplate.definition} maxHeight="24rem" />
          </CardContent>
        </Card>
      )}
    </div>
  );
}
