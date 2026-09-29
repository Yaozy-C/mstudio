import { t, useLanguage } from "../i18n";
import { ShotCardText } from "./ShotCardText";
import { memo, useLayoutEffect, useRef } from "react";
import {
  Play,
  Image,
  VideoCamera,
  FileText,
  CheckCircle,
} from "@phosphor-icons/react";
import { ObjectMenu } from "../ui/ObjectMenu";
import { mediaUrl } from "../bridge";
import { isLibraryAsset } from "../workspace/assetLibrary";
import { MissingAsset } from "../workspace/MissingAsset";
import type { Asset, Project } from "../model";
import type { ProductionItem } from "./types";
import type { ProductionController } from "./useProduction";
import "./cards.css";
export const labels = {
  script: "镜头说明",
  reference: "图片",
  image: "图片",
  video: "视频",
  note: "文字资料",
};
export const CanvasCard = memo(function CanvasCard({
  item,
  asset,
  compact,
  inTimeline = false,
  project,
  canvas,
  preview,
  collect,
  onAdd,
}: {
  item: ProductionItem;
  asset?: Asset;
  compact: boolean;
  inTimeline?: boolean;
  project: Project;
  canvas: ProductionController;
  preview: (item: ProductionItem) => void;
  collect: (asset: Asset) => void;
  onAdd: (asset: Asset) => void;
}) {
  useLanguage();
  const mediaTitle =
    asset &&
    (/^画面\s*\d+$/.test(item.title) ||
      /^生成结果-|^[a-f0-9]{20,}/i.test(item.title))
      ? asset.kind === "video"
        ? t("视频")
        : t("图片")
      : item.title;
  const Icon =
    asset?.kind === "video"
      ? VideoCamera
      : asset?.kind === "image"
        ? Image
        : FileText;
  const cardRef = useRef<HTMLElement>(null);
  useLayoutEffect(() => {
    if (compact || item.kind !== "script" || !cardRef.current) return;
    const element = cardRef.current;
    const report = () => canvas.measure?.(item.key, element.offsetHeight);
    const observer = new ResizeObserver(report);
    observer.observe(element);
    report();
    return () => observer.disconnect();
  }, [item.key, item.kind, compact, canvas.measure]);
  const dragging = useRef(false);
  const selected = canvas.selected.includes(item.key);
  const referenced = canvas.referenced.has(item.key);
  const actions = [
    { label: t("预览 / 编辑"), run: () => preview(item) },
    { label: t("引用到对话"), run: () => canvas.reference([item.key]) },
    ...(asset &&
    (asset.kind === "image" || asset.kind === "video" || asset.kind === "audio")
      ? [
          {
            label: t("加入时间线"),
            run: () => onAdd(asset),
            disabled: !!asset.missing,
          },
        ]
      : []),
    ...(asset
      ? [
          {
            label: isLibraryAsset(asset)
              ? t("已在项目素材中")
              : t("保存为项目素材"),
            run: () => collect(asset),
            disabled: isLibraryAsset(asset),
          },
        ]
      : []),
    { label: t("制作视频"), run: () => canvas.act("video", [item.key]) },
    { label: t("从画布移除"), run: () => canvas.remove(item) },
  ];
  return (
    <ObjectMenu actions={actions}>
      <article
        ref={cardRef}
        className={`canvas-card ${selected ? "selected" : ""} ${referenced ? "referenced" : ""} ${item.kind}`}
        style={{
          left: item.x,
          top: item.y,
          width: item.width,
          height: item.kind === "script" && !compact ? "auto" : item.height,
        }}
        data-card={item.key}
        draggable
        onDragStart={(e) => {
          const ref = item.assetId
            ? { kind: "asset", id: item.assetId }
            : { kind: "node", id: item.nodeId };
          e.dataTransfer.setData(
            "application/x-mstudio-reference",
            JSON.stringify(ref),
          );
          e.dataTransfer.effectAllowed = "copy";
        }}
        tabIndex={0}
        aria-label={`${labels[item.kind]}：${mediaTitle}`}
        aria-description={
          [
            selected && t("已选中"),
            referenced && t("已引用到聊天"),
            inTimeline && t("已加入时间线"),
          ]
            .filter(Boolean)
            .join("，") || undefined
        }
        onClick={(e) => {
          if (!dragging.current)
            canvas.select(item.key, e.shiftKey || e.metaKey || e.ctrlKey);
        }}
        onKeyDown={(e) => {
          if (e.target !== e.currentTarget) return;
          if (e.key === "Enter") {
            e.preventDefault();
            canvas.select(item.key, e.shiftKey);
          }
        }}
        onDoubleClick={() => preview(item)}
      >
        <strong
          className="card-title"
          title={item.title}
          onPointerDown={(e) => {
            if (e.button !== 0) return;
            e.stopPropagation();
            e.preventDefault();
            const el = e.currentTarget,
              card = el.parentElement!,
              world = el.closest(".canvas-world")!;
            const scale = Number(world.getAttribute("data-scale") ?? 1),
              sx = e.clientX,
              sy = e.clientY;
            dragging.current = false;
            el.setPointerCapture(e.pointerId);
            const drag = (p: PointerEvent) => {
              const dx = (p.clientX - sx) / scale,
                dy = (p.clientY - sy) / scale;
              dragging.current = Math.abs(dx) + Math.abs(dy) > 3;
              card.style.transform = `translate(${dx}px,${dy}px)`;
            };
            const end = (p: PointerEvent) => {
              el.removeEventListener("pointermove", drag);
              el.removeEventListener("pointerup", end);
              el.removeEventListener("pointercancel", cancel);
              card.style.transform = "";
              if (dragging.current)
                canvas.move(
                  item.key,
                  item.x + (p.clientX - sx) / scale,
                  item.y + (p.clientY - sy) / scale,
                );
              setTimeout(() => {
                dragging.current = false;
              }, 0);
            };
            const cancel = () => {
              el.removeEventListener("pointermove", drag);
              el.removeEventListener("pointerup", end);
              card.style.transform = "";
            };
            el.addEventListener("pointermove", drag);
            el.addEventListener("pointerup", end, { once: true });
            el.addEventListener("pointercancel", cancel, { once: true });
          }}
        >
          {item.kind !== "script" && <Icon size={16} />}
          <span>{item.kind === "script" ? item.title : mediaTitle}</span>
        </strong>
        {!compact && !!item.usages?.length && (
          <small className="card-usage">
            {t("用于：")}
            {item.usages.map((u) => u.title).join(" · ")}
          </small>
        )}
        {compact ? (
          <div className="card-overview">
            <Icon size={44} />
          </div>
        ) : item.kind === "script" ? (
          <ShotCardText item={item} project={project} />
        ) : !asset ? (
          <p className="card-copy">{item.text || t("双击补充镜头内容")}</p>
        ) : asset.missing ? (
          <MissingAsset asset={asset} />
        ) : asset.kind === "video" ? (
          <div className="card-video">
            <img
              src={mediaUrl(asset.preview)}
              alt=""
              draggable={false}
              loading="lazy"
              decoding="async"
            />
            <button
              aria-label={t("播放 {v0}", { v0: item.title })}
              onClick={(e) => {
                e.stopPropagation();
                preview(item);
              }}
            >
              <Play weight="fill" />
            </button>
          </div>
        ) : asset.kind === "image" ? (
          <img
            loading="lazy"
            decoding="async"
            src={mediaUrl(asset.preview || asset.path)}
            alt={item.title}
            draggable={false}
          />
        ) : (
          <p className="card-copy">{item.text || asset.name}</p>
        )}
        {asset && inTimeline && (
          <span
            className="card-timeline-usage"
            role="img"
            aria-label={t("已加入时间线")}
            title={t("已加入时间线")}
          >
            <CheckCircle size={20} weight="fill" aria-hidden="true" />
          </span>
        )}
      </article>
    </ObjectMenu>
  );
});
