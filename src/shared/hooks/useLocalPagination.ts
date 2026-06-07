import { useEffect, useMemo, useState } from 'react';

export const LOCAL_TABLE_PAGE_SIZE = 10;
export const LOCAL_TABLE_PAGE_SIZE_OPTIONS = [10, 25, 50];

export function useLocalPagination<T>(rows: T[], pageSize = LOCAL_TABLE_PAGE_SIZE) {
  const [page, setPage] = useState(0);
  const [currentPageSize, setCurrentPageSize] = useState(pageSize);
  const totalRows = rows.length;
  const totalPages = Math.max(1, Math.ceil(totalRows / currentPageSize));

  useEffect(() => {
    setPage((current) => Math.min(current, totalPages - 1));
  }, [totalPages]);

  const paginatedRows = useMemo(() => {
    const start = page * currentPageSize;
    return rows.slice(start, start + currentPageSize);
  }, [currentPageSize, page, rows]);
  const startIndex = page * currentPageSize;
  const fromRow = totalRows === 0 ? 0 : startIndex + 1;
  const toRow = Math.min(startIndex + currentPageSize, totalRows);

  const setPageSize = (nextPageSize: number) => {
    setCurrentPageSize(nextPageSize);
    setPage(0);
  };

  return {
    page,
    pageSize: currentPageSize,
    startIndex,
    totalRows,
    fromRow,
    toRow,
    totalPages,
    paginatedRows,
    canPreviousPage: page > 0,
    canNextPage: page < totalPages - 1,
    previousPage: () => setPage((current) => Math.max(0, current - 1)),
    nextPage: () => setPage((current) => Math.min(totalPages - 1, current + 1)),
    resetPage: () => setPage(0),
    setPageSize,
  };
}
