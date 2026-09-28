import { t, useLanguage } from "../i18n";
import { useEffect, useRef, useState } from "react";
import { Play, Pause, SkipBack } from "@phosphor-icons/react";
import { bridge } from "../bridge";
import type { Project } from "../model";
import type { PlaybackClock } from "./clock";
import { prepareCaptions } from "../creation/prepareCaptions";
import { ClockReadout } from "./ClockReadout";
import { previewSpec } from "./previewSpec";
import { startPreviewFrames } from "./previewFrames";
import { StudioSelect } from "../ui/StudioSelect";
import { PreviewCommands } from "./previewCommands";
import { ErrorNotice } from "../errors/ErrorNotice";
type Status = {
  frame: number;
  playing: boolean;
  total: number;
};
const captionCache = new Map<string, string>();
export function NativePreview({
  project,
  clock,
}: {
  project: Project;
  clock: PlaybackClock;
}) {
  useLanguage();
  const canvas = useRef<HTMLCanvasElement>(null);
  const [edge, setEdge] = useState("640");
  const [ready, setReady] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [error, setError] = useState("");
  const requestedSpec = previewSpec(project);
  const [settled, setSettled] = useState({
    id: project.id,
    spec: requestedSpec,
  });
  const spec = settled.id === project.id ? settled.spec : requestedSpec;
  // Keep editing responsive; rebuild once after a burst of property changes.
  useEffect(() => {
    const timer = setTimeout(
      () => setSettled({ id: project.id, spec: requestedSpec }),
      120,
    );
    return () => clearTimeout(timer);
  }, [requestedSpec, project.id]);
  useEffect(
    () => clock.subscribe(() => setPlaying(clock.getSnapshot().playing)),
    [clock],
  );
  useEffect(() => {
    const token = crypto.randomUUID();
    const doc = JSON.parse(spec);
    let alive = true,
      opened = false,
      polling = false;
    let timer: ReturnType<typeof setInterval> | undefined;
    let stopFrames: (() => void) | undefined;

    const fail = (e: unknown) => {
      if (alive) {
        setError(String(e));
        clock.acceptTransportState(clock.getSnapshot().time, false);
      }
    };
    const commands = new PreviewCommands(async ({ action, frame }) => {
      if (alive && opened)
        await bridge<void>("native_preview_control", { token, action, frame });
    }, fail);
    const command = commands.push;
    const detach = clock.attachTransport({
      play: () => command("play"),
      pause: () => command("pause"),
      seek: (t) => command("seek", Math.round(t * doc.fps)),
    });
    clock.ready = false;
    setReady(false);
    setError("");
    const load = async () => {
      if (!doc.clips.length && !doc.captions.length) return;
      doc.captions = await prepareCaptions(
        doc.captions,
        doc.width,
        doc.height,
        async (data) => {
          const key = `${project.id}:${data}`;
          if (captionCache.has(key)) return captionCache.get(key)!;
          const id = await bridge<string>("store_caption_image", {
            data,
            projectId: project.id,
          });
          captionCache.set(key, id);
          return id;
        },
      );
      if (!alive) return;
      await bridge("native_preview_open", {
        token,
        projectId: project.id,
        spec: doc,
        edge: Number(edge),
      });
      if (!alive) {
        await bridge("native_preview_control", { token, action: "close" });
        return;
      }
      opened = true;
      command("seek", Math.round(clock.getSnapshot().time * doc.fps));
      clock.ready = true;
      setReady(true);
      if (canvas.current)
        stopFrames = startPreviewFrames(canvas.current, token, fail);
      timer = setInterval(async () => {
        if (polling || !alive || commands.busy) return;
        const revision = commands.revision;
        polling = true;
        try {
          const s = await bridge<Status>("native_preview_status", { token });
          if (alive && revision === commands.revision && !commands.busy) {
            clock.acceptTransportState(
              !s.playing && s.frame >= s.total - 1
                ? clock.total
                : s.frame / doc.fps,
              s.playing,
            );
          }
        } catch (e) {
          fail(e);
        } finally {
          polling = false;
        }
      }, 50);
    };
    void load().catch(fail);
    return () => {
      alive = false;
      commands.clear();
      clock.ready = true;
      detach();
      clearInterval(timer);
      stopFrames?.();
      void bridge("native_preview_control", { token, action: "close" }).catch(
        () => {},
      );
    };
  }, [spec, project.id, clock, edge]);
  return (
    <div className="preview">
      <header>
        <span>{t("成片预览")}</span>
        <div className="preview-selectors">
          <StudioSelect
            label={t("预览清晰度")}
            value={edge}
            onValueChange={setEdge}
            options={[
              { value: "640", label: t("流畅") },
              { value: "1280", label: t("清晰") },
            ]}
          />
        </div>
      </header>
      <div className="preview-stage">
        <canvas
          ref={canvas}
          className="preview-canvas"
          style={{ visibility: ready ? "visible" : "hidden" }}
        />
        {!ready && (
          <div className="preview-placeholder">
            <Play size={26} />
            <span>
              {project.clips.length
                ? t("正在准备预览…")
                : t("将素材或镜头加入时间线")}
            </span>
          </div>
        )}
      </div>
      {error && <ErrorNotice error={error} fallback="OPERATION_FAILED" />}
      <div className="preview-transport">
        <button
          title={t("回到起点 Home")}
          onClick={() => {
            clock.pause();
            clock.seek(0);
          }}
        >
          <SkipBack />
        </button>
        <button
          className="play-button"
          disabled={!ready || !!error}
          title={playing ? t("暂停 Space") : t("播放 Space")}
          onClick={clock.toggle}
        >
          {playing ? <Pause weight="fill" /> : <Play weight="fill" />}
        </button>
        <ClockReadout clock={clock} />
      </div>
    </div>
  );
}
