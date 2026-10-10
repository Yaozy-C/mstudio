import { deleteAssets } from "../workspace/deleteAssets";
import { removeNodes } from "../canvas/removeNodes";
import { type ComposerMode, referenceIssue } from "./referenceMode";
import { changeBatch } from "./batch";
import { useTaskPanel } from "./useTaskPanel";
import type { AttachmentDraft } from "../assistant/useAttachments";
import { resultPlacement } from "./resultPlacement";
import { canvasViewport } from "../canvas/viewportMemory";
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
import { saveTask } from "./document";
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
  const [composerMode, setComposerMode] = useState<ComposerMode>("agent");
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
  const submit = async (requested: ProductionTask) => {
    if (!native) throw new Error("请在桌面应用中生成");
    const current = get().production?.drafts?.[requested.key];
    if (
      !current ||
      !["AWAITING_CONFIRMATION", "READY"].includes(current.status ?? "")
    )
      return;
    update({ status: "READY", error: undefined }, requested.key);
    await flush();
  };
  const runs = runsOf(project);
  const submitting = runs.some((t) =>
    ["UPLOADING", "SUBMITTING"].includes(t.status ?? ""),
  );
  const taskPanel = useTaskPanel(runs, change, get, open, setComposerMode);
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
    change((p) =>
      item.assetId
        ? deleteAssets(p, [item.assetId])
        : removeNodes(p, [item.nodeId ?? item.ownerId ?? ""]),
    );
    choose(selected.filter((id) => id !== item.key));
  };
  return {
    composerMode,
    setComposerMode: (mode: ComposerMode) => {
      setComposerMode(mode);
      const kind = mode === "reference" ? "video" : mode;
      if (kind !== "agent" && kind !== task.kind)
        update({
          kind,
          modelId: get().production?.models?.[kind] ?? "",
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
      const directReference = composerMode === "reference";
      if (directReference) {
        const issue = referenceIssue(current, model);
        if (issue) throw new Error(issue);
      }
      // Validate before spending a prompt-preparation request.
      directTask(get(), current, model, "validation");
      await flush();
      const prepared = await preparePrompt(
        bridge,
        project.id,
        {
          ...current,
          modelId: model.id,
        },
        directReference,
      );
      if (!active() || get().id !== project.id)
        throw new Error("已停止提示词整理，尚未提交生成任务");
      const bounds = document
        .querySelector(".creation-canvas")
        ?.getBoundingClientRect();
      prepared.position = resultPlacement(
        get(),
        prepared,
        bounds,
        canvasViewport(get()),
      );
      const run = directTask(get(), prepared, model, crypto.randomUUID());
      run.instruction = current.prompt;
      change((p) => acceptDirectTask(p, current, run), false);
      setSelection([]);
      setComposerMode(directReference ? "reference" : run.kind);
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
      update({ inputs });
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
