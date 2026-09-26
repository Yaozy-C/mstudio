import { stretchClip } from "./stretchClip";
import { detachAudio } from "./detachAudio";
import { ObjectMenu } from "../ui/ObjectMenu";
import { useRef } from "react";
import { duration, type Asset, type Project } from "../model";
import { mediaUrl } from "../bridge";
import { collectAsset, isLibraryAsset } from "../workspace/assetLibrary";
import { MissingAsset } from "../workspace/MissingAsset";
import type { IndexedClip } from "./geometry";
export function TimelineClip({
  entry,
  asset,
  zoom,
  fps,
  selected,
  audio,
  onOpen,
  onReference,
  onRemove,
  onSelect,
  onChange,
}: {
  entry: IndexedClip;
  asset?: Asset;
  zoom: number;
  fps: number;
  selected: boolean;
  audio: boolean;
  onOpen: () => void;
  onReference: () => void;
  onRemove: () => void;
  onSelect: () => void;
  onChange: (f: (p: Project) => Project) => void;
}) {
  const { clip, start } = entry;
  const drag = useRef<{ x: number; delta: number } | null>(null);
  const root = useRef<HTMLDivElement>(null);
  function trim(delta: number, side: "left" | "right") {
    if (asset?.kind === "video" && !audio)
      return stretchClip(clip, delta, side, fps);
    const frame = clip.speed / fps;
    if (side === "left") {
      const d = Math.max(
        -start,
        -clip.trimIn / clip.speed,
        Math.min(delta, duration(clip) - 1 / fps),
      );
      return {
        ...clip,
        start: start + d,
        trimIn: clip.trimIn + d * clip.speed,
      };
    }
    return {
      ...clip,
      trimOut: Math.max(
        clip.trimIn + frame,
        Math.min(
          asset?.kind === "image" ? 3600 : (asset?.duration ?? clip.trimOut),
          clip.trimOut + delta * clip.speed,
        ),
      ),
    };
  }
  return (
    <ObjectMenu
      actions={[
        { label: "编辑片段", run: onOpen },
        ...(asset?.kind === "video" && asset.hasAudio && !audio
          ? [
              {
                label: "分离音频",
                run: () => onChange((p) => detachAudio(p, clip.id)),
              },
            ]
          : []),
        { label: "引用到对话", run: onReference },
        ...(asset
          ? [
              {
                label: isLibraryAsset(asset)
                  ? "已在项目素材中"
                  : "保存为项目素材",
                disabled: isLibraryAsset(asset),
                run: () => onChange((p) => collectAsset(p, asset)),
              },
            ]
          : []),
        {
          label: "移除片段",
          danger: true,
          run: onRemove,
        },
      ]}
    >
      <div
        ref={root}
        role="button"
        tabIndex={0}
        draggable
        aria-label={`${asset?.name ?? "片段"} ${start.toFixed(2)}秒`}
        className={`timeline-clip ${audio ? "sound-clip" : ""} ${selected ? "selected" : ""}`}
        style={{
          left: start * zoom,
          width: Math.max(4, duration(clip) * zoom - 2),
        }}
        title="双击编辑片段 · 右键更多操作"
        onContextMenu={onSelect}
        onDoubleClick={onOpen}
        onClick={onSelect}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            e.stopPropagation();
            onOpen();
          }
        }}
        onDragStart={(e) => {
          if (drag.current) {
            e.preventDefault();
            return;
          }
          e.dataTransfer.effectAllowed = "move";
          e.dataTransfer.setData(
            "application/mstudio-clip",
            JSON.stringify({
              id: clip.id,
              offset:
                (e.clientX - e.currentTarget.getBoundingClientRect().left) /
                zoom,
            }),
          );
        }}
      >
        {asset?.missing && <MissingAsset asset={asset} compact />}
        {!asset?.missing && !audio && asset?.preview && (
          <img draggable={false} loading="lazy" src={mediaUrl(asset.preview)} />
        )}
        <span>{asset?.name || "素材丢失"}</span>
        <small>
          {duration(clip).toFixed(2)}s
          {clip.speed !== 1 ? ` · ${Number(clip.speed.toFixed(2))}×` : ""}
        </small>
        {(["left", "right"] as const).map((side) => (
          <span
            key={side}
            className={`trim-handle ${side}`}
            title={
              asset?.kind === "video" && !audio
                ? "拖动调整速度 · 保留片段内容（0.25–4×）"
                : side === "left"
                  ? "拖动入点"
                  : "拖动出点"
            }
            onDoubleClick={(e) => e.stopPropagation()}
            onClick={(e) => e.stopPropagation()}
            onPointerDown={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onSelect();
              drag.current = { x: e.clientX, delta: 0 };
              e.currentTarget.setPointerCapture(e.pointerId);
            }}
            onPointerMove={(e) => {
              if (!drag.current || !root.current) return;
              drag.current.delta =
                Math.round(((e.clientX - drag.current.x) / zoom) * fps) / fps;
              const c = trim(drag.current.delta, side);
              root.current.style.left = `${(c.start ?? 0) * zoom}px`;
              root.current.style.width = `${Math.max(4, duration(c) * zoom - 2)}px`;
            }}
            onPointerUp={(e) => {
              if (!drag.current) return;
              const c = trim(drag.current.delta, side);
              drag.current = null;
              e.currentTarget.releasePointerCapture(e.pointerId);
              onChange((p) => ({
                ...p,
                clips: p.clips.map((v) => (v.id === clip.id ? c : v)),
              }));
            }}
            onPointerCancel={() => {
              drag.current = null;
              if (root.current) {
                root.current.style.left = `${start * zoom}px`;
                root.current.style.width = `${Math.max(4, duration(clip) * zoom - 2)}px`;
              }
            }}
          />
        ))}
      </div>
    </ObjectMenu>
  );
}
