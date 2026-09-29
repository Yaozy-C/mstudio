import { t, useLanguage } from "../i18n";
import type { ComponentProps } from "react";
import { DockPanel } from "../ui/DockPanel";
import { Inspector } from "./Inspector";
export function StudioInspector({
  close,
  ...props
}: ComponentProps<typeof Inspector> & { close: () => void }) {
  useLanguage();
  if (!props.project.clips.some((clip) => clip.id === props.clipId))
    return null;
  return (
    <DockPanel id="inspector" title={t("编辑片段")} onClose={close}>
      <Inspector {...props} />
    </DockPanel>
  );
}
