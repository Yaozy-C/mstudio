import type { ComponentProps } from "react";
import { FloatingPanel } from "../ui/FloatingPanel";
import { Inspector } from "./Inspector";
export function StudioInspector({
  close,
  ...props
}: ComponentProps<typeof Inspector> & { close: () => void }) {
  return (
    <FloatingPanel
      id="inspector"
      title="属性"
      initial={{ x: 320, y: 136, width: 320, height: 460 }}
      onClose={close}
    >
      <Inspector {...props} />
    </FloatingPanel>
  );
}
