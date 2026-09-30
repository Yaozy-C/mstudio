import { PlaybackSpeed } from "./PlaybackSpeed";
import { t, useLanguage } from "../i18n";
import { useEffect, useRef, useState } from "react";
import { Play, Pause, SkipBack, ArrowClockwise } from "@phosphor-icons/react";
import { bridge } from "../bridge";
import type { Project } from "../model";
import type { PlaybackClock } from "./clock";
import { prepareProjectCaptions } from "../creation/prepareProjectCaptions";
import { ClockReadout } from "./ClockReadout";
import { previewSpec } from "./previewSpec";
import { startPreviewFrames } from "./previewFrames";
import {
  NativePreviewController,
  type PreviewStatus,
} from "./nativePreviewController";
import { ActionButton } from "../ui/ActionButton";
import { ErrorNotice } from "../errors/ErrorNotice";
export function NativePreview({
  project,
  clock,
}: {
  project: Project;
  clock: PlaybackClock;
}) {
  useLanguage();
  const canvas = useRef<HTMLCanvasElement>(null);
  const [ready, setReady] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [error, setError] = useState("");
  const [retry, setRetry] = useState(0);
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
    let stopFrames: (() => void) | undefined;
    setReady(false);
    setError("");
    const controller = new NativePreviewController(
      clock,
      doc.fps,
      {
        open: async () => {
          doc.captions = await prepareProjectCaptions(
            { ...doc, id: project.id },
            () => controller.phase === "closed",
          );
          if (controller.phase === "closed") return;
          await bridge("native_preview_open", {
            token,
            projectId: project.id,
            spec: doc,
            edge: 0,
          });
        },
        control: (command) =>
          bridge<PreviewStatus>("native_preview_control", { token, command }),
        status: () => bridge<PreviewStatus>("native_preview_status", { token }),
      },
      (phase, error) => {
        if (phase === "ready") setReady(true);
        if (phase === "error") {
          setError(String(error));
          stopFrames?.();
        }
      },
    );
    if (doc.clips.length || doc.captions.length) {
      if (canvas.current)
        stopFrames = startPreviewFrames(
          canvas.current,
          (last) =>
            bridge<ArrayBuffer>("native_preview_frame", { token, last }),
          controller.fail,
          controller.acceptFrame,
          controller.presented,
        );
      void controller.start();
    }
    return () => {
      stopFrames?.();
      controller.dispose();
    };
  }, [spec, project.id, clock, retry]);
  return (
    <div className="preview">
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
      {error && (
        <>
          <ErrorNotice error={error} fallback="PREVIEW_FAILED" />
          <ActionButton
            icon={ArrowClockwise}
            onClick={() => setRetry((n) => n + 1)}
          >
            {t("重新加载预览")}
          </ActionButton>
        </>
      )}
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
        <PlaybackSpeed clock={clock} />
      </div>
    </div>
  );
}
