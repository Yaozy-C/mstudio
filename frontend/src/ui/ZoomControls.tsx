import { Minus, Plus, CornersOut } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import "./zoom-controls.css";

export function ZoomControls({
  scale,
  zoom,
  fit,
  image = false,
  minimum = 0,
  maximum = Infinity,
}: {
  scale: number;
  zoom: (factor: number) => void;
  fit: () => void;
  image?: boolean;
  minimum?: number;
  maximum?: number;
}) {
  useLanguage();
  return (
    <div
      className="canvas-controls"
      role="group"
      aria-label={t(image ? "图片缩放" : "画布缩放")}
    >
      <button
        type="button"
        onClick={() => zoom(0.8)}
        disabled={scale <= minimum}
        aria-label={t(image ? "缩小图片" : "缩小画布")}
      >
        <Minus />
      </button>
      <span aria-label={t("缩放比例")}>{Math.round(scale * 100)}%</span>
      <button
        type="button"
        onClick={() => zoom(1.25)}
        disabled={scale >= maximum}
        aria-label={t(image ? "放大图片" : "放大画布")}
      >
        <Plus />
      </button>
      <button type="button" onClick={fit} title={t("适应全部内容")}>
        <CornersOut />
        {t("全部适应")}
      </button>
    </div>
  );
}
