import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { bridge, native } from "../bridge";
import type { Project } from "../model";
import {
  emptyWorkspace,
  newModel,
  newShot,
  type PrevisFile,
  type PrevisWorkspace,
} from "./types";
import { PrevisEditor } from "./PrevisEditor";
import "./previs.css";
import { t } from "../i18n";
import { Context } from "./context";
export function PrevisProvider({
  project,
  flush,
  change,
  children,
}: {
  project: Project;
  flush: () => Promise<unknown>;
  change: (fn: (p: Project) => Project) => void;
  children: ReactNode;
}) {
  const [workspace, setWorkspace] = useState<PrevisWorkspace>(emptyWorkspace);
  const [editing, setEditing] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const sequence = useRef(0);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      ++sequence.current;
    };
  }, []);
  const refresh = useCallback(async () => {
    if (!native) return;
    const seq = ++sequence.current;
    setLoading(true);
    try {
      const next = await bridge<PrevisWorkspace>("previs_workspace", {
        projectId: project.id,
      });
      if (mounted.current && seq === sequence.current) {
        setWorkspace(next);
        setError("");
      }
    } catch (e) {
      if (mounted.current && seq === sequence.current) setError(String(e));
    } finally {
      if (mounted.current && seq === sequence.current) setLoading(false);
    }
  }, [project.id]);
  useEffect(() => {
    setWorkspace(emptyWorkspace);
    setEditing(null);
    void refresh();
  }, [refresh]);
  const save = useCallback(
    async (file: PrevisFile) => {
      await flush();
      const saved = await bridge<PrevisFile>("save_previs_file", {
        projectId: project.id,
        file,
        expectedRevision: file.revision,
      });
      await refresh();
      return saved;
    },
    [project.id, flush, refresh],
  );
  const open = useCallback(
    async (id: string) => {
      try {
        await flush();
        const ws = await bridge<PrevisWorkspace>("previs_workspace", {
          projectId: project.id,
        });
        if (!ws.files.some((f) => f.id === id)) {
          const node = project.nodes.find((n) => n.id === id && n.shot);
          if (!node) throw Error("镜头已删除");
          await save(newShot(node, project));
        }
        await refresh();
        if (mounted.current) setEditing(id);
      } catch (e) {
        if (mounted.current) setError(String(e));
      }
    },
    [project, flush, refresh, save],
  );
  useEffect(() => {
    const handler = (e: Event) => void open((e as CustomEvent<string>).detail);
    window.addEventListener("mstudio-open-previs", handler);
    return () => window.removeEventListener("mstudio-open-previs", handler);
  }, [open]);
  const createModel = async () => {
    try {
      const names = new Set(
        workspace.files.filter((f) => f.kind === "model").map((f) => f.name),
      );
      let i = 1;
      while (names.has(`模型 ${i}`)) i++;
      const file = await save(newModel(`模型 ${i}`));
      setEditing(file.id);
    } catch (e) {
      setError(String(e));
    }
  };
  const attachReference = (fileId: string, assetId: string) =>
    change((p) => ({
      ...p,
      nodes: p.nodes.map((n) =>
        n.id === fileId && n.shot
          ? {
              ...n,
              references: [
                ...(n.references ?? []).filter((r) => r.assetId !== assetId),
                {
                  assetId,
                  purpose: "预演动作、构图与运镜参考；商品外观以实物参考为准",
                  start: 0,
                  end: p.assets.find((a) => a.id === assetId)?.duration,
                },
              ],
            }
          : n,
      ),
    }));
  return (
    <Context.Provider
      value={{
        workspace,
        project,
        refresh,
        save,
        flush,
        attachReference,
        open: (id) => void open(id),
        createModel: () => void createModel(),
        error,
        loading,
      }}
    >
      {children}
      {error && (
        <div className="previs-workspace-error" role="alert">
          <span>{error}</span>
          <button type="button" onClick={() => setError("")}>
            {t("关闭")}
          </button>
        </div>
      )}
      {editing && workspace.files.some((f) => f.id === editing) && (
        <PrevisEditor
          key={editing}
          fileId={editing}
          close={() => setEditing(null)}
        />
      )}
    </Context.Provider>
  );
}
