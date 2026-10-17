import { Dialog } from "@radix-ui/themes";
import { useEffect } from "react";
import { Cube, X, DownloadSimple, ArrowLeft } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import { readyModel } from "../models/types";
import { AsyncButton } from "../ui/AsyncState";
import { ActionButton } from "../ui/ActionButton";
import { IconButton } from "../ui/IconButton";
import { usePrevisEditor, type PrevisEditorState } from "./usePrevisEditor";
import { PrevisVisual } from "./PrevisVisual";
import { PrevisChat } from "./PrevisChat";
import { RuntimeFrame } from "./RuntimeFrame";
import type { PrevisTaskSummary } from "./PrevisTaskList";

/** The session stays mounted when its dialog closes or another shot opens. */
export function PrevisEditor({
  fileId,
  active,
  close,
  publishTask,
}: {
  fileId: string;
  active: boolean;
  close: () => void;
  publishTask: (task: PrevisTaskSummary) => void;
}) {
  useLanguage();
  const state = usePrevisEditor(fileId, close);
  const { file, busy, phase, stopping, error, lastKind } = state;
  useEffect(() => {
    publishTask({
      fileId,
      name: file.name,
      running: busy === "chat" || busy === "render",
      phase,
      stopping,
      error,
      lastKind,
    });
  }, [publishTask, fileId, file.name, busy, phase, stopping, error, lastKind]);
  return (
    <>
      {state.renderSource && (
        <div className="previs-render-runtime" aria-hidden="true" inert>
          <RuntimeFrame
            ref={state.renderRuntime}
            projectId={state.project.id}
            file={state.renderSource}
            files={state.renderFiles}
            onReady={state.onRenderReady}
            onError={state.onRenderError}
            onTime={() => {}}
          />
        </div>
      )}
      {active && <PrevisEditorDialog state={state} />}
    </>
  );
}
export function PrevisEditorDialog({ state }: { state: PrevisEditorState }) {
  const { busy, dismiss, file, draft, model, render } = state;
  const running = busy === "chat" || busy === "render";
  return (
    <Dialog.Root
      open
      onOpenChange={(v) => {
        if (!v) dismiss();
      }}
    >
      <Dialog.Content className="previs-dialog" aria-describedby={undefined}>
        <header className="previs-heading">
          <div>
            <Cube size={20} />
            <Dialog.Title title={file.name}>
              {file.kind === "model" ? t("共用模型") : t("镜头预演")} ·{" "}
              {file.name}
            </Dialog.Title>
          </div>
          <div className="previs-heading-actions">
            {running ? (
              <ActionButton icon={ArrowLeft} onClick={dismiss}>
                {t("后台继续")}
              </ActionButton>
            ) : (
              file.kind === "shot" && (
                <AsyncButton
                  className="primary"
                  busyLabel={t("正在生成预演…")}
                  disabled={
                    !!busy ||
                    (!draft.code.trim() && (!model || !readyModel(model)))
                  }
                  onClick={() => void render()}
                >
                  <DownloadSimple />
                  {draft.code.trim() ? t("生成预演视频") : t("一键生成预演")}
                </AsyncButton>
              )
            )}
            <IconButton
              className="previs-close"
              icon={X}
              label={t("关闭预演")}
              onClick={dismiss}
            />
          </div>
        </header>
        <div className="previs-editor-grid">
          <PrevisVisual state={state} />
          <PrevisChat state={state} />
        </div>
      </Dialog.Content>
    </Dialog.Root>
  );
}
