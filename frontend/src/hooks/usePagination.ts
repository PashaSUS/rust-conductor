import { useState, useMemo } from "react";

const DEFAULT_PAGE_SIZE = 25;

export interface PaginationState {
  page: number;
  pageSize: number;
  totalItems: number;
  totalPages: number;
  setPage: (page: number) => void;
  canPrev: boolean;
  canNext: boolean;
  prev: () => void;
  next: () => void;
  startIndex: number;
  endIndex: number;
}

/**
 * Client-side pagination hook. Paginates a pre-loaded array.
 * Returns the current page slice and pagination controls.
 */
export function usePagination<T>(
  items: T[],
  pageSize = DEFAULT_PAGE_SIZE,
): [T[], PaginationState] {
  const [page, setPage] = useState(0);

  const totalItems = items.length;
  const totalPages = Math.max(1, Math.ceil(totalItems / pageSize));

  // Clamp page to valid range when items change
  const safePage = Math.min(page, totalPages - 1);
  if (safePage !== page) setPage(safePage);

  const slice = useMemo(
    () => items.slice(safePage * pageSize, (safePage + 1) * pageSize),
    [items, safePage, pageSize],
  );

  const state: PaginationState = {
    page: safePage,
    pageSize,
    totalItems,
    totalPages,
    setPage,
    canPrev: safePage > 0,
    canNext: safePage < totalPages - 1,
    prev: () => setPage(Math.max(0, safePage - 1)),
    next: () => setPage(Math.min(totalPages - 1, safePage + 1)),
    startIndex: safePage * pageSize,
    endIndex: Math.min((safePage + 1) * pageSize, totalItems),
  };

  return [slice, state];
}
