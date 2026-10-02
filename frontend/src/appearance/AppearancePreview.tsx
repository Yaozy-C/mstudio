import { FilmSlate, Play } from "@phosphor-icons/react";
import { t } from "../i18n";
import type { Appearance } from "./preferences";
import "./preview.css";

export function AppearancePreview({ surface, contrast }: Appearance) {
  return (
    <span
      className="appearance-preview"
      data-surface={surface}
      data-contrast={contrast}
      aria-hidden="true"
    >
      <span className="appearance-preview-bar">
        <span>
          <FilmSlate /> m / studio
        </span>
        <span>
          {t("脚本")}
          <b>{t("制作画布")}</b>
          {t("成片")}
        </span>
      </span>
      <span className="appearance-preview-body">
        <span className="appearance-preview-sidebar">
          <span>{t("素材")}</span>
          <span className="appearance-preview-assets">
            {[0, 1, 2, 3].map((id) => (
              <i key={id} />
            ))}
          </span>
        </span>
        <span className="appearance-preview-canvas">
          {["01", "02", "03"].map((number) => (
            <span className="appearance-preview-shot" key={number}>
              <span>
                <FilmSlate />
              </span>
              <b>{number}</b>
              <i />
            </span>
          ))}
        </span>
      </span>
      <span className="appearance-preview-timeline">
        <span>
          <Play weight="fill" /> 00:00
        </span>
        <span className="appearance-preview-tracks">
          <i />
          <i />
          <i />
        </span>
      </span>
    </span>
  );
}
