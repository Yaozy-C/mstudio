import { useRef, useState } from "react";
import { Check, Link, PencilSimple } from "@phosphor-icons/react";
import { mediaUrl } from "../bridge";
import { t } from "../i18n";
import { ActionButton } from "../ui/ActionButton";
import { AsyncButton, StatusMessage } from "../ui/AsyncState";
import { usePrevis } from "./context";
import { isStale, type PrevisExport } from "./types";

export function PrevisVideoDetail({ video }: { video: PrevisExport }) {
  const {
    workspace,
    project,
    tasks,
    open,
    closeVideos,
    attachReference,
    collectVideo,
  } = usePrevis();
  const [pending, setPending] = useState<"reference" | "save" | null>(null);
  const [error, setError] = useState("");
  const [unavailable, setUnavailable] = useState(
    !!video.asset.missing || !!video.asset.discarded,
  );
  const saving = useRef(false);
  const target = project.nodes.find(
    (node) => node.id === video.fileId && node.shot,
  );
  const file = workspace.files.find((entry) => entry.id === video.fileId);
  const linked = !!target?.references?.some(
    (ref) => ref.assetId === video.asset.id,
  );
  const collected = project.assets.some(
    (asset) =>
      asset.id === video.savedAssetId &&
      asset.inLibrary &&
      !asset.missing &&
      !asset.discarded,
  );
  const rendering = tasks.some(
    (task) => task.fileId === video.fileId && task.running,
  );
  async function useVideo(kind: "reference" | "save") {
    if (saving.current) return;
    saving.current = true;
    setPending(kind);
    setError("");
    try {
      if (kind === "reference")
        await attachReference(video.fileId, video.id, video.asset.id);
      else await collectVideo(video.id, video.asset.id);
    } catch (e) {
      setError(String(e));
    } finally {
      saving.current = false;
      setPending(null);
    }
  }
  return (
    <>
      <div className="previs-video-player">
        {unavailable ? (
          <StatusMessage kind="error">
            {t("预演视频不可用，请重新生成。")}
          </StatusMessage>
        ) : (
          <video
            src={mediaUrl(video.asset.path)}
            controls
            preload="metadata"
            playsInline
            aria-label={video.asset.name}
            onError={() => setUnavailable(true)}
          />
        )}
      </div>
      <div className="previs-video-info">
        <h3>{target?.title || file?.name || video.asset.name}</h3>
        <p>
          {t("{seconds} 秒", { seconds: video.asset.duration.toFixed(2) })} ·{" "}
          {video.asset.width} × {video.asset.height}
        </p>
        {!target ? (
          <StatusMessage kind="info">
            {t("镜头已移除，视频已保留")}
          </StatusMessage>
        ) : (
          isStale(video, workspace.files) && (
            <StatusMessage kind="info">
              {t("场景已有修改；此视频为上次保存的预演。")}
            </StatusMessage>
          )
        )}
        {rendering && (
          <StatusMessage>
            {t("新的预演正在生成；当前仍可查看上次保存的视频。")}
          </StatusMessage>
        )}
        {error && <StatusMessage kind="error">{error}</StatusMessage>}
        <div className="previs-video-actions">
          <ActionButton
            icon={PencilSimple}
            disabled={!file || !target || !!pending}
            onClick={() => {
              closeVideos();
              open(video.fileId);
            }}
          >
            {t("查看或修改预演")}
          </ActionButton>
          <AsyncButton
            busy={pending === "reference"}
            busyLabel={t("正在设置参考…")}
            disabled={
              !!pending || unavailable || linked || !target || rendering
            }
            onClick={() => void useVideo("reference")}
          >
            {linked ? <Check /> : <Link />}
            {linked ? t("已设为参考") : t("设为参考")}
          </AsyncButton>
          <AsyncButton
            busy={pending === "save"}
            busyLabel={t("正在保存…")}
            disabled={!!pending || unavailable || collected || rendering}
            onClick={() => void useVideo("save")}
          >
            {collected ? t("已保存为项目素材") : t("保存为项目素材")}
          </AsyncButton>
        </div>
      </div>
    </>
  );
}
