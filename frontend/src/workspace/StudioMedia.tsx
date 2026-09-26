import type { ComponentProps } from "react";
import { DockPanel } from "../ui/DockPanel";
import { MediaPanel } from "./MediaPanel";
export function StudioMedia({
  close,
  ...props
}: ComponentProps<typeof MediaPanel> & { close: () => void }) {
  return (
    <DockPanel id="media" title="素材" onClose={close}>
      <MediaPanel {...props} />
    </DockPanel>
  );
}
