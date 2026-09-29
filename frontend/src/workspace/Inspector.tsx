import { t, useLanguage } from "../i18n";
import { useState } from "react";
import { ColorRequest } from "../timeline/ColorRequest";
import { VisualFields } from "../timeline/VisualFields";
import { detachAudio } from "../timeline/detachAudio";
import type { Project, Clip } from "../model";
import { ClipInspectorPanels } from "../timeline/ClipInspectorPanels";
import { ImageSquare, Sun, SpeakerHigh, Clock } from "@phosphor-icons/react";
import "../styles/inspector-desk.css";

type Props = {
  project: Project;
  clipId: string | null;
  onChange: (fn: (p: Project) => Project) => void;
};
export function Inspector({ project, clipId, onChange }: Props) {
  useLanguage();
  const [tab, setTab] = useState("visual");
  const clip = project.clips.find((c) => c.id === clipId);
  const asset = project.assets.find((a) => a.id === clip?.assetId);
  const track = project.tracks.find((t) => t.id === clip?.trackId);
  const video = track?.kind === "video";
  const activeTab =
    !video && (tab === "visual" || tab === "color")
      ? asset?.hasAudio
        ? "audio"
        : "time"
      : tab === "audio" && !asset?.hasAudio
        ? "visual"
        : tab;
  const updateClip = (updated: Clip) =>
    onChange((p) => ({
      ...p,
      clips: p.clips.map((c) => (c.id === updated.id ? updated : c)),
    }));
  if (!clip) return null;
  return (
    <div className="inspector" data-tab={activeTab}>
      <>
        <h4 className="inspector-asset" title={asset?.name}>
          {asset?.name ?? t("素材丢失")}
        </h4>
        <div
          className="inspector-tabs"
          role="group"
          aria-label={t("片段编辑分类")}
        >
          {(
            [
              ["visual", t("画面"), ImageSquare],
              ["color", t("调色"), Sun],
              ["audio", t("声音"), SpeakerHigh],
              ["time", t("时间"), Clock],
            ] as const
          )
            .filter(
              ([key]) =>
                key === "time" || (key === "audio" ? asset?.hasAudio : video),
            )
            .map(([key, label, Icon]) => (
              <button
                key={key}
                aria-pressed={activeTab === key}
                onClick={() => setTab(key)}
              >
                <Icon size={22} />
                {label}
              </button>
            ))}
        </div>
        <div className="inspector-panel-content">
          {asset && activeTab !== "color" && (
            <ClipInspectorPanels
              tab={activeTab}
              clip={clip}
              asset={asset}
              project={project}
              update={updateClip}
              detach={
                asset.hasAudio && asset.kind === "video" && video
                  ? () => onChange((p) => detachAudio(p, clip.id))
                  : undefined
              }
            />
          )}
          {activeTab === "color" && video && (
            <ColorRequest key={clip.id} clip={clip}>
              <VisualFields clip={clip} update={updateClip} />
            </ColorRequest>
          )}
        </div>
      </>
    </div>
  );
}
