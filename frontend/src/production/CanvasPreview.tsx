import { t, useLanguage } from "../i18n";
import { actionText, editShotText } from "../creative/shotText";
import { useState } from "react";
import { Dialog } from "@radix-ui/themes";
import { X } from "@phosphor-icons/react";
import { mediaUrl } from "../bridge";
import { collectAsset, isLibraryAsset } from "../workspace/assetLibrary";
import { MissingAsset } from "../workspace/MissingAsset";
import { DocumentPreview } from "../workspace/DocumentPreview";
import type { Asset, Project } from "../model";
import type { ChangeProject, ProductionItem } from "./types";
export function CanvasPreview({
  item,
  project,
  change,
  close,
  onAdd,
}: {
  item: ProductionItem;
  project: Project;
  change: ChangeProject;
  close: () => void;
  onAdd: (asset: Asset) => void;
}) {
  useLanguage();
  const asset = project.assets.find((a) => a.id === item.assetId);
  const node = project.nodes.find((n) => n.id === item.nodeId);
  const [text, setText] = useState(node ? actionText(node) : item.text);
  const [dialogue, setDialogue] = useState(node?.shot?.dialogue ?? "");
  const save = () =>
    change((p) =>
      node?.shot
        ? editShotText(
            editShotText(p, node.id, "action", text),
            node.id,
            "dialogue",
            dialogue,
          )
        : {
            ...p,
            nodes: p.nodes.map((n) => (n.id === node?.id ? { ...n, text } : n)),
          },
    );
  return (
    <Dialog.Root
      open
      onOpenChange={(v) => {
        if (!v) close();
      }}
    >
      <Dialog.Content
        className="media-preview-dialog canvas-preview"
        aria-describedby={undefined}
      >
        <header>
          <Dialog.Title>{item.title}</Dialog.Title>
          <Dialog.Close>
            <button aria-label={t("关闭预览")}>
              <X />
            </button>
          </Dialog.Close>
        </header>
        {asset ? (
          <div className="media-preview-stage">
            {asset.missing ? (
              <MissingAsset asset={asset} />
            ) : asset.kind === "image" ? (
              <img src={mediaUrl(asset.path)} alt={asset.name} />
            ) : asset.kind === "video" ? (
              <video src={mediaUrl(asset.path)} controls autoPlay />
            ) : (
              <DocumentPreview asset={asset} />
            )}
          </div>
        ) : (
          <>
            <label>
              {t("镜头画面与动作")}
              <textarea
                value={text}
                onChange={(e) => setText(e.target.value)}
                rows={8}
              />
            </label>
            {node?.shot && (
              <label>
                {t("台词 / 旁白")}
                <textarea
                  value={dialogue}
                  onChange={(e) => setDialogue(e.target.value)}
                  rows={3}
                />
              </label>
            )}
            <button
              className="primary"
              onClick={() => {
                save();
                close();
              }}
            >
              {t("保存镜头内容")}
            </button>
          </>
        )}
        {asset && (
          <footer>
            {(asset.kind === "image" ||
              asset.kind === "video" ||
              asset.kind === "audio") && (
              <button
                disabled={!!asset.missing}
                onClick={() => {
                  onAdd(asset);
                  close();
                }}
              >
                {t("加入时间线")}
              </button>
            )}
            <button
              disabled={isLibraryAsset(asset)}
              onClick={() => change((p) => collectAsset(p, asset))}
            >
              {isLibraryAsset(asset)
                ? t("已在项目素材中")
                : t("保存为项目素材")}
            </button>
          </footer>
        )}
      </Dialog.Content>
    </Dialog.Root>
  );
}
