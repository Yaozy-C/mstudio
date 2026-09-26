import type { WorkContext } from "../assistant/workContext";
import type { ProductionController } from "../production/useProduction";
import { lazy, Suspense } from "react";
import { DockPanel } from "../ui/DockPanel";
import type { Project } from "../model";
import type { PlaybackClock } from "../timeline/clock";
import type { AttachmentDraft } from "../assistant/useAttachments";
const AssistantPanel = lazy(() =>
  import("../assistant/AssistantPanel").then((m) => ({
    default: m.AssistantPanel,
  })),
);
export function StudioAgent({
  canvas,
  project,
  nodeId,
  clipId,
  clock,
  draft,
  visible,
  flush,
  work,
  onSettings,
  onClose,
}: {
  canvas?: ProductionController;
  project: Project;
  nodeId: string | null;
  clipId: string | null;
  clock: PlaybackClock;
  draft: AttachmentDraft;
  visible: boolean;
  flush: () => Promise<void>;
  work?: WorkContext;
  onSettings: () => void;
  onClose: () => void;
}) {
  return (
    <DockPanel id="agent" title="项目助手" visible={visible} onClose={onClose}>
      <Suspense fallback={<div className="agent-empty">加载 Agent…</div>}>
        <AssistantPanel
          visible={visible}
          canvas={canvas}
          project={project}
          nodeId={nodeId}
          clipId={clipId}
          clock={clock}
          draft={draft}
          flush={flush}
          work={work}
          onSettings={onSettings}
        />
      </Suspense>
    </DockPanel>
  );
}
