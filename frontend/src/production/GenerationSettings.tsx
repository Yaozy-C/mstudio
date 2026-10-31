import { MediaModelSelect } from "../models/MediaModelSelect";
import { StudioSelect } from "../ui/StudioSelect";
import { GenerationOptions } from "./GenerationOptions";
import { GenerationDuration } from "./GenerationDuration";
import { TaskTitleField } from "./TaskTitleField";
import { normalizeTaskTitle } from "./taskTitle";
import { t, useLanguage } from "../i18n";
import { failure } from "../errors/failure";
import { ErrorNotice } from "../errors/ErrorNotice";
import { selectMediaModel } from "./frameInputs";
import { GenerationMethod } from "./GenerationMethod";
import { ParameterChoices } from "./ParameterChoices";
import { GenerationReferences } from "./GenerationReferences";
import { useState } from "react";
import { Dialog } from "@radix-ui/themes";
import { X } from "@phosphor-icons/react";
import type { Project } from "../model";
import { canUseTaskReference } from "../workspace/resourceRoles";
import { mediaUrl } from "../bridge";
import type { MediaModel } from "../models/mediaRegistry";
import { mediaAdapter } from "../models/adapters";
import { inputFor } from "./request";
import {
  effectiveParameters,
  parameterFields,
  taskParameters,
} from "./parameters";
import type { GenerationParameters } from "./parameters";
import type { ProductionTask } from "./types";
import "./generation-panel.css";

export function GenerationSettings({
  task,
  project,
  models,
  close,
  save,
  inline = false,
  onPatch,
}: {
  inline?: boolean;
  onPatch?: (value: Partial<ProductionTask>) => void;
  task: ProductionTask;
  project: Project;
  models: MediaModel[];
  close: () => void;
  save: (task: ProductionTask, start: boolean) => void;
}) {
  useLanguage();
  const [localDraft, setDraft] = useState(() => structuredClone(task));
  const draft = onPatch ? task : localDraft;
  const [error, setError] = useState("");
  const model = models.find(
    (m) => m.id === draft.modelId && m.kind === draft.kind,
  );
  const fields = parameterFields(model);
  const options = draft.parameters ?? {};
  const displayed = effectiveParameters(model, options);
  const patch = (value: Partial<ProductionTask>) => {
    if (onPatch) onPatch(value);
    else setDraft((d) => ({ ...d, ...value }));
    setError("");
  };
  const param = (value: GenerationParameters) =>
    patch({ parameters: { ...options, ...value } });
  const adapter = model ? mediaAdapter(model) : undefined;
  const frames =
    draft.kind === "video" &&
    adapter?.fields.some((f) => f.role === "first-frame");
  const tail = frames && adapter?.fields.some((f) => f.role === "last-frame");
  const assets = project.assets.filter(
    (a) => a.kind === "image" && canUseTaskReference(project, a, task),
  );
  function frame(role: "first-frame" | "last-frame", assetId: string) {
    const inputs = draft.inputs.filter((r) => r.role !== role);
    if (assetId)
      inputs.push({
        key: `asset:${assetId}:${role}`,
        assetId,
        role,
        purpose: role === "first-frame" ? t("视频首帧") : t("视频尾帧"),
      });
    patch({
      inputs,
      mode: inputs.some((r) => r.role === "last-frame") ? "ends" : "single",
    });
  }
  function commit(start: boolean) {
    try {
      if (!model) throw failure("VALIDATION_FAILED", t("请先选择生成模型"));
      if (inline) taskParameters(model, draft.parameters);
      else
        inputFor(
          project,
          { ...draft, prompt: draft.prompt || (start ? "" : t("参数校验")) },
          model,
        );
      const saved = { ...draft, error: undefined };
      if (draft.title !== task.title)
        saved.title = normalizeTaskTitle(draft.title);
      save(saved, start);
      close();
    } catch (e) {
      setError(String(e).replace(/^Error: /, ""));
    }
  }
  const framePicker = (role: "first-frame" | "last-frame", label: string) => {
    const selected = assets.find(
      (a) => a.id === draft.inputs.find((r) => r.role === role)?.assetId,
    );
    return (
      <label className="generation-frame-slot">
        <span>{label}</span>
        <div>
          {selected ? (
            <img
              src={mediaUrl(selected.preview || selected.path)}
              alt={selected.name}
            />
          ) : (
            <span className="frame-empty">{t("选择图片")}</span>
          )}
        </div>
        <StudioSelect
          label={label}
          value={selected?.id ?? ""}
          onValueChange={(id) => frame(role, id)}
          options={[
            { value: "", label: `${t("不使用")}${label}` },
            ...assets.map((a) => ({ value: a.id, label: a.name })),
          ]}
        />
      </label>
    );
  };
  const content = (
    <>
      {!onPatch && (
        <div className="generation-settings-heading">
          <h3>{draft.kind === "image" ? t("图片设置") : t("视频设置")}</h3>
          <button aria-label={t("关闭生成设置")} onClick={close}>
            <X size={18} />
          </button>
        </div>
      )}
      {draft.ownerId && !onPatch && (
        <p className="generation-setting-note">
          {project.nodes.find((n) => n.id === draft.ownerId)?.title}
        </p>
      )}
      {!inline && (
        <TaskTitleField
          value={draft.title ?? ""}
          change={(title) => patch({ title })}
          commitOnBlur={!!onPatch}
        />
      )}
      {!inline && (
        <label className="generation-setting-field">
          {t("生成模型")}
          <MediaModelSelect
            value={draft.modelId}
            models={models.filter((m) => m.kind === draft.kind)}
            change={(next) => patch(selectMediaModel(draft, next))}
          />
        </label>
      )}
      <GenerationMethod draft={draft} models={models} change={patch} />
      <div className="generation-settings-grid">
        {!!fields.ratios.length && (
          <ParameterChoices
            label={t("画面比例")}
            ratios
            values={fields.ratios}
            value={displayed.aspectRatio}
            change={(value) => param({ aspectRatio: value })}
          />
        )}
        {!!fields.resolutions.length && (
          <ParameterChoices
            label={t("分辨率")}
            values={fields.resolutions}
            value={displayed.resolution}
            change={(value) => param({ resolution: value })}
          />
        )}
        {fields.customSize && (
          <>
            {(["width", "height"] as const).map((k) => (
              <label className="generation-setting-field" key={k}>
                {k === "width" ? t("宽度（px）") : t("高度（px）")}
                <input
                  type="number"
                  step={1}
                  min={fields.controls.imageSize?.min ?? 1}
                  max={fields.controls.imageSize?.max ?? undefined}
                  placeholder={t("模型默认")}
                  value={options[k] ?? ""}
                  onChange={(e) =>
                    param({
                      [k]: e.target.value ? Number(e.target.value) : undefined,
                    })
                  }
                />
              </label>
            ))}
          </>
        )}
        {(fields.duration ||
          (draft.kind === "video" && displayed.duration !== undefined)) && (
          <GenerationDuration
            value={displayed.duration}
            range={fields.durationRange}
            onChange={(duration) => param({ duration })}
          />
        )}
        {model && (
          <GenerationOptions
            model={model}
            parameters={options}
            change={(extra) => param({ options: extra })}
          />
        )}
      </div>
      {fields.frameRatio && (
        <p className="generation-setting-note">{t("画面比例跟随首帧图片。")}</p>
      )}
      {!inline && frames && (
        <div className="generation-settings-grid">
          {framePicker("first-frame", t("首帧"))}
          {tail && framePicker("last-frame", t("尾帧"))}
        </div>
      )}
      {!inline && (
        <GenerationReferences
          {...{ project, draft, model, frames, tail, frame, patch }}
        />
      )}
      {!inline && !onPatch && (
        <details className="generation-settings-prompt">
          <summary>{t("生成描述")}</summary>
          <textarea
            aria-label={t("生成描述")}
            value={draft.prompt}
            onChange={(e) => patch({ prompt: e.target.value })}
            rows={4}
          />
        </details>
      )}
      {error && <ErrorNotice error={error} fallback="VALIDATION_FAILED" />}
      {!onPatch && (
        <footer>
          <button onClick={close}>{t("取消")}</button>
          <button onClick={() => commit(false)}>{t("保存设置")}</button>
          {task.turnId && (
            <button onClick={() => commit(true)}>{t("生成")}</button>
          )}
        </footer>
      )}
    </>
  );
  if (onPatch) return <div className="task-generation-settings">{content}</div>;
  if (inline)
    return <div className="generation-settings-inline">{content}</div>;
  return (
    <Dialog.Root
      open
      onOpenChange={(open) => {
        if (!open) close();
      }}
    >
      <Dialog.Content
        aria-label={draft.kind === "image" ? t("图片设置") : t("视频设置")}
        aria-describedby={undefined}
        className="generation-settings"
        maxWidth="460px"
      >
        {content}
      </Dialog.Content>
    </Dialog.Root>
  );
}
