import { useColorComparison } from "./useColorComparison";
import { NativePreview } from "./NativePreview";
import { ErrorNotice } from "../errors/ErrorNotice";
import { memo, useEffect, useMemo, useRef, useState } from "react";
import { Play, Pause, SkipBack } from "@phosphor-icons/react";
import { type Project } from "../model";
import type { PlaybackClock } from "./clock";
import { intervals, activeAt, upcoming, inRange } from "./intervals";
import { tracksOf } from "./document";
import { MediaLayer } from "./MediaLayer";
import { usePreviewSources } from "./usePreviewSources";
import { PlaybackDiagnostics } from "./PlaybackDiagnostics";
import { ClockReadout } from "./ClockReadout";
import { captionImage } from "../creation/captions";
import { useFrameSize } from "./useFrameSize";
import { native } from "../bridge";
import { useAudioMix } from "./useAudioMix";
import { MixPlayer } from "./MixPlayer";
export function Preview(props: { project: Project; clock: PlaybackClock }) {
  const project = useColorComparison(props.project);
  return native ? (
    <NativePreview {...props} project={project} />
  ) : (
    <BrowserPreview {...props} project={project} />
  );
}
const BrowserPreview = memo(function BrowserPreview({
  project,
  clock,
}: {
  project: Project;
  clock: PlaybackClock;
}) {
  const frame = useFrameSize(project.width, project.height);
  const mix = useAudioMix(project, clock);
  const [view, setView] = useState({
    ids: [] as string[],
    warm: [] as string[],
    captions: [] as string[],
    playing: false,
  });
  const [error, setError] = useState("");
  const [element, setElement] = useState<HTMLVideoElement | null>(null);
  const [smooth, setSmooth] = useState(true);
  const [diagnostics, setDiagnostics] = useState(false);
  const [stats] = useState(() => ({ seeks: 0, waiting: 0, playCalls: 0 }));
  const masterId = useRef<string | undefined>(undefined);
  const index = useMemo(() => intervals(project.clips), [project.clips]);
  const assets = useMemo(
    () => new Map(project.assets.map((a) => [a.id, a])),
    [project.assets],
  );
  const tracks = tracksOf(project);
  const captionIndex = useMemo(
    () =>
      intervals(
        (project.captions ?? []).map((c) => ({
          id: c.id,
          assetId: "",
          start: c.start,
          trimIn: 0,
          trimOut: c.end - c.start,
          speed: 1,
          volume: 0,
          trackId: "v1",
        })),
      ),
    [project.captions],
  );
  useEffect(() => {
    let key = "";
    const sync = () => {
      const { time, playing } = clock.getSnapshot();
      const t = Math.min(time, Math.max(0, clock.total - 1 / project.fps));
      const ids = activeAt(index, t).map((e) => e.clip.id);
      // Warm only the next media per track; never decode the full canvas/library.
      const warm = [
        ...upcoming(index, t),
        ...inRange(index, Math.max(0, t - 2), t).filter((e) => e.end <= t),
      ].map((e) => e.clip.id);
      const captions = activeAt(captionIndex, t).map((c) => c.clip.id);
      const nextKey = `${playing}|${ids.join(",")}|${warm.join(",")}|${captions.join(",")}`;
      if (key !== nextKey) {
        key = nextKey;
        setView({ ids, warm, captions, playing });
      }
    };
    sync();
    return clock.subscribe(sync);
  }, [clock, index, project.fps, captionIndex]);
  const shown = useMemo(
    () =>
      index.filter((e) => {
        if (!view.ids.includes(e.clip.id) && !view.warm.includes(e.clip.id))
          return false;
        const asset = assets.get(e.clip.assetId),
          track = tracks.find((t) => t.id === e.clip.trackId);
        return (
          asset &&
          track &&
          ((track.kind === "video" && !track.hidden) ||
            (!native && asset.hasAudio && !track.muted))
        );
      }),
    [index, view.ids, view.warm, assets, tracks],
  );
  const sources = usePreviewSources(
    project.id,
    shown.map((e) => assets.get(e.clip.assetId)),
    clock,
    smooth,
  );
  const candidates = shown.filter(
    (e) =>
      view.ids.includes(e.clip.id) &&
      !assets.get(e.clip.assetId)?.missing &&
      assets.get(e.clip.assetId)?.kind !== "image",
  );
  const master =
    candidates.find((e) => e.clip.id === masterId.current) ??
    [...candidates].sort(
      (a, b) =>
        Number(assets.get(b.clip.assetId)?.kind === "audio") -
          Number(assets.get(a.clip.assetId)?.kind === "audio") || b.end - a.end,
    )[0];
  masterId.current = master?.clip.id;
  useEffect(() => {
    if (!master) setElement(null);
  }, [master]);
  useEffect(() => () => clock.pause(), [clock]);
  const order = new Map(tracks.map((t, i) => [t.id, i]));
  // Stable document order resolves overlaps within the same track.
  const documentOrder = new Map(project.clips.map((c, i) => [c.id, i]));
  const layers = [...shown].sort(
    (a, b) =>
      (order.get(a.clip.trackId!) ?? 0) - (order.get(b.clip.trackId!) ?? 0) ||
      documentOrder.get(a.clip.id)! - documentOrder.get(b.clip.id)!,
  );
  return (
    <div className="preview">
      <header>
        <span>成片预览</span>
        <small>
          {project.width} × {project.height}
        </small>
      </header>
      <div className="preview-stage" ref={frame.ref}>
        <div className="preview-frame" style={frame.size}>
          {layers.map((e) => {
            const asset = assets.get(e.clip.assetId),
              track = tracks.find((t) => t.id === e.clip.trackId);
            return asset && track ? (
              <MediaLayer
                key={e.clip.id}
                entry={e}
                asset={asset}
                track={track}
                path={sources.paths[asset.id]}
                active={view.ids.includes(e.clip.id)}
                master={!mix.path && master?.clip.id === e.clip.id}
                silent={native}
                clock={clock}
                stats={stats}
                onError={setError}
                onElement={setElement}
              />
            ) : null;
          })}
          {mix.path && (
            <MixPlayer
              path={mix.path}
              total={mix.total}
              clock={clock}
              stats={stats}
              onError={setError}
            />
          )}
          {(project.captions ?? [])
            .filter((c) => view.captions.includes(c.id))
            .map((c) => (
              <img
                key={c.id}
                className="caption-overlay"
                src={captionImage(c, project.width, project.height)}
                alt={c.text}
              />
            ))}
          {!project.clips.length && (
            <div className="preview-placeholder">
              <Play size={26} />
              <span>将素材或镜头加入时间线</span>
            </div>
          )}
        </div>
      </div>
      <div className="preview-options">
        <select
          aria-label="预览画质"
          value={smooth ? "smooth" : "original"}
          onChange={(e) => {
            clock.pause();
            setError("");
            setSmooth(e.target.value === "smooth");
          }}
        >
          <option value="smooth">流畅预览 · 640p</option>
          <option value="original">原片画质</option>
        </select>
        <button onClick={() => setDiagnostics((v) => !v)}>诊断</button>
      </div>
      {project.clips.some((c) => c.transition) && (
        <small className="preview-status">
          转场效果请在桌面应用中预览；浏览器当前显示直接切换。
        </small>
      )}
      {sources.pending && (
        <small className="preview-status">正在准备代理，当前播放原片</small>
      )}
      {mix.pending && (
        <small className="preview-status">正在准备声音预览…</small>
      )}
      {mix.error && (
        <ErrorNotice error={mix.error} fallback="OPERATION_FAILED" />
      )}
      {sources.error && (
        <small className="preview-status">{sources.error}</small>
      )}
      {diagnostics && <PlaybackDiagnostics element={element} stats={stats} />}
      {error && <ErrorNotice error={error} fallback="OPERATION_FAILED" />}
      <div className="preview-transport">
        <button
          title="回到起点 Home"
          onClick={() => {
            clock.pause();
            clock.seek(0);
          }}
        >
          <SkipBack />
        </button>
        <button
          className="play-button"
          disabled={!project.clips.length || mix.pending || !!mix.error}
          title={view.playing ? "暂停 Space" : "播放 Space"}
          onClick={clock.toggle}
        >
          {view.playing ? <Pause weight="fill" /> : <Play weight="fill" />}
        </button>
        <ClockReadout clock={clock} />
      </div>
    </div>
  );
});
