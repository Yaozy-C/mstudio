import { platformShortcut } from "../ui/platformShortcut";
import { t, useLanguage } from "../i18n";
import { Check } from "@phosphor-icons/react";
import { requestClipEdit } from "./clipEdits";
import { Waveform } from "./Waveform";
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
  projectId,
  asset,
  zoom,
  fps,
  selected,
  audio,
  muted = false,
  hidden = false,
  onOpen,
  onReference,
  onRemove,
  onSelect,
  onChange,
}: {
  entry: IndexedClip;
  projectId: string;
  asset?: Asset;
  zoom: number;
  fps: number;
  selected: boolean;
  audio: boolean;
  muted?: boolean;
  hidden?: boolean;
  onOpen: () => void;
  onReference: () => void;
  onRemove: () => void;
  onSelect: () => void;
  onChange: (f: (p: Project) => Project) => void;
}) {
  useLanguage();
  const { clip, start } = entry;
  const status = [
    !asset || asset.missing ? t("素材丢失") : "",
    hidden ? t("画面已隐藏") : "",
    muted ? t("轨道已静音") : clip.volume === 0 ? t("片段已静音") : "",
  ]
    .filter(Boolean)
    .join(" · ");
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
        { label: t("编辑片段"), run: onOpen },
        {
          label: platformShortcut(t("复制片段 ⌘C")),
          run: () => requestClipEdit("copy", clip.id),
        },
        {
          label: platformShortcut(t("剪切片段 ⌘X")),
          run: () => requestClipEdit("cut", clip.id),
        },
        {
          label: platformShortcut(t("粘贴片段 ⌘V")),
          run: () => requestClipEdit("paste", clip.id),
        },
        ...(asset?.kind === "video" && asset.hasAudio && !audio
          ? [
              {
                label: t("分离音频"),
                run: () => onChange((p) => detachAudio(p, clip.id)),
              },
            ]
          : []),
        { label: t("引用到对话"), run: onReference },
        ...(asset
          ? [
              {
                label: isLibraryAsset(asset)
                  ? t("已在项目素材中")
                  : t("保存为项目素材"),
                disabled: isLibraryAsset(asset),
                run: () => onChange((p) => collectAsset(p, asset)),
              },
            ]
          : []),
        {
          label: t("移除片段 Delete / Backspace"),
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
        aria-label={t("{v0} {v1}秒{v2}", {
          v0: asset?.name ?? t("片段"),
          v1: start.toFixed(2),
          v2: status ? ` · ${status}` : "",
        })}
        aria-pressed={selected}
        data-muted={muted || clip.volume === 0}
        data-hidden={hidden}
        data-compact={duration(clip) * zoom < 96}
        data-preview={!!asset?.preview && !audio}
        data-missing={!asset || asset.missing}
        className={`timeline-clip ${audio ? "sound-clip" : ""} ${selected ? "selected" : ""}`}
        style={{
          left: start * zoom,
          width: Math.max(4, duration(clip) * zoom - 2),
        }}
        title={t("{v0}{v1} · 双击编辑片段 · 右键更多操作", {
          v0: asset?.name ?? t("素材丢失"),
          v1: status ? ` · ${status}` : "",
        })}
        onContextMenu={onSelect}
        onDoubleClick={onOpen}
        onClick={onSelect}
        onKeyDown={(e) => {
          if (e.key === " " && !selected) {
            e.preventDefault();
            e.stopPropagation();
            onSelect();
          }
          if (e.key === "Enter") {
            e.preventDefault();
            e.stopPropagation();
            onSelect();
            onOpen();
          }
        }}
        onDragStart={(e) => {
          if (drag.current) {
            e.preventDefault();
            return;
          }
          onSelect();
          e.currentTarget.dataset.interaction = "dragging";
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
        onDragEnd={(e) => delete e.currentTarget.dataset.interaction}
      >
        {asset?.missing && <MissingAsset asset={asset} compact />}
        {!asset?.missing && !audio && asset?.preview && (
          <img draggable={false} loading="lazy" src={mediaUrl(asset.preview)} />
        )}
        <span className="clip-title">
          {selected && (
            <Check
              className="clip-selection-mark"
              weight="bold"
              aria-hidden="true"
            />
          )}
          {asset?.name || t("素材丢失")}
        </span>
        <small>
          {status && `${status} · `}
          {duration(clip).toFixed(2)}s
          {clip.speed !== 1 ? ` · ${Number(clip.speed.toFixed(2))}×` : ""}
        </small>
        {audio && asset?.hasAudio && !asset.missing && (
          <Waveform asset={asset} clip={clip} projectId={projectId} />
        )}
        {(["left", "right"] as const).map((side) => (
          <span
            key={side}
            className={`trim-handle ${side}`}
            title={
              asset?.kind === "video" && !audio
                ? t("拖动调整速度 · 保留片段内容（0.25–4×）")
                : side === "left"
                  ? t("拖动入点")
                  : t("拖动出点")
            }
            onDoubleClick={(e) => e.stopPropagation()}
            onClick={(e) => e.stopPropagation()}
            onPointerDown={(e) => {
              if (e.button !== 0) return;
              e.preventDefault();
              e.stopPropagation();
              if (root.current) root.current.dataset.interaction = "trimming";
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
              if (root.current) delete root.current.dataset.interaction;
              e.currentTarget.releasePointerCapture(e.pointerId);
              onChange((p) => ({
                ...p,
                clips: p.clips.map((v) => (v.id === clip.id ? c : v)),
              }));
            }}
            onPointerCancel={() => {
              drag.current = null;
              if (root.current) delete root.current.dataset.interaction;
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
