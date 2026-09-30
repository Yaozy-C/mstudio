import { StudioSelect } from "../ui/StudioSelect";
import { useEffect, useState } from "react";
import type { Caption } from "../model";
import { t, useLanguage } from "../i18n";
import { captionAnimations } from "./captionAnimation";
import { captionImage } from "./captions";
export function CaptionAnimationSelect({
  caption,
  patch,
  mixed = false,
}: {
  caption: Caption;
  mixed?: boolean;
  patch: (v: Partial<Caption>) => void;
}) {
  useLanguage();
  return (
    <label>
      {t("字幕动画")}
      <StudioSelect
        label={t("字幕动画")}
        value={mixed ? "mixed" : (caption.animation ?? "none")}
        onValueChange={(value) =>
          patch({ animation: value as Caption["animation"] })
        }
        options={[
          ...(mixed
            ? [{ value: "mixed", label: t("多种动画"), disabled: true }]
            : []),
          ...Object.entries(captionAnimations).map(([value, name]) => ({
            value,
            label: t(name),
          })),
        ]}
      />
    </label>
  );
}
export function CaptionAnimationFields({
  caption,
  width,
  height,
  patch,
}: {
  caption: Caption;
  width: number;
  height: number;
  patch: (v: Partial<Caption>) => void;
}) {
  useLanguage();
  const [time, setTime] = useState(0),
    [playing, setPlaying] = useState(false);
  const duration = caption.end - caption.start;
  useEffect(() => {
    setPlaying(false);
    setTime(0);
  }, [caption.animation, caption.text, duration]);
  useEffect(() => {
    if (!playing) return;
    const start = performance.now();
    const timer = setInterval(() => {
      const elapsed = (performance.now() - start) / 1000;
      setTime(Math.min(elapsed, duration));
      if (elapsed >= duration) setPlaying(false);
    }, 1000 / 30);
    return () => clearInterval(timer);
  }, [playing, duration]);
  const animated = caption.animation && caption.animation !== "none";
  return (
    <section className="caption-animation-fields">
      <CaptionAnimationSelect caption={caption} patch={patch} />
      {animated && (
        <>
          {(caption.animation === "highlight" ||
            caption.animation === "glow") && (
            <label>
              {t("强调颜色")}
              <input
                type="color"
                value={
                  caption.highlightColor ??
                  (caption.animation === "glow" ? "#5fe5ff" : "#ffe14a")
                }
                onChange={(e) => patch({ highlightColor: e.target.value })}
              />
            </label>
          )}
          {caption.animation === "highlight" && (
            <small>
              {t(
                caption.words?.length
                  ? "使用词级时间；时间相对于本句开始。"
                  : "当前按文字均分时长，不代表语音对齐。",
              )}
            </small>
          )}
          {duration > 0 && (
            <>
              <img
                className="caption-animation-preview"
                alt={t("字幕动画预览")}
                src={captionImage(
                  caption,
                  Math.round((width / Math.max(width, height)) * 640),
                  Math.round((height / Math.max(width, height)) * 640),
                  Math.min(time, duration - 0.001),
                )}
              />
              <div className="inline">
                <button
                  type="button"
                  onClick={() => {
                    setTime(0);
                    setPlaying((v) => !v);
                  }}
                >
                  {t(playing ? "停止预览" : "播放动画")}
                </button>
                <small>
                  {time.toFixed(1)} / {duration.toFixed(1)} s
                </small>
              </div>
              <input
                aria-label={t("动画预览进度")}
                type="range"
                min={0}
                max={Math.max(0, duration - 0.001)}
                step={1 / 30}
                value={time}
                onChange={(e) => {
                  setPlaying(false);
                  setTime(+e.target.value);
                }}
              />
            </>
          )}
        </>
      )}
    </section>
  );
}
