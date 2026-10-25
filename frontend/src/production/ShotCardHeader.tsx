import { Cube, Play } from "@phosphor-icons/react";
import { native } from "../bridge";
import { t } from "../i18n";
import type { Project } from "../model";
import { openPrevis, useOptionalPrevis } from "../previs/context";
import { latestPrevisVideos } from "../previs/savedTasks";
import type { ProductionItem } from "./types";

export function ShotCardHeader({
  item,
  project,
}: {
  item: ProductionItem;
  project: Project;
}) {
  const shot = project.nodes.find((n) => n.id === item.nodeId)?.shot;
  const previs = useOptionalPrevis();
  const video =
    previs &&
    latestPrevisVideos(previs.workspace.exports).find(
      (v) => v.fileId === item.nodeId,
    );
  return (
    <header className="shot-card-heading">
      <div className="shot-card-heading-copy">
        <span className="shot-card-number">
          SHOT {String(shot?.order ?? 1).padStart(2, "0")}
        </span>
        <h2 title={item.title}>{item.title}</h2>
      </div>
      <div className="shot-card-actions">
        <strong className="shot-card-duration">
          {shot?.duration != null
            ? t("{v0} 秒", { v0: Number(shot.duration.toFixed(2)) })
            : t("时长待定")}
        </strong>
        {video && (
          <button
            type="button"
            className="previs-shot-button"
            disabled={!!video.asset.missing}
            onClick={(e) => {
              e.stopPropagation();
              previs.showVideos(item.nodeId, true);
            }}
          >
            <Play size={15} weight="fill" />
            {t("播放预演")}
          </button>
        )}
        <button
          type="button"
          className="previs-shot-button"
          disabled={!native}
          onClick={(e) => {
            e.stopPropagation();
            if (item.nodeId) openPrevis(item.nodeId);
          }}
        >
          <Cube size={15} />
          {t("三维预演")}
        </button>
      </div>
    </header>
  );
}
