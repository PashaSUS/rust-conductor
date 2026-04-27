import { useMemo, useCallback } from "react";
import { useSearchParams } from "react-router";
import type { SearchParams } from "@/api/conductor";

const PAGE_SIZE = 25;

export { PAGE_SIZE };

export function useFilterParams() {
  const [searchParams, setSearchParams] = useSearchParams();

  const params: SearchParams = useMemo(() => ({
    status: searchParams.get("status") || undefined,
    workflowType: searchParams.get("workflowType") || undefined,
    freeText: searchParams.get("freeText") || undefined,
    start: searchParams.has("start") ? Number(searchParams.get("start")) : 0,
    size: PAGE_SIZE,
    tags: searchParams.get("tags") || undefined,
  }), [searchParams]);

  const setParams = useCallback((updater: (prev: SearchParams) => SearchParams) => {
    setSearchParams((prev) => {
      const current: SearchParams = {
        status: prev.get("status") || undefined,
        workflowType: prev.get("workflowType") || undefined,
        freeText: prev.get("freeText") || undefined,
        start: prev.has("start") ? Number(prev.get("start")) : 0,
        size: PAGE_SIZE,
        tags: prev.get("tags") || undefined,
      };
      const next = updater(current);
      const qs = new URLSearchParams();
      if (next.status) qs.set("status", next.status);
      if (next.workflowType) qs.set("workflowType", next.workflowType);
      if (next.freeText) qs.set("freeText", next.freeText);
      if (next.start) qs.set("start", String(next.start));
      if (next.tags) qs.set("tags", next.tags);
      return qs;
    });
  }, [setSearchParams]);

  return { params, setParams };
}
