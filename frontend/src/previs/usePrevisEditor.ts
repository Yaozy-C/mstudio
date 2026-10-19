import { useEffect, useRef, useState } from "react";
import { t } from "../i18n";
import { useModels } from "../models/useModels";
import { selectedModel } from "../models/types";
import type { RuntimeHandle } from "./RuntimeFrame";
import { usePrevisTask } from "./usePrevisTask";
import { usePrevis } from "./context";
import type { PrevisExport } from "./types";
import { usePrevisImages } from "./usePrevisImages";
import { supportsInput } from "../models/inputCapabilities";
import { submitPrevisDraft } from "./submitDraft";

export function usePrevisEditor(fileId: string, close: () => void) {
  const {
    workspace,
    project,
    refresh,
    save,
    attachReference,
    collectVideo,
    change,
    flush,
  } = usePrevis();
  const file = workspace.files.find((f) => f.id === fileId)!;
  const [draft, setDraft] = useState(file);
  const [linkingId, setLinkingId] = useState("");
  const [savingId, setSavingId] = useState("");
  const referencePending = useRef(false);
  const [prompt, setPrompt] = useState("");
  const images = usePrevisImages(project, change);
  const [ready, setReady] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [time, setTime] = useState(0);
  const [modelId, setModelId] = useState("");
  const runtime = useRef<RuntimeHandle>(null);
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
  async function persist() {
    await flush();
    if (!dirty) return file;
    const saved = await save(draft);
    setDraft(saved);
    return saved;
  }
  const task = usePrevisTask({
    projectId: project.id,
    fileId,
    model,
    persist,
    refresh,
    onResult: setDraft,
  });
  const { busy, setBusy, error, setError } = task;
  async function send(text = prompt) {
    if (!text.trim() || busy || images.busy) return;
    if (images.items.length && (!model || !supportsInput(model, "image")))
      return;
    await submitPrevisDraft(text, images, setPrompt, task.send, setError);
  }
  async function render() {
    if (busy || images.busy) return;
    if (draft.code.trim()) {
      await task.render();
      return;
    }
    if (images.items.length && (!model || !supportsInput(model, "image")))
      return;
    const submitted = [...images.items];
    images.clear();
    if (!(await task.render(submitted))) images.restore(submitted);
  }
  function onReady(value: boolean) {
    setReady(value);
    if (!value) setPlaying(false);
  }
  async function useVideo(video: PrevisExport, collect = false) {
    if (busy || referencePending.current) return;
    referencePending.current = true;
    if (collect) setSavingId(video.id);
    else setLinkingId(video.id);
    setBusy("share");
    setError("");
    try {
      if (collect) await collectVideo(video.id, video.asset.id);
      else await attachReference(fileId, video.id, video.asset.id);
    } catch (e) {
      setError(String(e));
    } finally {
      referencePending.current = false;
      setLinkingId("");
      setSavingId("");
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
    if (busy === "chat" || busy === "render") {
      close();
      return;
    }
    if (dirty) {
      setError(t("请先保存当前修改，或放弃未保存修改后关闭。"));
      return;
    }
    close();
  };
  return {
    ...task,
    fileId,
    workspace,
    project,
    file,
    draft,
    setDraft,
    prompt,
    setPrompt,
    images,
    error,
    setError,
    busy,
    ready,
    playing,
    setPlaying,
    time,
    setTime,
    model,
    setModelId,
    runtime,
    catalog,
    models,
    messages,
    exports,
    dirty,
    send,
    render,
    onReady,
    dependency,
    manualSave,
    dismiss,
    addReference: (video: PrevisExport) => useVideo(video),
    saveVideo: (video: PrevisExport) => useVideo(video, true),
    linkingId,
    savingId,
  };
}
export type PrevisEditorState = ReturnType<typeof usePrevisEditor>;
