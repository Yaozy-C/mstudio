import "./shot-editor.css";
import { ActionButton } from "../ui/ActionButton";
import { ZoomableImage } from "../workspace/ZoomableImage";
import { t, useLanguage } from "../i18n";
import { actionText, editShotText } from "../creative/shotText";
import { useState } from "react";
import { Dialog } from "@radix-ui/themes";
import { X, FloppyDisk, FilmSlate } from "@phosphor-icons/react";
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
        className={`media-preview-dialog canvas-preview${!asset ? " shot-editor-dialog" : ""}${asset && ["image", "video"].includes(asset.kind) && !asset.missing ? " visual-preview-dialog" : ""}`}
        aria-describedby={undefined}
      >
        <header>
          <Dialog.Title>
            {!asset && <FilmSlate size={22} />}
            {item.title}
          </Dialog.Title>
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
              <ZoomableImage
                key={asset.id}
                src={mediaUrl(asset.path)}
                alt={asset.name}
              />
            ) : asset.kind === "video" ? (
              <video src={mediaUrl(asset.path)} controls autoPlay />
            ) : (
              <DocumentPreview asset={asset} />
            )}
          </div>
        ) : (
          <form
            className="shot-editor-form"
            onSubmit={(e) => {
              e.preventDefault();
              save();
              close();
            }}
          >
            <div className="shot-editor-fields">
              <label>
                {t("镜头画面与动作")}
                <textarea
                  value={text}
                  onChange={(e) => setText(e.target.value)}
                  rows={12}
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
            </div>
            <footer className="shot-editor-actions">
              <ActionButton icon={X} onClick={close}>
                {t("取消")}
              </ActionButton>
              <button className="primary" type="submit">
                <FloppyDisk size={18} />
                {t("保存镜头内容")}
              </button>
            </footer>
          </form>
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
