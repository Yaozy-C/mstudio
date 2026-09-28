import { useEffect, useId, useState } from "react";
import { duration, type Clip } from "../model";
import { t, useLanguage } from "../i18n";
import { InspectorControl } from "./InspectorControl";
import { speedForDuration, targetDurationRange } from "./targetDuration";

export function ClipSpeedControl({
  clip,
  change,
}: {
  clip: Clip;
  change: (speed: number) => void;
}) {
  useLanguage();
  const [mode, setMode] = useState("speed");
  const seconds = duration(clip);
  const [draft, setDraft] = useState(seconds.toFixed(1));
  const [dirty, setDirty] = useState(false);
  const [invalid, setInvalid] = useState(false);
  const id = useId();
  const { min, max } = targetDurationRange(clip);
  useEffect(() => {
    setDraft(seconds.toFixed(1));
    setDirty(false);
    setInvalid(false);
  }, [seconds]);
  const commit = () => {
    if (!dirty) return;
    const speed = speedForDuration(clip, draft.trim() ? Number(draft) : NaN);
    setInvalid(speed === null);
    if (speed !== null) {
      setDirty(false);
      if (speed !== clip.speed) change(speed);
    }
  };
  return (
    <>
      <label className="inspector-select-row">
        <span>{t("变速方式")}</span>
        <select value={mode} onChange={(e) => setMode(e.target.value)}>
          <option value="speed">{t("播放倍率")}</option>
          <option value="duration">{t("目标时长")}</option>
        </select>
      </label>
      {mode === "speed" ? (
        <InspectorControl
          label={t("播放速度")}
          value={clip.speed}
          unit="×"
          min={0.25}
          max={4}
          step={0.05}
          change={change}
        />
      ) : (
        <div className="inspector-control">
          <div className="inspector-control-row">
            <label htmlFor={id}>{t("目标时长")}</label>
            <div className="inspector-number">
              <input
                id={id}
                type="number"
                min={min}
                max={max}
                step={0.1}
                disabled={max < min}
                value={draft}
                aria-invalid={invalid}
                aria-describedby={`${id}-hint`}
                onChange={(e) => {
                  setDraft(e.target.value);
                  setDirty(true);
                  setInvalid(false);
                }}
                onBlur={commit}
                onKeyDown={(e) => {
                  if (e.key === "Enter") e.currentTarget.blur();
                  if (e.key === "Escape") {
                    setDraft(seconds.toFixed(1));
                    setDirty(false);
                    setInvalid(false);
                  }
                }}
              />
              <span>s</span>
            </div>
          </div>
          <p
            className="retime-hint"
            id={`${id}-hint`}
            role={invalid ? "alert" : undefined}
          >
            {max < min
              ? t("片段过短，无法按 0.1 秒设置目标时长")
              : t("输入 {v0}–{v1} 秒，精确到 0.1 秒；回车或离开输入框应用", {
                  v0: min.toFixed(1),
                  v1: max.toFixed(1),
                })}
          </p>
          <div className="inspector-duration">
            <span>{t("播放速度")}</span>
            <output>
              {Number(clip.speed.toFixed(3))}
              <small> ×</small>
            </output>
          </div>
        </div>
      )}
    </>
  );
}
