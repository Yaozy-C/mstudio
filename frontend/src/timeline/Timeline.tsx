import { TrackHeading } from "./TrackHeading";
import { TimelineToolbar } from "./TimelineToolbar";
import { memo, useMemo, useRef, useEffect } from "react";
import { type Project } from "../model";
import { type PlaybackClock } from "./clock";
import { ticks, frameTime } from "./geometry";
import { intervals, inRange } from "./intervals";
import { endTime, tracksOf } from "./document";
import { CLIP_DRAG_TYPE, REFERENCE_DRAG_TYPE, dropOnTimeline } from "./drop";
import { Playhead } from "./ClockReadout";
import { useTimelineViewport } from "./useTimelineViewport";
import { TimelineClip } from "./TimelineClip";
type Props = {
  onOpen: (id: string) => void;
  onPlay: () => void;
  project: Project;
  clock: PlaybackClock;
  selected: string | null;
  onSelect: (id: string | null) => void;
  onChange: (fn: (p: Project) => Project) => void;
  onSplit: () => void;
  onCollapse: () => void;
  onReference: (id: string) => void;
};
export const Timeline = memo(function Timeline({
  onOpen,
  onPlay,
  project,
  clock,
  selected,
  onSelect,
  onChange,
  onSplit,
  onCollapse,
  onReference,
}: Props) {
  const { ref, labels, zoom, viewport, changeZoom, onScroll } =
    useTimelineViewport(clock);
  const seekFrame = useRef(0);
  useEffect(() => () => cancelAnimationFrame(seekFrame.current), []);
  const index = useMemo(() => intervals(project.clips), [project.clips]);
  const assets = useMemo(
    () => new Map(project.assets.map((a) => [a.id, a])),
    [project.assets],
  );
  const total = endTime(project),
    tracks = tracksOf(project);
  // Top row is the top visual layer, as in the composition preview/export.
  const rows = [
    ...tracks.filter((t) => t.kind === "video").reverse(),
    ...tracks.filter((t) => t.kind === "audio"),
  ];
  const width = Math.max(viewport.width, (total + 10) * zoom);
  const visible = useMemo(
    () =>
      inRange(
        index,
        (viewport.left - 200) / zoom,
        (viewport.left + viewport.width + 200) / zoom,
      ),
    [index, viewport, zoom],
  );
  const marks = ticks(viewport.left, viewport.width, zoom, project.fps);
  function seek(clientX: number) {
    if (!ref.current) return;
    clock.seek(
      (clientX -
        ref.current.getBoundingClientRect().left +
        ref.current.scrollLeft) /
        zoom,
    );
  }
  return (
    <section className="timeline floating-timeline" aria-label="多轨时间线">
      <TimelineToolbar
        onPlay={onPlay}
        onFit={() =>
          changeZoom(
            Math.max(24, viewport.width - 24) / (Math.max(total, 1) * zoom),
          )
        }
        {...{
          project,
          clock,
          selected,
          onSelect,
          onChange,
          onSplit,
          onCollapse,
          changeZoom,
          zoom,
          total,
        }}
      />
      <div className="timeline-body">
        <div className="track-label-viewport" ref={labels}>
          <div className="track-labels">
            <div>轨道 · 上层覆盖下层</div>
            {rows.map((t) => (
              <TrackHeading
                key={t.id}
                track={t}
                project={project}
                onChange={onChange}
                onRemove={() => {
                  clock.pause();
                  if (
                    project.clips.some(
                      (c) => c.id === selected && c.trackId === t.id,
                    )
                  )
                    onSelect(null);
                }}
              />
            ))}
            <div>字幕</div>
          </div>
        </div>
        <div
          className="track-scroll"
          ref={ref}
          onScroll={(e) => {
            onScroll();
            if (labels.current)
              labels.current.scrollTop = e.currentTarget.scrollTop;
          }}
        >
          <div className="track-content" style={{ width }}>
            <div
              className="ruler"
              onPointerDown={(e) => {
                if (e.button !== 0) return;
                e.preventDefault();
                clock.pause();
                e.currentTarget.setPointerCapture(e.pointerId);
                seek(e.clientX);
              }}
              onPointerMove={(e) => {
                if (!e.currentTarget.hasPointerCapture(e.pointerId)) return;
                cancelAnimationFrame(seekFrame.current);
                const x = e.clientX;
                seekFrame.current = requestAnimationFrame(() => seek(x));
              }}
              onPointerUp={(e) => {
                cancelAnimationFrame(seekFrame.current);
                seek(e.clientX);
                e.currentTarget.releasePointerCapture(e.pointerId);
              }}
            >
              {marks.map((f) => (
                <span key={f} style={{ left: (f / project.fps) * zoom }}>
                  {frameTime(f / project.fps, project.fps)}
                </span>
              ))}
            </div>
            {rows.map((t) => (
              <div
                key={t.id}
                className={`${t.kind}-track ${t.hidden ? "track-hidden" : ""}`}
                onDragOver={(e) => {
                  if (
                    e.dataTransfer.types.some(
                      (type) =>
                        type === CLIP_DRAG_TYPE || type === REFERENCE_DRAG_TYPE,
                    )
                  ) {
                    e.preventDefault();
                    e.dataTransfer.dropEffect = e.dataTransfer.types.includes(
                      CLIP_DRAG_TYPE,
                    )
                      ? "move"
                      : "copy";
                  }
                }}
                onDrop={(e) => {
                  e.preventDefault();
                  const type = e.dataTransfer.types.includes(CLIP_DRAG_TYPE)
                    ? CLIP_DRAG_TYPE
                    : REFERENCE_DRAG_TYPE;
                  const raw = e.dataTransfer.getData(type);
                  if (!raw) return;
                  const time =
                    (e.clientX - e.currentTarget.getBoundingClientRect().left) /
                    zoom;
                  clock.pause();
                  onChange((p) => dropOnTimeline(p, type, raw, time, t.id));
                }}
              >
                {visible
                  .filter((e) => e.clip.trackId === t.id)
                  .map((e) => (
                    <TimelineClip
                      key={e.clip.id}
                      entry={e}
                      asset={assets.get(e.clip.assetId)}
                      zoom={zoom}
                      fps={project.fps}
                      selected={selected === e.clip.id}
                      audio={t.kind === "audio"}
                      onSelect={() => {
                        onSelect(e.clip.id);
                        clock.pause();
                        clock.seek(e.start);
                      }}
                      onOpen={() => onOpen(e.clip.id)}
                      onReference={() => onReference(e.clip.id)}
                      onRemove={() => {
                        onChange((p) => ({
                          ...p,
                          clips: p.clips.filter((c) => c.id !== e.clip.id),
                        }));
                        onSelect(null);
                      }}
                      onChange={onChange}
                    />
                  ))}
                {!project.clips.some((c) => c.trackId === t.id) && (
                  <div className="track-empty">
                    将片段拖到此处 · 可自由错开、重叠
                  </div>
                )}
              </div>
            ))}
            <div className="caption-track">
              {(project.captions ?? [])
                .filter(
                  (c) =>
                    c.end * zoom >= viewport.left &&
                    c.start * zoom <= viewport.left + viewport.width,
                )
                .map((c) => (
                  <button
                    key={c.id}
                    className="caption-chip"
                    style={{
                      left: c.start * zoom,
                      width: (c.end - c.start) * zoom,
                    }}
                    onClick={() => {
                      clock.pause();
                      clock.seek(c.start);
                    }}
                    title={c.text}
                  >
                    {c.text}
                  </button>
                ))}
            </div>
            <Playhead clock={clock} zoom={zoom} />
          </div>
        </div>
      </div>
    </section>
  );
});
