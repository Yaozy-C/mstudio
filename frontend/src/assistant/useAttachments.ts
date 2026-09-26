import { importBrowserFile, type ImportedFiles } from "./fileImports";
import type { CreativeTask } from "../creative/aiTasks";
import { useCallback, useEffect, useRef, useState } from "react";
import { bridge } from "../bridge";
import type { Asset, Project } from "../model";
import {
  attachmentKey,
  mergeAttachments,
  MAX_ATTACHMENTS,
  type AttachmentRef,
} from "./attachments";
export function useAttachments(
  projectId: string,
  change: (fn: (p: Project) => Project) => void,
) {
  const [items, setItems] = useState<AttachmentRef[]>([]);
  const current = useRef(items);
  const update = (next: AttachmentRef[]) => {
    current.current = next;
    setItems(next);
  };
  const [targetNodeId, setTargetNodeId] = useState<string | null>(null);
  const [omitWork, setOmitWork] = useState(false);
  const [focus, setFocus] = useState(0);
  const [suggestion, setSuggestion] = useState<CreativeTask | null>(null);
  useEffect(() => {
    const request = (e: Event) => {
      const task = (e as CustomEvent<CreativeTask>).detail;
      const refs = task.refs ?? [];
      if (
        new Set([...current.current, ...refs].map(attachmentKey)).size >
        MAX_ATTACHMENTS
      ) {
        setNotice("本轮最多 12 个附件，请先移除部分附件，再发起 AI 任务。");
        return;
      }
      update(mergeAttachments(current.current, refs));
      setTargetNodeId(task.targetNodeId ?? null);
      setNotice("");
      setSuggestion(task);
      setFocus((v) => v + 1);
    };
    window.addEventListener("studio-creative-request", request);
    return () => {
      window.removeEventListener("studio-creative-request", request);
    };
  }, []);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const add = useCallback((refs: AttachmentRef[]) => {
    const count = new Set([...current.current, ...refs].map(attachmentKey))
      .size;
    if (count > MAX_ATTACHMENTS) {
      setNotice("本轮最多 12 个附件，请先移除不需要的内容。");
      return;
    }
    setNotice("");
    update(mergeAttachments(current.current, refs));
    setFocus((v) => v + 1);
  }, []);
  const remove = (ref: AttachmentRef) => {
    update(
      current.current.filter((a) => attachmentKey(a) !== attachmentKey(ref)),
    );
    setNotice("");
  };
  const consume = useCallback((sent: AttachmentRef[]) => {
    const keys = new Set(sent.map(attachmentKey));
    update(current.current.filter((a) => !keys.has(attachmentKey(a))));
  }, []);
  const queue = useRef(Promise.resolve());
  const pending = useRef(0);
  function receiveAssets(
    assets: Asset[],
    receive?: (refs: AttachmentRef[]) => void,
  ) {
    if (!assets.length) return false;
    change((p) => ({
      ...p,
      assets: [
        ...p.assets,
        ...assets.filter((a) => !p.assets.some((v) => v.id === a.id)),
      ],
    }));
    const refs = assets.map((a) => ({ kind: "asset" as const, id: a.id }));
    if (receive) {
      receive(refs);
      return false;
    }
    const overflow = current.current.length + refs.length > MAX_ATTACHMENTS;
    update(mergeAttachments(current.current, refs));
    setFocus((v) => v + 1);
    return overflow;
  }
  function enqueue(run: () => Promise<void>) {
    pending.current++;
    setBusy(true);
    const task = queue.current.then(async () => {
      setNotice("正在导入附件…");
      try {
        await run();
      } catch (error) {
        setNotice(String(error));
      } finally {
        if (--pending.current === 0) setBusy(false);
      }
    });
    queue.current = task;
    return task;
  }
  function importFiles(
    receive?: (refs: AttachmentRef[]) => void,
    clipboard = false,
  ) {
    return enqueue(async () => {
      const result = await bridge<ImportedFiles>(
        clipboard ? "import_clipboard_files" : "import_media",
        { projectId },
      );
      const overflow = receiveAssets(result.assets, receive);
      setNotice(
        [
          ...result.errors,
          ...(overflow
            ? ["本轮最多 12 个附件，其余文件已保留在素材库，可用 @ 引用。"]
            : []),
        ].join("\n"),
      );
    });
  }
  function importDroppedFiles(
    files: File[],
    receive?: (refs: AttachmentRef[]) => void,
  ) {
    return enqueue(async () => {
      const errors: string[] = [];
      let overflow = false;
      for (const [index, file] of files.entries()) {
        try {
          setNotice(
            `正在导入 ${index + 1}/${files.length} · ${file.name || "粘贴的文件"}`,
          );
          const asset = await importBrowserFile(projectId, file, (percent) =>
            setNotice(
              `正在导入 ${index + 1}/${files.length} · ${file.name || "粘贴的文件"} · ${percent}%`,
            ),
          );
          overflow = receiveAssets([asset], receive) || overflow;
        } catch (error) {
          errors.push(`${file.name || "文件"}：${String(error)}`);
        }
      }
      setNotice(
        [
          ...errors,
          ...(overflow
            ? ["本轮最多 12 个附件，其余文件已保留在素材库，可用 @ 引用。"]
            : []),
        ].join("\n"),
      );
    });
  }
  return {
    items,
    targetNodeId,
    omitWork,
    dismissTarget: (id: string) => {
      setTargetNodeId(null);
      setOmitWork(true);
      remove({ kind: "node", id });
    },
    clearTarget: () => {
      setTargetNodeId(null);
    },
    consumeTarget: (id: string | null) => {
      setTargetNodeId((current) => (current === id ? null : current));
    },
    focus,
    busy,
    notice,
    add,
    remove,
    consume,
    importFiles,
    importDroppedFiles,
    suggestion,
    clearSuggestion: () => setSuggestion(null),
  };
}
export type AttachmentDraft = ReturnType<typeof useAttachments>;
