import {
  ArrowsHorizontal,
  ArrowsVertical,
  ArrowsOutSimple,
  Crosshair,
  Stack,
  Clock,
  Gauge,
  BracketsSquare,
  SpeakerHigh,
} from "@phosphor-icons/react";
import { duration, type Clip, type Asset, type Project } from "../model";
import { validateClip } from "./document";
import { InspectorControl } from "./InspectorControl";
export function ClipInspectorPanels({
  tab,
  clip,
  asset,
  project,
  update,
}: {
  tab: string;
  clip: Clip;
  asset: Asset;
  project: Project;
  update: (clip: Clip) => void;
}) {
  const patch = (fields: Partial<Clip>) => {
    const next = { ...clip, ...fields };
    if (validateClip(next, asset)) update(next);
  };
  if (tab === "visual")
    return (
      <>
        <section className="inspector-section">
          <h3>位置</h3>
          <InspectorControl
            label="水平"
            icon={ArrowsHorizontal}
            value={(clip.x ?? 0.5) * 100}
            unit="%"
            min={-100}
            max={200}
            change={(x) => patch({ x: x / 100 })}
          />
          <InspectorControl
            label="垂直"
            icon={ArrowsVertical}
            value={(clip.y ?? 0.5) * 100}
            unit="%"
            min={-100}
            max={200}
            change={(y) => patch({ y: y / 100 })}
          />
          <button
            className="inspector-action"
            onClick={() => patch({ x: 0.5, y: 0.5 })}
          >
            <Crosshair size={20} />
            居中
          </button>
        </section>
        <section className="inspector-section">
          <h3>大小</h3>
          <InspectorControl
            label="缩放"
            icon={ArrowsOutSimple}
            value={(clip.scale ?? 1) * 100}
            unit="%"
            min={5}
            max={400}
            slider
            change={(scale) => patch({ scale: scale / 100 })}
          />
        </section>
        <section className="inspector-section">
          <h3>合成</h3>
          <InspectorControl
            label="不透明度"
            icon={Stack}
            value={(clip.opacity ?? 1) * 100}
            unit="%"
            min={0}
            max={100}
            slider
            change={(opacity) => patch({ opacity: opacity / 100 })}
          />
        </section>
      </>
    );
  if (tab === "time")
    return (
      <>
        <section className="inspector-section">
          <h3>素材截取</h3>
          <InspectorControl
            label="开始"
            icon={BracketsSquare}
            value={clip.trimIn}
            unit="s"
            min={0}
            max={clip.trimOut - 0.001}
            step={0.001}
            change={(trimIn) => patch({ trimIn })}
          />
          <InspectorControl
            label="结束"
            icon={BracketsSquare}
            value={clip.trimOut}
            unit="s"
            min={clip.trimIn + 0.001}
            max={
              asset.kind === "image"
                ? clip.trimIn + 3600 * clip.speed
                : asset.duration
            }
            step={0.001}
            change={(trimOut) => patch({ trimOut })}
          />
        </section>
        <section className="inspector-section">
          <h3>
            <Gauge size={20} />
            速度
          </h3>
          <InspectorControl
            label="播放速度"
            value={clip.speed}
            unit="×"
            min={0.25}
            max={4}
            step={0.05}
            change={(speed) => patch({ speed })}
          />
          <div className="inspector-duration">
            <span>片段时长</span>
            <output>
              {duration(clip).toFixed(2)}
              <small> s</small>
            </output>
          </div>
        </section>
        <section className="inspector-section">
          <h3>
            <Clock size={20} />
            时间线
          </h3>
          <InspectorControl
            label="放置位置"
            value={clip.start ?? 0}
            unit="s"
            min={0}
            max={86400}
            step={1 / project.fps}
            change={(start) =>
              patch({ start: Math.round(start * project.fps) / project.fps })
            }
          />
          <label className="inspector-select-row">
            <span>
              <Stack size={20} />
              所在轨道
            </span>
            <select
              aria-label="所在轨道"
              value={clip.trackId}
              onChange={(e) => patch({ trackId: e.target.value })}
            >
              {project.tracks
                .filter((t) =>
                  t.kind === "video" ? asset.kind !== "audio" : asset.hasAudio,
                )
                .map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.name}
                  </option>
                ))}
            </select>
          </label>
        </section>
      </>
    );
  return (
    <section className="inspector-section">
      <h3>声音</h3>
      <InspectorControl
        label="音量"
        icon={SpeakerHigh}
        value={clip.volume * 100}
        unit="%"
        min={0}
        max={100}
        slider
        change={(volume) => patch({ volume: volume / 100 })}
      />
      <InspectorControl
        label="淡入"
        value={clip.fadeIn ?? 0}
        unit="s"
        min={0}
        max={duration(clip)}
        step={0.1}
        change={(fadeIn) => patch({ fadeIn })}
      />
      <InspectorControl
        label="淡出"
        value={clip.fadeOut ?? 0}
        unit="s"
        min={0}
        max={duration(clip)}
        step={0.1}
        change={(fadeOut) => patch({ fadeOut })}
      />
    </section>
  );
}
