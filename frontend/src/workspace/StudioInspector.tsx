import type { ComponentProps } from "react";
import { X } from "@phosphor-icons/react";
import { Inspector } from "./Inspector";
export function StudioInspector({
  close,
  ...props
}: ComponentProps<typeof Inspector> & { close: () => void }) {
  return (
    <aside className="dock-panel dock-inspector" aria-label="片段编辑器">
      <div className="dock-heading">
        <h2>{props.clipId ? "编辑片段" : "项目设置"}</h2>
        <button aria-label="关闭编辑器" onClick={close}>
          <X size={20} />
        </button>
      </div>
      <Inspector {...props} />
    </aside>
  );
}
