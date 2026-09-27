import { t, useLanguage } from "../i18n";
import { MediaReferenceStrip } from "./MediaReferenceStrip";
import { X, ArrowsLeftRight } from "@phosphor-icons/react";
import { frameRoles, setFrameInput } from "../production/frameInputs";
import type { Project } from "../model";
import type { ProductionController } from "../production/useProduction";
import type { AttachmentDraft } from "./useAttachments";
import { ComposerReference } from "./ComposerReference";

export function ComposerMediaInputs(props: {
  canvas: ProductionController;
  draft: AttachmentDraft;
  project: Project;
}) {
  useLanguage();
  const { canvas, project } = props;
  const task = canvas.task;
  const model = canvas.media.models.find(
    (m) =>
      m.id ===
      (task?.modelId ||
        canvas.modelPreferences[
          canvas.composerMode === "video" ? "video" : "image"
        ]),
  );
  const roles = canvas.composerMode === "video" ? frameRoles(model) : [];
  if (!task || !roles.length) return <MediaReferenceStrip {...props} />;
  return (
    <>
      <div className="composer-frame-inputs" aria-label={t("视频帧素材")}>
        {roles.map((role) => {
          const selected = task.inputs.some((r) => r.role === role);
          return (
            <div className="composer-frame-input" key={role}>
              <ComposerReference {...props} role={role} />
              {selected && (
                <button
                  type="button"
                  className="composer-frame-remove"
                  aria-label={
                    role === "first-frame" ? t("移除首帧") : t("移除尾帧")
                  }
                  onClick={() =>
                    canvas.update(setFrameInput(task, project, role))
                  }
                >
                  <X size={12} />
                </button>
              )}
            </div>
          );
        })}
        {roles.length === 2 &&
          roles.every((role) => task.inputs.some((r) => r.role === role)) && (
            <button
              type="button"
              className="composer-frame-swap"
              title={t("交换首尾帧")}
              aria-label={t("交换首尾帧")}
              onClick={() =>
                canvas.update({
                  inputs: task.inputs.map((r) =>
                    r.role === "first-frame"
                      ? { ...r, role: "last-frame", purpose: t("视频尾帧") }
                      : r.role === "last-frame"
                        ? { ...r, role: "first-frame", purpose: t("视频首帧") }
                        : r,
                  ),
                })
              }
            >
              <ArrowsLeftRight size={16} />
            </button>
          )}
      </div>
      <MediaReferenceStrip {...props} extrasOnly hiddenRoles={roles} />
    </>
  );
}
