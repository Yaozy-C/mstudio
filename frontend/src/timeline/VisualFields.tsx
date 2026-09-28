import { ActionButton } from "../ui/ActionButton";
import { t, useLanguage } from "../i18n";
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
import { GradeFields } from "./GradeFields";
export { defaultVisual } from "./visualSettings";
export function VisualFields({
  clip,
  update,
}: {
  clip: Clip;
  update: (c: Clip) => void;
}) {
  useLanguage();
  const value = { ...defaultVisual, ...clip.visual };
  const patch = (fields: Partial<Visual>) =>
    update({ ...clip, visual: { ...value, ...fields } });
  return (
    <>
      <GradeFields grade={value.grade} change={(grade) => patch({ grade })} />
      <section className="inspector-section">
        <div className="inspector-section-heading">
          <h3>{t("色彩")}</h3>
          <ActionButton
            icon={ArrowCounterClockwise}
            aria-label={t("重置画面效果")}
            disabled={!clip.visual}
            onClick={() => update({ ...clip, visual: undefined })}
          >
            {t("重置")}
          </ActionButton>
        </div>
        {(
          [
            ["brightness", t("亮度"), Sun, -50, 50],
            ["contrast", t("对比度"), CircleHalf, 50, 150],
            ["saturation", t("饱和度"), Drop, 0, 200],
            ["temperature", t("色温"), Thermometer, -100, 100],
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
            endpoints={key === "temperature" ? [t("冷"), t("暖")] : undefined}
            change={(n) => patch({ [key]: n / 100 })}
          />
        ))}
      </section>
      <section className="inspector-section">
        <label className="inspector-select-row">
          <span>
            <Sparkle size={20} />
            {t("效果")}
          </span>
          <select
            aria-label={t("画面效果")}
            value={value.effect}
            onChange={(e) =>
              patch({ effect: e.target.value as Visual["effect"] })
            }
          >
            {[
              ["none", t("无特效")],
              ["grayscale", t("黑白")],
              ["sepia", t("复古")],
              ["blur", t("柔焦")],
              ["vignette", t("暗角")],
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
