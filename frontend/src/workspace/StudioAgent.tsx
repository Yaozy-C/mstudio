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
  onInspect,
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
  onInspect: () => void;
}) {
  return (
    <DockPanel id="agent" title="项目助手" visible={visible} onClose={onClose}>
      {visible && clipId && (
        <div className="agent-clip-context">
          <span>
            当前片段 ·{" "}
            {project.assets.find(
              (a) =>
                a.id === project.clips.find((c) => c.id === clipId)?.assetId,
            )?.name ?? "视频"}
          </span>
          <button onClick={onInspect}>返回片段属性</button>
        </div>
      )}
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
