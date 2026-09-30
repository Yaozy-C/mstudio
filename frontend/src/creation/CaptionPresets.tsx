import { useState } from "react";
import { t, useLanguage } from "../i18n";
import type { Caption } from "../model";
import {
  captionAppearance,
  captionPresets,
  type CaptionAppearance,
} from "./captionStyle";
import { captionImage } from "./captions";
import "../styles/caption-style.css";

export function CaptionPresets({
  caption,
  apply,
}: {
  caption?: Caption;
  apply: (style: CaptionAppearance) => void;
}) {
  useLanguage();
  const [group, setGroup] = useState("全部");
  return (
    <div className="caption-presets">
      <div className="caption-preset-filters" aria-label={t("字幕风格分类")}>
        {["全部", "口播", "花字", "叙事"].map((g) => (
          <button
            key={g}
            aria-pressed={group === g}
            onClick={() => setGroup(g)}
          >
            {t(g)}
          </button>
        ))}
      </div>
      <div className="caption-preset-grid">
        {captionPresets
          .filter((p) => group === "全部" || p.group === group)
          .map((p) => (
            <button
              key={p.id}
              className="caption-preset-card"
              aria-pressed={
                !!caption &&
                JSON.stringify(captionAppearance(caption)) ===
                  JSON.stringify(p.style)
              }
              onClick={() => apply(p.style)}
              title={t(p.name)}
            >
              <img
                alt=""
                src={captionImage(
                  {
                    id: p.id,
                    start: 0,
                    end: 1,
                    ...p.style,
                    text: t(p.name),
                    fontSize: 0.22,
                    x: 0.5,
                    y: 0.5,
                  },
                  480,
                  200,
                )}
              />
              <span>{t(p.name)}</span>
            </button>
          ))}
      </div>
    </div>
  );
}
