import { duration, type Clip } from "../model";
export function ClipFields({
  clip,
  assetDuration,
  image,
  onUpdate,
}: {
  clip: Clip;
  assetDuration: number;
  image: boolean;
  onUpdate: (clip: Clip) => void;
}) {
  function numeric(
    key: "trimIn" | "trimOut" | "speed" | "volume",
    value: number,
  ) {
    if (!Number.isFinite(value)) return;
    const next = { ...clip, [key]: value };
    if (
      next.trimIn < 0 ||
      next.trimOut <= next.trimIn ||
      (!image && next.trimOut > assetDuration) ||
      next.speed < 0.25 ||
      next.speed > 4 ||
      next.volume < 0 ||
      next.volume > 1
    )
      return;
    onUpdate(next);
  }
  return (
    <>
      <div className="field-grid">
        <label>
          入点 / 秒
          <input
            type="number"
            min={0}
            step={0.1}
            value={clip.trimIn}
            onChange={(e) => numeric("trimIn", e.target.valueAsNumber)}
          />
        </label>
        <label>
          出点 / 秒
          <input
            type="number"
            min={0.1}
            max={image ? undefined : assetDuration}
            step={0.1}
            value={clip.trimOut}
            onChange={(e) => numeric("trimOut", e.target.valueAsNumber)}
          />
        </label>
      </div>
      <label>
        播放速度
        <select
          value={clip.speed}
          onChange={(e) => numeric("speed", +e.target.value)}
        >
          {[0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 3, 4].map((v) => (
            <option key={v} value={v}>
              {v}× {v === 1 ? "原速" : ""}
            </option>
          ))}
        </select>
      </label>
      <label>
        音量 <span>{Math.round(clip.volume * 100)}%</span>
        <input
          type="range"
          min={0}
          max={1}
          step={0.05}
          value={clip.volume}
          onChange={(e) => numeric("volume", +e.target.value)}
        />
      </label>
      <p className="subtle">
        片段时长 {duration(clip).toFixed(2)} 秒 · 原素材不受影响
      </p>
    </>
  );
}
