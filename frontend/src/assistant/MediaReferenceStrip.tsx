import { isVideoMode } from "../production/referenceMode";
import { ZoomableImage } from "../workspace/ZoomableImage";
import { VideoReferenceRange } from "../production/VideoReferenceRange";
import { t, useLanguage } from "../i18n";
import { useState } from "react";
import { Dialog, Popover } from "@radix-ui/themes";
import {
  FileText,
  Image,
  VideoCamera,
  X,
  DotsThree,
} from "@phosphor-icons/react";
import { mediaUrl } from "../bridge";
import type { Project } from "../model";
import type { ProductionController } from "../production/useProduction";
import type { ProductionInput } from "../production/types";
import { roleLabels } from "../production/request";
import type { AttachmentDraft } from "./useAttachments";
import { ComposerReference } from "./ComposerReference";
import { describeAttachment } from "./attachments";
import "../styles/composer-reference-labels.css";

export function MediaReferenceStrip(props: {
  canvas: ProductionController;
  project: Project;
  draft: AttachmentDraft;
  extrasOnly?: boolean;
  hiddenRoles?: string[];
}) {
  useLanguage();
  const { canvas, project } = props;
  const [preview, setPreview] = useState<ProductionInput>();
  const task = canvas.task;
  const inputs =
    task?.inputs.filter((r) => !props.hiddenRoles?.includes(r.role)) ?? [];
  const asset = project.assets.find((a) => a.id === preview?.assetId);
  function reorder(key: string, before: string) {
    if (!task || key === before) return;
    const moved = task.inputs.find((r) => r.key === key);
    if (!moved) return;
    const next = task.inputs.filter((r) => r.key !== key);
    next.splice(
      next.findIndex((r) => r.key === before),
      0,
      moved,
    );
    canvas.update({ inputs: next });
  }
  function purpose(ref: ProductionInput, role: ProductionInput["role"]) {
    if (!task) return;
    const inputs = task.inputs.map((r): ProductionInput =>
      r.key === ref.key
        ? { ...r, role }
        : ["edit", "first-frame", "last-frame"].includes(role) &&
            r.role === role
          ? { ...r, role: "reference" }
          : r,
    );
    canvas.update({
      inputs,
      ...(isVideoMode(canvas.composerMode)
        ? {
            mode: inputs.some((r) => r.role === "last-frame")
              ? "ends"
              : inputs.some((r) => r.role === "first-frame")
                ? "single"
                : "multi",
          }
        : {}),
    });
  }
  return (
    <>
      <div
        className={`composer-media-strip${canvas.composerMode === "agent" ? " composer-agent-references" : ""}`}
        aria-label={t("本次引用")}
      >
        {inputs.map((ref) => {
          const a = project.assets.find((a) => a.id === ref.assetId);
          const node = project.nodes.find((n) => n.id === ref.nodeId);
          const title =
            (ref.sourceRef &&
              describeAttachment(project, ref.sourceRef)?.title) ??
            a?.name ??
            node?.title ??
            t("素材已移除");
          const src = a && (a.preview || (a.kind === "image" ? a.path : ""));
          const invalid = (!a && !node) || a?.missing;
          return (
            <div
              className={`composer-media-thumb${invalid ? " invalid" : ""}`}
              key={ref.key}
              title={title}
              draggable
              onDragStart={(e) => {
                e.dataTransfer.setData("application/x-mstudio-order", ref.key);
              }}
              onDragOver={(e) => {
                if (
                  e.dataTransfer.types.includes("application/x-mstudio-order")
                ) {
                  e.preventDefault();
                  e.stopPropagation();
                }
              }}
              onDrop={(e) => {
                const key = e.dataTransfer.getData(
                  "application/x-mstudio-order",
                );
                if (key) {
                  e.preventDefault();
                  e.stopPropagation();
                  reorder(key, ref.key);
                }
              }}
            >
              <button
                className="reference-preview"
                type="button"
                aria-label={t("预览引用 {v0}", { v0: title })}
                onClick={() => setPreview(ref)}
              >
                {src ? (
                  <img src={mediaUrl(src)} alt={title} />
                ) : ref.role === "script" ? (
                  <>
                    <FileText size={20} />
                    <small>{title}</small>
                  </>
                ) : a?.kind === "video" ? (
                  <VideoCamera size={22} />
                ) : (
                  <Image size={22} />
                )}
              </button>
              {canvas.composerMode === "agent" && (
                <span className="reference-object-label">
                  <span>{title}</span>
                  {ref.sourceRef?.kind === "clip" && (
                    <small>{t("时间线片段")}</small>
                  )}
                </span>
              )}
              {canvas.composerMode !== "agent" && ref.role !== "reference" && (
                <span className="reference-role">
                  {ref.role === "edit"
                    ? t("修改这张")
                    : t(roleLabels[ref.role])}
                  {a?.kind === "video" &&
                    ` · ${(ref.start ?? 0).toFixed(2)}–${(ref.end ?? a.duration).toFixed(2)}s`}
                </span>
              )}
              <button
                type="button"
                className="composer-media-remove"
                aria-label={t("移除引用 {v0}", { v0: title })}
                title={t("移除")}
                onClick={() =>
                  canvas.update({
                    inputs: task!.inputs.filter((r) => r.key !== ref.key),
                  })
                }
              >
                <X size={11} weight="bold" />
              </button>
              <Popover.Root>
                <Popover.Trigger>
                  <button
                    type="button"
                    className="reference-options"
                    aria-label={t("引用用途 {v0}", { v0: title })}
                    title={t("用途与顺序")}
                  >
                    <DotsThree size={16} />
                  </button>
                </Popover.Trigger>
                <Popover.Content side="top" className="reference-options-menu">
                  {a?.kind === "video" && (
                    <VideoReferenceRange
                      input={ref}
                      duration={a.duration}
                      change={(patch) =>
                        canvas.update({
                          inputs: task!.inputs.map((r) =>
                            r.key === ref.key ? { ...r, ...patch } : r,
                          ),
                        })
                      }
                    />
                  )}
                  {a?.kind === "image" &&
                    (isVideoMode(canvas.composerMode)
                      ? (["reference", "first-frame", "last-frame"] as const)
                      : (["reference", "edit"] as const)
                    ).map((role) => (
                      <button
                        key={role}
                        type="button"
                        aria-pressed={ref.role === role}
                        onClick={() => purpose(ref, role)}
                      >
                        {role === "edit" ? t("修改这张") : t(roleLabels[role])}
                      </button>
                    ))}
                  <button
                    type="button"
                    disabled={task?.inputs[0]?.key === ref.key}
                    onClick={() =>
                      reorder(
                        ref.key,
                        task!.inputs[
                          task!.inputs.findIndex((r) => r.key === ref.key) - 1
                        ].key,
                      )
                    }
                  >
                    {t("向前移")}
                  </button>
                </Popover.Content>
              </Popover.Root>
            </div>
          );
        })}
        {canvas.composerMode !== "agent" &&
          !props.extrasOnly &&
          inputs.length < 12 && <ComposerReference {...props} />}
      </div>
      <Dialog.Root
        open={!!preview}
        onOpenChange={(open) => {
          if (!open) setPreview(undefined);
        }}
      >
        <Dialog.Content
          className={`media-preview-dialog${asset && ["image", "video"].includes(asset.kind) && !asset.missing ? " visual-preview-dialog" : ""}`}
          aria-describedby={undefined}
        >
          <header>
            <Dialog.Title>
              {asset?.name ??
                project.nodes.find((n) => n.id === preview?.nodeId)?.title ??
                t("引用预览")}
            </Dialog.Title>
            <Dialog.Close>
              <button aria-label={t("关闭引用预览")}>
                <X />
              </button>
            </Dialog.Close>
          </header>
          <div className="media-preview-stage">
            {asset?.kind === "image" ? (
              <ZoomableImage
                key={asset.id}
                src={mediaUrl(asset.path)}
                alt={asset.name}
              />
            ) : asset?.kind === "video" ? (
              <video src={mediaUrl(asset.path)} controls />
            ) : (
              <p style={{ whiteSpace: "pre-wrap" }}>{preview?.purpose}</p>
            )}
          </div>
        </Dialog.Content>
      </Dialog.Root>
    </>
  );
}
