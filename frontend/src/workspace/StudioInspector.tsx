import { t, useLanguage } from "../i18n";
import type { ComponentProps } from "react";
import { DockPanel } from "../ui/DockPanel";
import { Inspector } from "./Inspector";
export function StudioInspector({
  close,
  ...props
}: ComponentProps<typeof Inspector> & { close: () => void }) {
  useLanguage();
  return (
    <DockPanel
      id="inspector"
      title={props.clipId ? t("编辑片段") : t("项目设置")}
      onClose={close}
    >
      <Inspector {...props} />
    </DockPanel>
  );
}
