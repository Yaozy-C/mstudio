import { ZoomControls } from "../ui/ZoomControls";
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
      <ZoomControls scale={scale} zoom={zoom} fit={fit} />
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
          <button onClick={reference}>{t("引用到对话")}</button>
        </div>
      )}
      {empty && (
        <div className="canvas-empty">
          <h2>{t("画布为空")}</h2>
          <p>{t("让 Agent 拆出镜头，或从素材的右键菜单放入画布。")}</p>
        </div>
      )}
    </>
  );
}
