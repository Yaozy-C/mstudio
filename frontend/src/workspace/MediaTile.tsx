import { t } from "../i18n";
import type { ComponentPropsWithRef } from "react";
import {
  FileText,
  FilmStrip,
  ImageSquare,
  MusicNotes,
} from "@phosphor-icons/react";
import { mediaUrl } from "../bridge";
import { MissingAsset } from "./MissingAsset";
import { formatTime, type Asset } from "../model";
export function MediaTile({
  asset: a,
  scope,
  selected,
  select,
  preview,
  onContextMenu,
  className,
  ...props
}: {
  asset: Asset;
  scope: "project" | "global";
  selected: boolean;
  select: (id: string) => void;
  preview: (id: string) => void;
} & ComponentPropsWithRef<"article">) {
  return (
    <article
      {...props}
      className={["media-item", className].filter(Boolean).join(" ")}
      onContextMenu={(event) => {
        select(a.id);
        onContextMenu?.(event);
      }}
    >
      <button
        className="media-thumbnail"
        draggable={scope === "project" && !a.missing}
        onDragStart={(e) => {
          if (scope !== "project" || a.missing) {
            e.preventDefault();
            return;
          }
          e.dataTransfer.setData(
            "application/x-mstudio-reference",
            JSON.stringify({ kind: "asset", id: a.id }),
          );
          e.dataTransfer.effectAllowed = "copy";
        }}
        onClick={() => select(a.id)}
        onDoubleClick={() => preview(a.id)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            preview(a.id);
          }
        }}
        aria-pressed={selected}
        title={t("双击预览 · 右键更多操作")}
      >
        {a.missing ? (
          <MissingAsset asset={a} compact />
        ) : a.preview ? (
          <img loading="lazy" src={mediaUrl(a.preview)} alt={a.name} />
        ) : a.kind === "audio" ? (
          <MusicNotes size={28} />
        ) : (
          <FileText size={28} />
        )}
        {!a.missing && (
          <span>
            {a.kind === "image"
              ? t("图片")
              : a.kind === "text"
                ? t("文本")
                : a.kind === "document"
                  ? "PDF"
                  : formatTime(a.duration)}
          </span>
        )}
      </button>
      <div className="media-caption">
        {a.kind === "video" ? (
          <FilmStrip />
        ) : a.kind === "image" ? (
          <ImageSquare />
        ) : a.kind === "audio" ? (
          <MusicNotes />
        ) : (
          <FileText />
        )}
        <span title={a.name}>{a.name}</span>
      </div>
    </article>
  );
}
