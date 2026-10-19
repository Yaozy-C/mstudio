import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { bridge } from "../bridge";
import { t } from "../i18n";
import { usePrevisAgent } from "./usePrevisAgent";
import { readyModel, type ModelConnection } from "../models/types";
import type { RuntimeHandle } from "./RuntimeFrame";
import type { PrevisFile, PrevisImage, PrevisWorkspace } from "./types";

export type PrevisPhase =
  | "saving"
  | "generating"
  | "loading"
  | "rendering"
  | "encoding"
  | "complete"
  | "cancelled"
  | null;

/** Owned by the project session, never by the dismissible dialog. */
export function usePrevisTask({
  projectId,
  fileId,
  model,
  persist,
  refresh,
  onResult,
}: {
  projectId: string;
  fileId: string;
  model: ModelConnection | undefined;
  persist: () => Promise<PrevisFile>;
  refresh: () => Promise<void>;
  onResult: (file: PrevisFile) => void;
}) {
  const agent = usePrevisAgent();
  const [busy, setBusy] = useState<"chat" | "render" | "save" | "share" | null>(
    null,
  );
  const [lastKind, setLastKind] = useState<"chat" | "render">("chat");
  const [phase, setPhase] = useState<PrevisPhase>(null);
  const [error, setError] = useState("");
  const [pendingPrompt, setPendingPrompt] = useState("");
  const [pendingImages, setPendingImages] = useState<PrevisImage[]>([]);
  const [pendingRequestId, setPendingRequestId] = useState("");
  const accepted = useRef(false);
  const [stopping, setStopping] = useState(false);
  const [frames, setFrames] = useState({ done: 0, total: 0 });
  const [renderFiles, setRenderFiles] = useState<PrevisFile[]>([]);
  const [renderSource, setRenderSource] = useState<PrevisFile | null>(null);
  const [seconds, setSeconds] = useState(0);
  const requestId = useRef("");
  const renderId = useRef("");
  const cancelled = useRef(false);
  const running = useRef(false);
  const renderRuntime = useRef<RuntimeHandle>(null);
  const renderReady = useRef(false);
  const renderError = useRef("");
  useEffect(() => {
    if (busy !== "chat" && busy !== "render") return;
    const start = Date.now();
    const timer = setInterval(
      () => setSeconds(Math.floor((Date.now() - start) / 1000)),
      1000,
    );
    return () => clearInterval(timer);
  }, [busy]);
  useEffect(
    () => () => {
      // Leaving the project must not cancel a submitted model request. It saves on the host.
      // Frame capture still needs this webview; release an unfinished export on project teardown.
      cancelled.current = true;
      if (renderId.current)
        void bridge("cancel_previs_render", {
          projectId,
          id: renderId.current,
        }).catch(() => {});
    },
    [projectId],
  );
  function checkCancelled() {
    if (cancelled.current) throw Error(t("预演任务已停止"));
  }
  function start(kind: "chat" | "render", text: string) {
    if (running.current || busy) return false;
    running.current = true;
    cancelled.current = false;
    setBusy(kind);
    setLastKind(kind);
    setError("");
    setPhase("saving");
    setStopping(false);
    setPendingPrompt(text);
    setPendingImages([]);
    setPendingRequestId("");
    accepted.current = false;
    setFrames({ done: 0, total: 0 });
    setSeconds(0);
    return true;
  }
  function fail(e: unknown) {
    if (cancelled.current) setPhase("cancelled");
    else setError(String(e));
  }
  function finish() {
    running.current = false;
    requestId.current = "";
    renderId.current = "";
    setPendingPrompt("");
    setPendingImages([]);
    setPendingRequestId("");
    setBusy(null);
    setStopping(false);
    setRenderSource(null);
  }
  async function chat(
    text: string,
    saved: PrevisFile,
    images: PrevisImage[] = [],
  ) {
    checkCancelled();
    if (!model || !readyModel(model))
      throw Error(t("请先选择已连接的对话模型"));
    setPhase("generating");
    requestId.current = crypto.randomUUID();
    setPendingRequestId(requestId.current);
    try {
      const result = await agent.run(
        {
          projectId,
          fileId,
          expectedRevision: saved.revision,
          prompt: text,
          imageAssetIds: images.map((a) => a.id),
          modelId: model.id,
          requestId: requestId.current,
        },
        () => {
          accepted.current = true;
        },
      );
      accepted.current = true;
      onResult(result.file);
      return result.file;
    } finally {
      requestId.current = "";
      await refresh();
    }
  }
  async function send(text: string, images: PrevisImage[] = []) {
    if (!text.trim() || !start("chat", text)) return false;
    setPendingImages(images);
    try {
      await chat(text, await persist(), images);
      setPhase("complete");
      return true;
    } catch (e) {
      fail(e);
      // A generation error does not unsend an already accepted user message.
      return accepted.current;
    } finally {
      finish();
    }
  }
  async function waitForRuntime(revision: number) {
    const start = Date.now();
    while (Date.now() - start < 30000) {
      checkCancelled();
      if (renderError.current) throw Error(renderError.current);
      if (renderReady.current && renderRuntime.current?.revision() === revision)
        return;
      await new Promise((r) => setTimeout(r, 100));
    }
    throw Error(t("预演加载超时，请检查代码或重新加载"));
  }
  async function render(images: PrevisImage[] = []) {
    if (!start("render", t("根据镜头说明生成预演视频"))) return false;
    try {
      let source = await persist();
      checkCancelled();
      if (!source.code.trim()) {
        setPendingImages(images);
        source = await chat(
          `请根据当前镜头说明创建预演，优先复用项目模型，不足时新增有名字的模型资产。${source.description}`,
          source,
          images,
        );
      }
      checkCancelled();
      const snapshot = await bridge<PrevisWorkspace>("previs_workspace", {
        projectId,
      });
      checkCancelled();
      setRenderFiles(snapshot.files);
      renderReady.current = false;
      renderError.current = "";
      setRenderSource(source);
      setPhase("loading");
      await waitForRuntime(source.revision);
      const prepared = await bridge<{ id: string; frameCount: number }>(
        "begin_previs_render",
        {
          projectId,
          sessionId: renderRuntime.current!.session(),
        },
      );
      renderId.current = prepared.id;
      setPhase("rendering");
      setFrames({ done: 0, total: prepared.frameCount });
      for (let i = 0; i < prepared.frameCount; i++) {
        checkCancelled();
        const bytes = await renderRuntime.current!.frame(i / source.fps);
        checkCancelled();
        await invoke("append_previs_frame", bytes, {
          headers: {
            "x-project-id": projectId,
            "x-render-id": prepared.id,
            "x-frame-index": String(i),
          },
        });
        setFrames({ done: i + 1, total: prepared.frameCount });
      }
      checkCancelled();
      setPhase("encoding");
      await bridge("finish_previs_render", { projectId, id: prepared.id });
      renderId.current = "";
      await refresh();
      setPhase("complete");
      return true;
    } catch (e) {
      if (renderId.current)
        await bridge("cancel_previs_render", {
          projectId,
          id: renderId.current,
        }).catch(() => {});
      fail(e);
      return accepted.current;
    } finally {
      finish();
    }
  }
  async function stop() {
    cancelled.current = true;
    setStopping(true);
    try {
      if (requestId.current)
        await bridge("cancel_previs_chat", { requestId: requestId.current });
      if (renderId.current)
        await bridge("cancel_previs_render", {
          projectId,
          id: renderId.current,
        });
    } catch (e) {
      setError(String(e));
    }
  }
  return {
    agentPartial: agent.partial,
    agentEvents: agent.events,
    busy,
    setBusy,
    lastKind,
    phase,
    error,
    setError,
    pendingPrompt,
    pendingImages,
    pendingRequestId,
    stopping,
    frames,
    seconds,
    renderSource,
    renderFiles,
    renderRuntime,
    send,
    render,
    stop,
    onRenderReady: (value: boolean) => {
      renderReady.current = value;
    },
    onRenderError: (value: string) => {
      renderError.current = value;
    },
  };
}
