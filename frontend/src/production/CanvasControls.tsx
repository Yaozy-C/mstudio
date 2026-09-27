import { Minus, Plus, CornersOut } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
export function CanvasControls({
  scale,
  zoom,
  fit,
  count,
  reference,
  empty,
}: {
  scale: number;
  zoom: (factor: number) => void;
  fit: () => void;
  count: number;
  reference: () => void;
  empty: boolean;
}) {
  useLanguage();
  return (
    <>
      <div className="canvas-controls">
        <button onClick={() => zoom(0.8)} aria-label={t("缩小画布")}>
          <Minus />
        </button>
        <span>{Math.round(scale * 100)}%</span>
        <button onClick={() => zoom(1.25)} aria-label={t("放大画布")}>
          <Plus />
        </button>
        <button onClick={() => fit()} title={t("适应全部内容")}>
          <CornersOut />
          {t("全部适应")}
        </button>
      </div>
      <div className="canvas-help">
        {t(
          "拖空白移动 · 滚轮平移 · ⌘/Ctrl + 滚轮缩放 · Shift 多选 / 框选 · 双击预览",
        )}
      </div>
      {!!count && (
        <div className="selection-actions">
          <span>
            {t("已选")} {count} {t("项")}
          </span>
          <button onClick={reference}>{t("引用到输入框")}</button>
        </div>
      )}
      {empty && (
        <div className="canvas-empty">
          <h2>{t("从脚本开始，把画面做出来。")}</h2>
          <p>{t("让 Agent 拆出镜头，或从素材的右键菜单放入画布。")}</p>
        </div>
      )}
    </>
  );
}
