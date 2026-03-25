import { Button } from "@/components/ui/button";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { useThemeText } from "@/components/ThemeContext";

interface PaginationControlsProps {
  page: number;
  totalPages: number;
  canPrev: boolean;
  canNext: boolean;
  onPrev: () => void;
  onNext: () => void;
  /** e.g. "1–25" */
  rangeLabel?: string;
  totalItems?: number;
}

export function PaginationControls({
  page,
  totalPages,
  canPrev,
  canNext,
  onPrev,
  onNext,
  rangeLabel,
  totalItems,
}: PaginationControlsProps) {
  const t = useThemeText();

  if (totalPages <= 1) return null;

  return (
    <div className="flex items-center justify-between">
      <p className="text-xs text-muted-foreground">
        {rangeLabel && totalItems !== undefined
          ? `${t.showing} ${rangeLabel} ${t.of} ${totalItems}`
          : `${t.page} ${page + 1} ${t.of} ${totalPages}`}
      </p>
      <div className="flex gap-1">
        <Button variant="outline" size="sm" disabled={!canPrev} onClick={onPrev}>
          <ChevronLeft className="h-4 w-4 mr-1" />
          {t.prev}
        </Button>
        <Button variant="outline" size="sm" disabled={!canNext} onClick={onNext}>
          {t.next}
          <ChevronRight className="h-4 w-4 ml-1" />
        </Button>
      </div>
    </div>
  );
}
