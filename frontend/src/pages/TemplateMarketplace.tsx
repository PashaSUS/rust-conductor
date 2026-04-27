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
import { BUILTIN_TEMPLATES, type Template } from "./builtin-templates";

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
