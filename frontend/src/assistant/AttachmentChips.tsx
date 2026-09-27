import { t, useLanguage } from "../i18n";
import {
  FileText,
  Image,
  FilmStrip,
  MusicNote,
  X,
} from "@phosphor-icons/react";
import { mediaUrl } from "../bridge";
import { useState } from "react";
import { MediaPreview } from "../workspace/MediaPreview";
import type { Project } from "../model";
import { attachmentKey, type Attachment } from "./attachments";
export function AttachmentChips({
  items,
  project,
  remove,
}: {
  items: Attachment[];
  project: Project;
  remove?: (item: Attachment) => void;
}) {
  useLanguage();
  const [previewId, setPreviewId] = useState<string | null>(null);
  if (!items.length) return null;
  return (
    <div
      className="attachment-chips"
      aria-label={remove ? t("本轮附件") : t("消息附件")}
    >
      {items.map((a) => {
        const asset = project.assets.find((v) => v.id === a.assetId);
        const Icon =
          a.mediaKind === "video"
            ? FilmStrip
            : a.mediaKind === "image"
              ? Image
              : a.mediaKind === "audio"
                ? MusicNote
                : FileText;
        const kind =
          a.kind === "clip"
            ? t("时间线片段 · 修改范围")
            : a.mediaKind === "video"
              ? t("视频")
              : a.mediaKind === "image"
                ? t("图片")
                : a.mediaKind === "audio"
                  ? t("音频")
                  : a.mediaKind === "text"
                    ? t("文本")
                    : a.mediaKind === "document"
                      ? "PDF"
                      : t("文字 / 镜头");
        return (
          <div className="attachment-chip" key={attachmentKey(a)}>
            <button
              className="attachment-preview"
              type="button"
              disabled={!asset}
              aria-label={t("预览附件 {v0}", { v0: a.title })}
              onClick={() => asset && setPreviewId(asset.id)}
            >
              {asset?.preview ? (
                <img src={mediaUrl(asset.preview)} alt="" />
              ) : (
                <Icon size={20} />
              )}
              <span>
                <strong title={a.title}>{a.title}</strong>
                <small>{kind}</small>
              </span>
            </button>
            {remove && (
              <button
                type="button"
                aria-label={t("移除附件 {v0}", { v0: a.title })}
                onClick={() => remove(a)}
              >
                <X size={14} />
              </button>
            )}
          </div>
        );
      })}
      <MediaPreview
        asset={project.assets.find((a) => a.id === previewId)}
        close={() => setPreviewId(null)}
      />
    </div>
  );
}
