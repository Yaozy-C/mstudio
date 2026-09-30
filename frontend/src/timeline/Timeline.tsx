import { CaptionTrackHeadings } from "./CaptionTrackHeading";
import { useTimelineCaptionSelection } from "../creation/captionSelection";
import { t as translate, useLanguage } from "../i18n";
import { TransitionSeams } from "./TransitionSeams";
import { CaptionTrackRows } from "./CaptionTrackRows";
import { trackRows } from "./trackRows";
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
  onCaption?: () => void;
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
  onCaption,
}: Props) {
  useLanguage();
  const { ref, labels, zoom, viewport, changeZoom, onScroll } =
    useTimelineViewport(clock);
  const [captionId, setCaptionId] = useTimelineCaptionSelection(
    clock,
    selected,
    onSelect,
  );
  const selectedCaption =
    !selected && project.captions.some((c) => c.id === captionId)
      ? captionId
      : null;
  const selectClip = (id: string | null) => {
    setCaptionId(null);
    onSelect(id);
  };
  const removeCaption = () => {
    onChange((p) => ({
      ...p,
      captions: p.captions.filter((c) => c.id !== selectedCaption),
    }));
    setCaptionId(null);
  };
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
  const rows = trackRows(tracks);
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
    <section
      className="timeline floating-timeline"
      aria-label={translate("多轨时间线")}
      onKeyDown={(e) => {
        if (
          !selectedCaption ||
          e.defaultPrevented ||
          e.nativeEvent.isComposing ||
          (e.target instanceof HTMLElement &&
            e.target.closest(
              'input, textarea, select, [contenteditable="true"], [role="menu"]',
            ))
        )
          return;
        if (e.key === "Delete" || e.key === "Backspace") {
          e.preventDefault();
          removeCaption();
        }
        if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "b")
          e.preventDefault();
      }}
    >
      <TimelineToolbar
        selectedCaption={selectedCaption}
        onRemoveCaption={removeCaption}
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
            <div>{translate("轨道 · 上层覆盖下层")}</div>
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
            <CaptionTrackHeadings project={project} change={onChange} />
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
                      projectId={project.id}
                      asset={assets.get(e.clip.assetId)}
                      zoom={zoom}
                      fps={project.fps}
                      selected={selected === e.clip.id}
                      audio={t.kind === "audio"}
                      muted={!!t.muted}
                      hidden={!!t.hidden}
                      onSelect={() => {
                        selectClip(e.clip.id);
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
                <TransitionSeams
                  project={project}
                  trackId={t.id}
                  zoom={zoom}
                  clock={clock}
                  change={onChange}
                />
                {!project.clips.some((c) => c.trackId === t.id) && (
                  <div className="track-empty">
                    {translate("将片段拖到此处 · 可自由错开、重叠")}
                  </div>
                )}
              </div>
            ))}
            <CaptionTrackRows
              selected={selectedCaption}
              onSelect={(id) => {
                onSelect(null);
                setCaptionId(id);
              }}
              {...{ project, clock, zoom, viewport, onChange, onCaption }}
            />
            <Playhead clock={clock} zoom={zoom} />
          </div>
        </div>
      </div>
    </section>
  );
});
