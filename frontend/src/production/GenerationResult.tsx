import { ZoomableImage } from "../workspace/ZoomableImage";
import { t, useLanguage } from "../i18n";
import { useState } from "react";
import { Dialog } from "@radix-ui/themes";
import { X, At, Crosshair, MagnifyingGlass } from "@phosphor-icons/react";
import { mediaUrl } from "../bridge";
import { MissingAsset } from "../workspace/MissingAsset";
import type { Project } from "../model";
import type { ProductionTask } from "./types";
import { requestCreativeTask } from "../creative/aiTasks";

export function GenerationResult({
  task,
  project,
  reference,
  locate,
}: {
  task: ProductionTask;
  project: Project;
  reference?: (id: string) => void;
  locate?: () => void;
}) {
  useLanguage();
  const [preview, setPreview] = useState(false);
  const asset = project.assets.find((a) => a.id === task.resultAssetId);
  if (!asset) return null;
  if (asset.missing)
    return (
      <div className="generation-result">
        <MissingAsset asset={asset} />
      </div>
    );
  return (
    <div
      className="generation-result"
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData(
          "application/x-mstudio-reference",
          JSON.stringify({ kind: "asset", id: asset.id }),
        );
        e.dataTransfer.effectAllowed = "copy";
      }}
    >
      <div className="generation-result-tools">
        {locate && (
          <button
            className="result-reference result-locate"
            aria-label={t("在画布查看此结果")}
            title={t("在画布查看")}
            onClick={locate}
          >
            <Crosshair size={16} />
          </button>
        )}
        <button
          className="result-reference result-inspect"
          aria-label={t("让媒体制作检查此结果")}
          title={t("检查此结果")}
          onClick={() =>
            requestCreativeTask({
              agentId: "production",
              refs: [{ kind: "asset", id: asset.id }],
              text: t(
                "请检查这个生成结果。先读取实际画面，对照本次提交的提示词和每个参考素材的用途，说明哪些结论有真实帧支持，哪些还需要播放或听声音才能确认；发现的问题给出最小修复方向，不要只凭任务状态下结论。",
              ),
            })
          }
        >
          <MagnifyingGlass size={16} />
        </button>
        {reference && (
          <button
            className="result-reference"
            aria-label={t("引用此结果")}
            title={t("引用到输入框")}
            onClick={() => reference(asset.id)}
          >
            <At size={16} />
          </button>
        )}
      </div>
      {asset.kind === "image" ? (
        <button
          type="button"
          className="generation-result-image"
          aria-label={t("预览生成图片 {v0}", { v0: asset.name })}
          onClick={() => setPreview(true)}
        >
          <img src={mediaUrl(asset.preview || asset.path)} alt={asset.name} />
        </button>
      ) : (
        <video
          aria-label={asset.name}
          src={mediaUrl(asset.path)}
          poster={mediaUrl(asset.preview)}
          controls
          preload="metadata"
        />
      )}
      <Dialog.Root open={preview} onOpenChange={setPreview}>
        <Dialog.Content
          className={`media-preview-dialog${asset && ["image", "video"].includes(asset.kind) && !asset.missing ? " visual-preview-dialog" : ""}`}
          aria-describedby={undefined}
        >
          <header>
            <Dialog.Title>{asset.name}</Dialog.Title>
            <Dialog.Close>
              <button aria-label={t("关闭结果预览")}>
                <X />
              </button>
            </Dialog.Close>
          </header>
          <div className="media-preview-stage">
            {asset.kind === "image" ? (
              <ZoomableImage
                key={asset.id}
                src={mediaUrl(asset.path)}
                alt={asset.name}
              />
            ) : (
              <video src={mediaUrl(asset.path)} controls autoPlay />
            )}
          </div>
        </Dialog.Content>
      </Dialog.Root>
    </div>
  );
}
