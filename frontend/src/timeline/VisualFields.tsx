import {
  ArrowCounterClockwise,
  Sun,
  CircleHalf,
  Drop,
  Thermometer,
  Sparkle,
} from "@phosphor-icons/react";
import type { Clip, Visual } from "../model";
import { defaultVisual } from "./visualSettings";
import { InspectorControl } from "./InspectorControl";
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
    <>
      <section className="inspector-section">
        <div className="inspector-section-heading">
          <h3>色彩</h3>
          <button
            className="inspector-action"
            aria-label="重置画面效果"
            disabled={!clip.visual}
            onClick={() => update({ ...clip, visual: undefined })}
          >
            <ArrowCounterClockwise size={20} />
            重置
          </button>
        </div>
        {(
          [
            ["brightness", "亮度", Sun, -50, 50],
            ["contrast", "对比度", CircleHalf, 50, 150],
            ["saturation", "饱和度", Drop, 0, 200],
            ["temperature", "色温", Thermometer, -100, 100],
          ] as const
        ).map(([key, label, icon, min, max]) => (
          <InspectorControl
            key={key}
            label={label}
            icon={icon}
            value={value[key] * 100}
            unit={key === "temperature" ? "" : "%"}
            min={min}
            max={max}
            slider
            endpoints={key === "temperature" ? ["冷", "暖"] : undefined}
            change={(n) => patch({ [key]: n / 100 })}
          />
        ))}
      </section>
      <section className="inspector-section">
        <label className="inspector-select-row">
          <span>
            <Sparkle size={20} />
            效果
          </span>
          <select
            aria-label="画面效果"
            value={value.effect}
            onChange={(e) =>
              patch({ effect: e.target.value as Visual["effect"] })
            }
          >
            {[
              ["none", "无特效"],
              ["grayscale", "黑白"],
              ["sepia", "复古"],
              ["blur", "柔焦"],
              ["vignette", "暗角"],
            ].map(([v, label]) => (
              <option key={v} value={v}>
                {label}
              </option>
            ))}
          </select>
        </label>
      </section>
    </>
  );
}
