import { changeBatch } from "./batch";
import { useTaskPanel } from "./useTaskPanel";
import type { AttachmentDraft } from "../assistant/useAttachments";
import { resultPlacement } from "./resultPlacement";
import { preparePrompt } from "./preparePrompt";
import { directTask, acceptDirectTask } from "./directTask";
import { setFrameInput, type FrameRole } from "./frameInputs";
import { collectAsset } from "../workspace/assetLibrary";
import { useCardLayout } from "./useCardLayout";
import { useEffect, useMemo, useState } from "react";
import { native, bridge } from "../bridge";
import { runsOf } from "./requestTask";
import type { AttachmentRef } from "../assistant/attachments";
import { attachmentInput, sameInput } from "./attachmentInput";
import { referencedItems } from "./referencedItems";
import { type Asset, type Project } from "../model";
import { useGenerationAgent } from "../creation/useGenerationAgent";
import { productionItems } from "./items";
import { createTask, taskKey } from "./tasks";
import { saveTask, recoverUploads } from "./document";
import { useProductionSubmit } from "./useProductionSubmit";
import type {
  ChangeProject,
  GenerationPreferences,
  ProductionItem,
  ProductionTask,
} from "./types";

export function useProduction(
  project: Project,
  change: ChangeProject,
  get: () => Project,
  flush: () => Promise<void>,
  open: () => void,
  attachments: AttachmentDraft,
) {
  const rawItems = useMemo(
    () => productionItems(project),
    [
      project.nodes,
      project.assets,
      project.production?.positions,
      project.production?.hidden,
    ],
  );
  const { items, measure } = useCardLayout(rawItems, project);
  const [composerMode, setComposerMode] = useState<"agent" | "image" | "video">(
    "agent",
  );
  const [selection, setSelection] = useState<string[]>([]);
  const selected = selection.filter((id) => items.some((n) => n.key === id));
  const key = taskKey([]);
  const task = project.production?.drafts?.[key] ?? createTask(project, []);
  const media = useGenerationAgent();
  const [focus, setFocus] = useState<{
    id?: string;
    itemKey?: string;
    tick: number;
  }>({
    tick: 0,
  });
  useEffect(() => {
    change((p) => recoverUploads(p), false);
  }, [project.id, change]);
  function update(patch: Partial<ProductionTask>, targetKey = key) {
    change(
      (p) =>
        saveTask(p, {
          ...(p.production?.drafts?.[targetKey] ?? task),
          error: undefined,
          ...patch,
          ...(targetKey === key ? { ownerId: undefined } : {}),
        }),
      false,
    );
  }
  const { submit, submitting, available, slotVersion } = useProductionSubmit({
    projectId: project.id,
    media,
    get,
    flush,
    update,
  });
  const runs = runsOf(project);
  const taskPanel = useTaskPanel(runs, change, get, open, setComposerMode);
  useEffect(() => {
    if (!native) return;
    for (const next of runs.filter((t) => t.status === "READY")) {
      if (available() <= 0) break;
      void submit(next);
    }
  }, [project.production?.drafts, slotVersion]);
  function preferences(patch: Partial<GenerationPreferences>) {
    change(
      (p) => ({
        ...p,
        production: {
          ...p.production,
          models: { ...p.production?.models, ...patch },
        },
      }),
      false,
    );
  }
  function choose(ids: string[]) {
    setSelection(ids);
  }
  useEffect(() => {
    if (!attachments.items.length) return;
    const current = get().production?.drafts?.[key] ?? task;
    const inputs = [...current.inputs];
    for (const ref of attachments.items) {
      const input = attachmentInput(get(), ref);
      if (input && !inputs.some((r) => sameInput(r, input))) inputs.push(input);
    }
    update({ inputs });
    attachments.consume(attachments.items);
  }, [attachments.items]);
  function attach(ref: AttachmentRef, role?: FrameRole) {
    const task = get().production?.drafts?.[key] ?? createTask(get(), []);
    const input = attachmentInput(get(), ref);
    if (role) {
      if (!input) throw new Error("请选择图片");
      update(setFrameInput(task, get(), role, input));
      return;
    }
    if (!input || task.inputs.some((r) => sameInput(r, input))) return;
    if (task.inputs.length >= 12) {
      update({ error: "本次最多引用 12 个素材" });
      return;
    }
    update({ inputs: [...task.inputs, input] });
  }

  function select(id: string, multi = false) {
    choose(
      multi
        ? selected.includes(id)
          ? selected.filter((v) => v !== id)
          : [...selected, id]
        : [id],
    );
  }
  function act(
    kind: ProductionTask["kind"],
    ids = selected,
    refs: AttachmentRef[] = [],
  ) {
    setComposerMode(kind);
    open();
    const next = createTask(
      get(),
      ids.map((id) => items.find((n) => n.key === id)!).filter(Boolean),
      kind,
    );
    for (const ref of refs) {
      const input = attachmentInput(get(), ref);
      if (input && !next.inputs.some((r) => sameInput(r, input)))
        next.inputs.push(input);
    }
    next.key = key;
    change((p) => saveTask(p, next), false);
    return next;
  }
  const move = (id: string, x: number, y: number) =>
    change((p) => ({
      ...p,
      production: {
        ...p.production,
        positions: { ...p.production?.positions, [id]: { x, y } },
      },
    }));
  const remove = (item: ProductionItem) => {
    change((p) => ({
      ...p,
      production: {
        ...p.production,
        hidden: [...(p.production?.hidden ?? []), item.key],
      },
    }));
    choose(selected.filter((id) => id !== item.key));
  };
  return {
    composerMode,
    setComposerMode: (mode: "agent" | "image" | "video") => {
      setComposerMode(mode);
      if (mode !== "agent" && mode !== task.kind)
        update({
          kind: mode,
          modelId: get().production?.models?.[mode] ?? "",
          parameters: {},
        });
    },
    execute: async (active: () => boolean = () => true) => {
      const current = structuredClone(get().production?.drafts?.[key] ?? task);
      const modelId =
        current.modelId || get().production?.models?.[current.kind];
      const model = media.models.find((m) => m.id === modelId);
      if (!model) throw new Error("请先选择生成模型");
      if (!native) throw new Error("请在桌面应用中生成");
      // Validate before spending a prompt-preparation request.
      directTask(get(), current, model, "validation");
      await flush();
      const prepared = await preparePrompt(bridge, project.id, {
        ...current,
        modelId: model.id,
      });
      if (!active() || get().id !== project.id)
        throw new Error("已停止提示词整理，尚未提交生成任务");
      if (
        JSON.stringify(get().production?.drafts?.[key] ?? task) !==
        JSON.stringify(current)
      )
        throw new Error("输入内容已变化，请重新发送");
      const bounds = document
        .querySelector(".creation-canvas")
        ?.getBoundingClientRect();
      prepared.position = resultPlacement(get(), prepared, bounds);
      const run = directTask(get(), prepared, model, crypto.randomUUID());
      run.instruction = current.prompt;
      change((p) => acceptDirectTask(p, current, run), false);
      setSelection([]);
      setComposerMode(run.kind);
      return run;
    },
    ...taskPanel,
    measure,
    items,
    selected,
    referenced: referencedItems(project, items, task, attachments.items),
    task: task as ProductionTask | undefined,
    media,
    submitting,
    focus,
    choose,
    select,
    act,
    update,
    move,
    remove,
    submit,
    batch: (turnId: string, action: "resume" | "stop" | "retry") =>
      change((p) => changeBatch(p, turnId, action)),
    runs,
    preferences,
    modelPreferences: project.production?.models ?? {},
    attach,
    reference: (ids: string[]) => {
      const current = get().production?.drafts?.[key] ?? task;
      const added = createTask(
        get(),
        items.filter((n) => ids.includes(n.key)),
      ).inputs.map((r) => ({
        ...r,
        role: r.assetId
          ? get().assets.find((a) => a.id === r.assetId)?.kind === "video"
            ? ("video-reference" as const)
            : ("reference" as const)
          : ("script" as const),
      }));
      const inputs = [...current.inputs];
      for (const input of added)
        if (
          !inputs.some(
            (r) =>
              r.key === input.key || (r.assetId && r.assetId === input.assetId),
          )
        )
          inputs.push(input);
      if (inputs.length > 12) update({ error: "本次最多引用 12 个素材" });
      else update({ inputs });
      open();
    },
    referenceAsset: async (asset: Asset, global = false, role?: FrameRole) => {
      if (global && !get().assets.some((a) => a.id === asset.id)) {
        const saved = await bridge<Asset>("use_global_asset", {
          projectId: project.id,
          id: asset.id,
        });
        change((p) => collectAsset(p, saved), false);
        attach({ kind: "asset", id: saved.id }, role);
      } else attach({ kind: "asset", id: asset.id }, role);
    },
    focusShot: (id?: string) => setFocus({ id, tick: Date.now() }),
    focusItem: (itemKey: string) => {
      setSelection([itemKey]);
      setFocus({ itemKey, tick: Date.now() });
    },
  };
}
export type ProductionController = ReturnType<typeof useProduction>;
