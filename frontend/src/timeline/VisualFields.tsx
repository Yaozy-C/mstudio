import type { Clip, Visual } from "../model";
import { defaultVisual } from "./visualSettings";
export { defaultVisual } from "./visualSettings";
export function VisualFields({
  clip,
  update,
}: {
  clip: Clip;
  update: (c: Clip) => void;
}) {
  const value = { ...defaultVisual, ...clip.visual };
  const patch = (fields: Partial<Visual>) =>
    update({ ...clip, visual: { ...value, ...fields } });
  return (
    <section className="inspector-section">
      <div className="inline">
        <h3>手动微调</h3>
        <button onClick={() => update({ ...clip, visual: undefined })}>
          重置画面效果
        </button>
      </div>
      <label>
        特效
        <select
          value={value.effect}
          onChange={(e) =>
            patch({ effect: e.target.value as Visual["effect"] })
          }
        >
          <option value="none">无特效</option>
          <option value="grayscale">黑白</option>
          <option value="sepia">复古</option>
          <option value="blur">柔焦</option>
          <option value="vignette">暗角</option>
        </select>
      </label>
      {(
        [
          ["brightness", "亮度", -0.5, 0.5],
          ["contrast", "对比度", 0.5, 1.5],
          ["saturation", "饱和度", 0, 2],
          ["temperature", "色温", -1, 1],
        ] as const
      ).map(([key, label, min, max]) => (
        <label key={key}>
          {label} · {Math.round(value[key] * 100)}
          {key === "temperature" ? "（负值偏冷，正值偏暖）" : "%"}
          <input
            aria-label={label}
            type="range"
            min={min}
            max={max}
            step={0.01}
            value={value[key]}
            onChange={(e) => patch({ [key]: +e.target.value })}
          />
        </label>
      ))}
    </section>
  );
}
