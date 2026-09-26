import { useCallback, useEffect, useRef, useState } from "react";
import { bridge, native } from "../bridge";
import { emptyCatalog, type ModelCatalog } from "./types";
export const modelsChanged = () =>
  window.dispatchEvent(new Event("agent-config-changed"));
export function useModels(projectId?: string) {
  const [catalog, setCatalog] = useState<ModelCatalog>(emptyCatalog);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const revision = useRef(0);
  const refresh = useCallback(async () => {
    const sequence = ++revision.current;
    setLoading(true);
    try {
      const next = native
        ? await bridge<ModelCatalog>("model_catalog", {
            projectId: projectId || null,
          })
        : emptyCatalog;
      if (sequence === revision.current) {
        setCatalog(next);
        setError("");
      }
    } catch (e) {
      if (sequence === revision.current) setError(String(e));
    } finally {
      if (sequence === revision.current) setLoading(false);
    }
  }, [projectId]);
  useEffect(() => {
    const update = () => {
      void refresh();
    };
    update();
    window.addEventListener("agent-config-changed", update);
    return () => {
      ++revision.current;
      window.removeEventListener("agent-config-changed", update);
    };
  }, [refresh]);
  return { catalog, loading, error, refresh };
}
