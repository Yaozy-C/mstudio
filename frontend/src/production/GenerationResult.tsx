import { useState } from "react";
import { Dialog } from "@radix-ui/themes";
import { X, At, Crosshair } from "@phosphor-icons/react";
import { mediaUrl } from "../bridge";
import { MissingAsset } from "../workspace/MissingAsset";
import type { Project } from "../model";
import type { ProductionTask } from "./types";

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
      {locate && (
        <button
          className="result-reference result-locate"
          aria-label="在画布查看此结果"
          title="在画布查看"
          onClick={locate}
        >
          <Crosshair size={16} />
        </button>
      )}
      {reference && (
        <button
          className="result-reference"
          aria-label="引用此结果"
          title="引用到输入框"
          onClick={() => reference(asset.id)}
        >
          <At size={16} />
        </button>
      )}
      {asset.kind === "image" ? (
        <button
          type="button"
          className="generation-result-image"
          aria-label={`预览生成图片 ${asset.name}`}
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
          className="media-preview-dialog"
          aria-describedby={undefined}
        >
          <header>
            <Dialog.Title>{asset.name}</Dialog.Title>
            <Dialog.Close>
              <button aria-label="关闭结果预览">
                <X />
              </button>
            </Dialog.Close>
          </header>
          <div className="media-preview-stage">
            {asset.kind === "image" ? (
              <img src={mediaUrl(asset.path)} alt={asset.name} />
            ) : (
              <video src={mediaUrl(asset.path)} controls autoPlay />
            )}
          </div>
        </Dialog.Content>
      </Dialog.Root>
    </div>
  );
}
