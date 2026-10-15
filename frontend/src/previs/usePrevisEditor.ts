import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { bridge } from "../bridge";
import { t } from "../i18n";
import { useModels } from "../models/useModels";
import { readyModel, selectedModel } from "../models/types";
import type { RuntimeHandle } from "./RuntimeFrame";
import { usePrevis } from "./context";
import type { PrevisExport, PrevisFile } from "./types";

export function usePrevisEditor(fileId: string, close: () => void) {
  const { workspace, project, refresh, save, flush, attachReference } =
    usePrevis();
  const file = workspace.files.find((f) => f.id === fileId)!;
  const [draft, setDraft] = useState(file);
  const [prompt, setPrompt] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState<"chat" | "render" | "save" | "share" | null>(
    null,
  );
  const [ready, setReady] = useState(false);
  const readyRef = useRef(false);
  const [playing, setPlaying] = useState(false);
  const [time, setTime] = useState(0);
  const [progress, setProgress] = useState("");
  const [showCode, setShowCode] = useState(false);
  const [modelId, setModelId] = useState("");
  const requestId = useRef("");
  const renderId = useRef("");
  const cancelled = useRef(false);
  const mounted = useRef(true);
  const runtime = useRef<RuntimeHandle>(null);
  const chatEnd = useRef<HTMLDivElement>(null);
  const { catalog } = useModels();
  const model =
    catalog.profiles.find((m) => m.id === modelId) || selectedModel(catalog);
  const models = workspace.files.filter((f) => f.kind === "model");
  const messages = workspace.messages.filter((m) => m.fileId === fileId);
  const exports = workspace.exports.filter((e) => e.fileId === fileId);
  const dirty = JSON.stringify(draft) !== JSON.stringify(file);
  useEffect(() => {
    setDraft(file);
  }, [file.revision]);
  useEffect(() => {
    chatEnd.current?.scrollIntoView({ block: "nearest" });
  }, [messages.length, busy]);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      cancelled.current = true;
      if (requestId.current)
        void bridge("cancel_previs_chat", { requestId: requestId.current });
      if (renderId.current)
        void bridge("cancel_previs_render", {
          projectId: project.id,
          id: renderId.current,
        }).catch(() => {});
    };
  }, [project.id]);
  async function persist() {
    return dirty ? await save(draft) : file;
  }
  async function chat(text: string): Promise<PrevisFile> {
    if (!model || !readyModel(model))
      throw Error(t("请先选择已连接的对话模型"));
    const saved = await persist();
    requestId.current = crypto.randomUUID();
    const result = await bridge<{ file: PrevisFile; message: string }>(
      "previs_chat",
      {
        request: {
          projectId: project.id,
          fileId,
          expectedRevision: saved.revision,
          prompt: text,
          modelId: model.id,
          requestId: requestId.current,
        },
      },
    );
    requestId.current = "";
    await refresh();
    if (mounted.current) {
      setDraft(result.file);
      setPrompt("");
    }
    return result.file;
  }
  async function send(text = prompt) {
    if (!text.trim() || busy) return;
    setBusy("chat");
    setError("");
    cancelled.current = false;
    try {
      await chat(text);
    } catch (e) {
      if (mounted.current) setError(String(e));
    } finally {
      requestId.current = "";
      if (mounted.current) setBusy(null);
    }
  }
  function onReady(value: boolean) {
    readyRef.current = value;
    setReady(value);
    if (!value) setPlaying(false);
  }
  async function waitForRuntime(revision: number) {
    const start = Date.now();
    while (Date.now() - start < 30000) {
      if (cancelled.current) throw Error(t("预演导出已停止"));
      if (readyRef.current && runtime.current?.revision() === revision) return;
      await new Promise((r) => setTimeout(r, 100));
    }
    throw Error(t("预演加载超时，请检查代码或重新加载"));
  }
  async function render() {
    if (busy) return;
    setBusy("render");
    setError("");
    cancelled.current = false;
    setPlaying(false);
    runtime.current?.play(false);
    try {
      let source = await persist();
      if (!source.code.trim()) {
        setProgress(t("正在创建镜头代码…"));
        source = await chat(
          `请根据当前镜头说明创建预演，优先复用项目模型，不足时新增有名字的模型资产。${source.description}`,
        );
      }
      setProgress(t("正在加载三维场景…"));
      await waitForRuntime(source.revision);
      const prepared = await bridge<{ id: string; frameCount: number }>(
        "begin_previs_render",
        { projectId: project.id, sessionId: runtime.current!.session() },
      );
      renderId.current = prepared.id;
      for (let i = 0; i < prepared.frameCount; i++) {
        if (cancelled.current) throw Error(t("预演导出已停止"));
        const bytes = await runtime.current!.frame(i / source.fps);
        await invoke("append_previs_frame", bytes, {
          headers: {
            "x-project-id": project.id,
            "x-render-id": prepared.id,
            "x-frame-index": String(i),
          },
        });
        if (mounted.current)
          setProgress(
            t("正在渲染 {done} / {total} 帧", {
              done: i + 1,
              total: prepared.frameCount,
            }),
          );
      }
      setProgress(t("正在编码预演视频…"));
      await bridge<PrevisExport>("finish_previs_render", {
        projectId: project.id,
        id: prepared.id,
      });
      renderId.current = "";
      await refresh();
      setProgress(t("预演视频已保存"));
    } catch (e) {
      if (renderId.current)
        await bridge("cancel_previs_render", {
          projectId: project.id,
          id: renderId.current,
        }).catch(() => {});
      if (mounted.current) setError(String(e));
    } finally {
      renderId.current = "";
      if (mounted.current) setBusy(null);
    }
  }
  async function stop() {
    cancelled.current = true;
    runtime.current?.play(false);
    setPlaying(false);
    if (requestId.current)
      await bridge("cancel_previs_chat", { requestId: requestId.current });
    if (renderId.current)
      await bridge("cancel_previs_render", {
        projectId: project.id,
        id: renderId.current,
      }).catch(() => {});
  }
  async function share(video: PrevisExport, shared: boolean) {
    setBusy("share");
    setError("");
    try {
      await flush();
      await bridge("share_previs_video", {
        projectId: project.id,
        id: video.id,
        shared,
      });
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  }
  function dependency(id: string, checked: boolean) {
    setDraft((d) => {
      const asset = models.find((m) => m.id === id)!;
      const ids = checked
        ? [...new Set([...d.assetIds, id])]
        : d.assetIds.filter((a) => a !== id);
      const bindings = { ...d.bindings };
      if (checked) bindings[asset.name] = id;
      for (const [alias, value] of Object.entries(bindings))
        if (!ids.includes(value)) delete bindings[alias];
      return { ...d, assetIds: ids, bindings };
    });
  }
  async function manualSave() {
    setBusy("save");
    setError("");
    try {
      setDraft(await save(draft));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  }
  const dismiss = () => {
    if (busy) return;
    if (dirty) {
      setError(t("请先保存当前修改，或恢复已保存版本后关闭。"));
      return;
    }
    close();
  };
  return {
    fileId,
    workspace,
    project,
    file,
    draft,
    setDraft,
    prompt,
    setPrompt,
    error,
    setError,
    busy,
    ready,
    playing,
    setPlaying,
    time,
    setTime,
    progress,
    showCode,
    setShowCode,
    model,
    setModelId,
    requestId,
    runtime,
    chatEnd,
    catalog,
    models,
    messages,
    exports,
    dirty,
    send,
    onReady,
    render,
    stop,
    share,
    dependency,
    manualSave,
    dismiss,
    attachReference,
  };
}
export type PrevisEditorState = ReturnType<typeof usePrevisEditor>;
