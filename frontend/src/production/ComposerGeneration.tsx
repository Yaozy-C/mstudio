import { selectMediaModel } from "./frameInputs";
import { useState } from "react";
import { Popover } from "@radix-ui/themes";
import {
  CaretDown,
  Check,
  Image,
  VideoCamera,
  Robot,
  SlidersHorizontal,
} from "@phosphor-icons/react";
import type { Project } from "../model";
import type { ProductionController } from "./useProduction";
import { GenerationSettings } from "./GenerationSettings";
import { ConversationModel } from "../assistant/ConversationModel";
import "./composer-generation.css";

export function ComposerGeneration({
  canvas,
  project,
  settings,
  running,
}: {
  canvas: ProductionController;
  project: Project;
  settings: () => void;
  running: boolean;
}) {
  const [modeOpen, setModeOpen] = useState(false);
  const [modelOpen, setModelOpen] = useState(false);
  const [paramsOpen, setParamsOpen] = useState(false);
  const mode = canvas.composerMode;
  const task = canvas.task;
  const kind = mode === "video" ? "video" : "image";
  const modelId =
    (mode !== "agent" && task?.modelId) || canvas.modelPreferences[kind] || "";
  const selected = canvas.media.models.find((m) => m.id === modelId);
  const Icon =
    mode === "image" ? Image : mode === "video" ? VideoCamera : Robot;
  const params = task?.parameters;
  const summary =
    mode === "video"
      ? `${task?.mode === "ends" ? "首尾帧" : task?.inputs.some((r) => r.role === "first-frame") ? "首帧" : "参考图/视频"} · ${params?.duration ? `${params.duration}s` : "时长"}`
      : `${params?.aspectRatio || "比例"} · ${params?.resolution || (params?.width ? `${params.width}×${params.height}` : "尺寸")}`;
  return (
    <>
      <Popover.Root open={modeOpen} onOpenChange={setModeOpen}>
        <Popover.Trigger>
          <button
            type="button"
            className="composer-mode-chip"
            disabled={running}
            aria-label={`创作模式：${mode === "agent" ? "Agent" : mode === "image" ? "图片" : "视频"}`}
            title={`切换创作模式 · ${mode === "agent" ? "Agent" : mode === "image" ? "图片" : "视频"}`}
          >
            <Icon size={17} />
            <CaretDown size={12} />
          </button>
        </Popover.Trigger>
        <Popover.Content
          side="top"
          align="start"
          className="composer-mode-menu"
        >
          {(["agent", "image", "video"] as const).map((value) => (
            <button
              type="button"
              key={value}
              aria-pressed={mode === value}
              onClick={() => {
                canvas.setComposerMode(value);
                setModeOpen(false);
              }}
            >
              {value === "agent" ? (
                <Robot />
              ) : value === "image" ? (
                <Image />
              ) : (
                <VideoCamera />
              )}
              <span>
                {value === "agent"
                  ? "Agent"
                  : value === "image"
                    ? "图片"
                    : "视频"}
              </span>
              {mode === value && <Check />}
            </button>
          ))}
        </Popover.Content>
      </Popover.Root>
      {mode === "agent" && (
        <ConversationModel
          projectId={project.id}
          settings={settings}
          disabled={running}
        />
      )}
      {mode !== "agent" && task && (
        <Popover.Root open={paramsOpen} onOpenChange={setParamsOpen}>
          <Popover.Trigger>
            <button
              type="button"
              className="composer-setting-chip"
              aria-label={mode === "image" ? "图片生成参数" : "视频生成参数"}
            >
              <span>{summary}</span>
              <CaretDown size={12} />
            </button>
          </Popover.Trigger>
          <Popover.Content
            side="top"
            align="start"
            className="composer-parameters-popover"
          >
            <GenerationSettings
              inline
              task={{ ...task, modelId }}
              project={project}
              models={canvas.media.models}
              close={() => setParamsOpen(false)}
              save={(draft) => {
                canvas.update(draft, draft.key);
                canvas.preferences({ [draft.kind]: draft.modelId });
              }}
            />
          </Popover.Content>
        </Popover.Root>
      )}
      {mode !== "agent" && (
        <Popover.Root open={modelOpen} onOpenChange={setModelOpen}>
          <Popover.Trigger>
            <button
              type="button"
              className="composer-setting-chip"
              title={selected?.name || "选择生成模型"}
              aria-label="选择生成模型"
            >
              <SlidersHorizontal size={17} />
              <span>{selected?.name.split(" · ").at(-1) || "选择模型"}</span>
              <CaretDown size={12} />
            </button>
          </Popover.Trigger>
          <Popover.Content
            side="top"
            align="end"
            className="composer-media-menu"
          >
            <strong>{mode === "image" ? "图片模型" : "视频模型"}</strong>
            {canvas.media.models
              .filter((m) => m.kind === kind && m.enabled)
              .map((m) => (
                <button
                  type="button"
                  className="composer-media-option"
                  key={m.id}
                  aria-pressed={modelId === m.id}
                  onClick={() => {
                    canvas.preferences({ [kind]: m.id });
                    if (task) canvas.update(selectMediaModel(task, m));
                    setModelOpen(false);
                  }}
                >
                  <span>{m.name}</span>
                  {modelId === m.id && <Check size={16} />}
                </button>
              ))}
            {!canvas.media.models.some((m) => m.kind === kind && m.enabled) && (
              <p>还没有可用的{kind === "image" ? "图片" : "视频"}模型</p>
            )}
            <button
              type="button"
              className="composer-manage-models"
              onClick={() => {
                setModelOpen(false);
                settings();
              }}
            >
              管理模型连接
            </button>
          </Popover.Content>
        </Popover.Root>
      )}
    </>
  );
}
