import { useCallback, useEffect, useRef, useState } from "react";
import type { AnalysisController } from "./controller";
import type { RecordSummary } from "./types";

export function useAnalysisHistory(
  controller: AnalysisController,
  recordId?: string,
) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [items, setItems] = useState<RecordSummary[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const sequence = useRef(0);
  const load = useCallback(
    async (offset = 0) => {
      const request = ++sequence.current;
      setLoading(true);
      setError("");
      try {
        const page = await controller.listRecords(query, offset);
        if (request !== sequence.current) return;
        setItems((previous) =>
          offset ? [...previous, ...page.items] : page.items,
        );
        setTotal(page.total);
      } catch (error) {
        if (request === sequence.current) setError(String(error));
      } finally {
        if (request === sequence.current) setLoading(false);
      }
    },
    [controller, query],
  );
  useEffect(() => {
    const timer = setTimeout(
      () => {
        void load();
      },
      query ? 180 : 0,
    );
    return () => {
      clearTimeout(timer);
      sequence.current++;
    };
  }, [load, open, recordId]);
  return {
    open,
    setOpen,
    query,
    setQuery,
    items,
    total,
    loading,
    error,
    reload: () => load(),
    more: () => load(items.length),
  };
}
export type AnalysisHistoryState = ReturnType<typeof useAnalysisHistory>;
