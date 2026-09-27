import { t, useLanguage } from "../i18n";
import type { ComponentProps } from "react";
import { X } from "@phosphor-icons/react";
import { Inspector } from "./Inspector";
export function StudioInspector({
  close,
  ...props
}: ComponentProps<typeof Inspector> & { close: () => void }) {
  useLanguage();
  return (
    <aside className="dock-panel dock-inspector" aria-label={t("片段编辑器")}>
      <div className="dock-heading">
        <h2>{props.clipId ? t("编辑片段") : t("项目设置")}</h2>
        <button aria-label={t("关闭编辑器")} onClick={close}>
          <X size={20} />
        </button>
      </div>
      <Inspector {...props} />
    </aside>
  );
}
