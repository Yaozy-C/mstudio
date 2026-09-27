import { t, useLanguage } from "../i18n";
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
  useLanguage();
  const clip = project.clips.find((c) => c.id === clipId);
  const asset = project.assets.find((a) => a.id === clip?.assetId);
  return (
    <DockPanel
      id="agent"
      title={t("项目助手")}
      visible={visible}
      onClose={onClose}
    >
      {visible && clip && (
        <div className="agent-clip-context">
          <span title={asset?.name}>
            {t("已选片段 ·")} {(clip.start ?? 0).toFixed(2)} {t("秒起")}
          </span>
          <button onClick={onInspect} title={t("调整画面、调色、声音和时间")}>
            {t("编辑片段")}
          </button>
        </div>
      )}
      <Suspense
        fallback={<div className="agent-empty">{t("加载 Agent…")}</div>}
      >
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
