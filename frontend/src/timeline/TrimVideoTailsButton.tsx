import { useRef, useState } from "react";
import { Crop, X } from "@phosphor-icons/react";
import { bridge } from "../bridge";
import { t, useLanguage } from "../i18n";
import type { Project } from "../model";
import type { PlaybackClock } from "./clock";
import { trimVideoTails } from "./trimVideoTails";

export function TrimVideoTailsButton({
  project,
  clock,
  onChange,
}: {
  project: Project;
  clock: PlaybackClock;
  onChange: (fn: (p: Project) => Project) => void;
}) {
  useLanguage();
  const running = useRef(false);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const videos = project.assets.filter(
    (a) =>
      a.kind === "video" &&
      project.clips.some(
        (c) =>
          c.assetId === a.id &&
          project.tracks.some((t) => t.id === c.trackId && t.kind === "video"),
      ),
  );
  async function trim() {
    if (running.current) return;
    running.current = true;
    setBusy(true);
    setNotice("");
    clock.pause();
    try {
      const ends = await bridge<Record<string, number>>("video_source_ends", {
        projectId: project.id,
        assetIds: videos.map((a) => a.id),
      });
      onChange((current) => {
        if (current.id !== project.id) return current;
        if (
          videos.some(
            (a) => current.assets.find((v) => v.id === a.id)?.path !== a.path,
          )
        )
          throw new Error(t("素材已变化，请重新裁末尾"));
        const next = trimVideoTails(current, ends);
        setNotice(
          next === current ? t("没有需要裁掉的片尾") : t("已裁末尾，可撤销"),
        );
        return next;
      });
    } catch (error) {
      setNotice(String(error));
    } finally {
      running.current = false;
      setBusy(false);
    }
  }
  return (
    <div className="timeline-trim-control">
      <button
        aria-label={t("裁末尾")}
        title={t(
          "按有效画面时长裁掉视频尾部空档，并前移同轨后续片段（可撤销）",
        )}
        disabled={busy || !videos.length}
        aria-busy={busy}
        onClick={() => void trim()}
      >
        <Crop />
        {busy ? t("处理中…") : t("裁末尾")}
      </button>
      {notice && (
        <div className="timeline-trim-notice" role="status">
          <span>{notice}</span>
          <button aria-label={t("关闭")} onClick={() => setNotice("")}>
            <X />
          </button>
        </div>
      )}
    </div>
  );
}
