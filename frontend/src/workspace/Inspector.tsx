import { ActionButton } from "../ui/ActionButton";
import { t, useLanguage } from "../i18n";
import { useState } from "react";
import { ColorRequest } from "../timeline/ColorRequest";
import { VisualFields } from "../timeline/VisualFields";
import { detachAudio } from "../timeline/detachAudio";
import type { Project, Clip } from "../model";
import { ClipInspectorPanels } from "../timeline/ClipInspectorPanels";
import {
  ImageSquare,
  Sun,
  SpeakerHigh,
  Clock,
  Waveform,
} from "@phosphor-icons/react";
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
  return (
    <div className="inspector" data-tab={activeTab}>
      {clip ? (
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
              />
            )}
            {activeTab === "color" && video && (
              <ColorRequest key={clip.id} clip={clip}>
                <VisualFields clip={clip} update={updateClip} />
              </ColorRequest>
            )}
            {activeTab === "audio" &&
              asset?.hasAudio &&
              asset.kind === "video" &&
              video && (
                <ActionButton
                  icon={Waveform}
                  onClick={() => onChange((p) => detachAudio(p, clip.id))}
                >
                  {t("分离音频")}
                </ActionButton>
              )}
          </div>
        </>
      ) : (
        <>
          <label>
            {t("项目名称")}
            <input
              value={project.name}
              onChange={(e) =>
                onChange((p) => ({ ...p, name: e.target.value }))
              }
            />
          </label>
          <label>
            {t("画面比例")}
            <select
              value={`${project.width}x${project.height}`}
              onChange={(e) => {
                const [width, height] = e.target.value.split("x").map(Number);
                onChange((p) => ({ ...p, width, height }));
              }}
            >
              <option value="1080x1920">{t("9:16 · 竖屏")}</option>
              <option value="1920x1080">{t("16:9 · 横屏")}</option>
              <option value="1080x1080">{t("1:1 · 方形")}</option>
              <option value="720x1280">9:16 · 720p</option>
            </select>
          </label>
          <label>
            {t("帧率")}
            <select
              value={project.fps}
              onChange={(e) =>
                onChange((p) => ({ ...p, fps: +e.target.value }))
              }
            >
              {[24, 25, 30, 60].map((v) => (
                <option key={v}>{v}</option>
              ))}
            </select>
          </label>
          <label>
            {t("创作要求与已确认事实")}
            <textarea
              rows={5}
              placeholder={t("视频目标、受众、风格、锁定要求…")}
              value={project.brief}
              onChange={(e) =>
                onChange((p) => ({ ...p, brief: e.target.value }))
              }
            />
          </label>
        </>
      )}
    </div>
  );
}
