import { t, useLanguage } from "../i18n";
import { useRef, useState } from "react";
import type { Caption } from "../model";
import { captionFonts } from "./captionStyle";
import "../styles/caption-style.css";
export function CaptionStyleFields({
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
  const area = useRef<HTMLDivElement>(null);
  const [draft, setDraft] = useState<{ x: number; y: number } | null>(null);
  const position = draft ?? { x: caption.x ?? 0.5, y: caption.y ?? 0.85 };
  const locate = (clientX: number, clientY: number) => {
    const rect = area.current!.getBoundingClientRect();
    return {
      x: Math.max(0.05, Math.min(0.95, (clientX - rect.left) / rect.width)),
      y: Math.max(0.05, Math.min(0.95, (clientY - rect.top) / rect.height)),
    };
  };
  return (
    <div className="caption-style-fields">
      <div className="field-grid">
        <label>
          {t("字体")}
          <select
            value={caption.font ?? "sans"}
            onChange={(e) => patch({ font: e.target.value as Caption["font"] })}
          >
            {Object.entries(captionFonts).map(([key, font]) => (
              <option key={key} value={key}>
                {font.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          {t("文字颜色")}
          <input
            type="color"
            value={caption.color ?? "#ffffff"}
            onChange={(e) => patch({ color: e.target.value })}
          />
        </label>
      </div>
      <label>
        {t("字号 ·")}{" "}
        {Math.round(Math.min(width, height) * (caption.fontSize ?? 0.048))} px
        <input
          aria-label={t("字幕字号")}
          type="range"
          min={0.02}
          max={0.12}
          step={0.002}
          value={caption.fontSize ?? 0.048}
          onChange={(e) => patch({ fontSize: +e.target.value })}
        />
      </label>
      <label className="inline">
        <input
          type="checkbox"
          checked={caption.background ?? true}
          onChange={(e) => patch({ background: e.target.checked })}
        />
        {t("显示字幕底色")}
      </label>
      <div
        className="caption-position"
        ref={area}
        style={{
          aspectRatio: `${width}/${height}`,
          width: `min(100%, ${Math.min(280, (300 * width) / height)}px)`,
        }}
        role="group"
        aria-label={t("字幕画面位置")}
      >
        <span className="caption-safe-area" />
        <button
          className="caption-position-text"
          aria-label={t("拖动字幕位置，方向键微调")}
          style={{
            left: `${position.x * 100}%`,
            top: `${position.y * 100}%`,
            color: caption.color ?? "#ffffff",
            fontFamily: captionFonts[caption.font ?? "sans"].family,
            background: caption.background === false ? "transparent" : "#000a",
          }}
          onPointerDown={(e) => {
            e.preventDefault();
            e.currentTarget.setPointerCapture(e.pointerId);
            setDraft(position);
          }}
          onPointerMove={(e) => {
            if (e.currentTarget.hasPointerCapture(e.pointerId))
              setDraft(locate(e.clientX, e.clientY));
          }}
          onPointerUp={(e) => {
            if (!e.currentTarget.hasPointerCapture(e.pointerId)) return;
            patch(locate(e.clientX, e.clientY));
            setDraft(null);
            e.currentTarget.releasePointerCapture(e.pointerId);
          }}
          onPointerCancel={() => setDraft(null)}
          onKeyDown={(e) => {
            const move: Record<string, [number, number]> = {
              ArrowLeft: [-0.01, 0],
              ArrowRight: [0.01, 0],
              ArrowUp: [0, -0.01],
              ArrowDown: [0, 0.01],
            };
            if (move[e.key]) {
              e.preventDefault();
              const [dx, dy] = move[e.key];
              patch({
                x: Math.max(0.05, Math.min(0.95, position.x + dx)),
                y: Math.max(0.05, Math.min(0.95, position.y + dy)),
              });
            }
          }}
        >
          {caption.text || t("字幕")}
        </button>
      </div>
      <small>{t("拖动文字调整画面位置，也可用方向键微调。")}</small>
      <div className="field-grid">
        {(["x", "y"] as const).map((key) => (
          <label key={key}>
            {key === "x" ? t("水平位置") : t("垂直位置")} / %
            <input
              type="number"
              min={5}
              max={95}
              value={Math.round(position[key] * 100)}
              onChange={(e) => {
                if (Number.isFinite(e.target.valueAsNumber))
                  patch({
                    [key]: Math.max(
                      0.05,
                      Math.min(0.95, +e.target.value / 100),
                    ),
                  });
              }}
            />
          </label>
        ))}
      </div>
      <button
        onClick={() =>
          patch({
            x: 0.5,
            y: 0.85,
            font: "sans",
            fontSize: 0.048,
            color: "#ffffff",
            background: true,
          })
        }
      >
        {t("重置字幕样式")}
      </button>
    </div>
  );
}
