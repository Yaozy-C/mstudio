import { useEffect, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowUpRight,
  CaretDown,
  ClockCounterClockwise,
  CheckCircle,
} from "@phosphor-icons/react";
import { mediaUrl, native, bridge } from "../../bridge";
import { t, useLanguage } from "../../i18n";
import {
  emptyCatalog,
  readyModel,
  type ModelCatalog,
} from "../../models/types";
import { AnalysisHistory } from "./AnalysisHistory";
import { useAnalysisHistory } from "./useAnalysisHistory";
import { AnalysisProgress } from "./AnalysisProgress";
import { ErrorNotice } from "../../errors/ErrorNotice";
import { AnalysisResults } from "./AnalysisResults";
import { AnalysisEntry } from "./AnalysisEntry";
import { AnalysisRequest } from "./AnalysisRequest";
import { SourcePicker } from "./SourcePicker";
import { timecode, type AnalysisState } from "./types";
import type { AnalysisController } from "./controller";
import "./video-analysis.css";

export function VideoAnalysis({
  state,
  controller,
  onBrowse,
  onSettings,
  onCreate,
}: {
  state: AnalysisState;
  controller: AnalysisController;
  onBrowse: () => void;
  onSettings: () => void;
  onCreate?: () => Promise<void>;
}) {
  const language = useLanguage();
  const main = useRef<HTMLElement>(null);
  const player = useRef<HTMLVideoElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const [catalog, setCatalog] = useState<ModelCatalog>(emptyCatalog);
  const [modelId, setModelId] = useState("");
  const [modelError, setModelError] = useState("");
  const [playbackError, setPlaybackError] = useState("");
  const [currentTime, setCurrentTime] = useState(0);
  const busy = !!state.busy || !!state.openingRecord;
  const history = useAnalysisHistory(controller, state.recordId);
  const hasSource = !!(state.video || state.work);
  const models = catalog.profiles.filter(
    (model) => model.inputs.video && readyModel(model),
  );
  const selected = models.some(
    (model) => model.id === (modelId || state.restoredModelId),
  )
    ? modelId || state.restoredModelId!
    : ((models.find((model) => model.id === catalog.defaultId) ?? models[0])
        ?.id ?? "");
  useEffect(() => {
    setModelId("");
  }, [state.recordId]);
  useEffect(() => {
    heading.current?.focus();
  }, []);
  useEffect(() => {
    if (!native) return;
    let active = true;
    void bridge<ModelCatalog>("model_catalog").then(
      (value) => {
        if (active) setCatalog(value);
      },
      (error) => {
        if (active) setModelError(String(error));
      },
    );
    return () => {
      active = false;
    };
  }, []);
  useEffect(() => {
    setCurrentTime(0);
    setPlaybackError("");
    main.current?.scrollTo({ top: 0 });
  }, [state.video?.id, state.work?.id]);
  const seek = (time: number) => {
    if (player.current) {
      player.current.currentTime = time;
      setCurrentTime(time);
      player.current.focus();
    }
  };
  return (
    <main
      ref={main}
      className={`library-main video-analysis-page${hasSource ? " has-source" : ""}`}
    >
      <header className="library-heading analysis-page-heading">
        <h1 ref={heading} tabIndex={-1}>
          {t("视频拆解")}
        </h1>
        <div className="analysis-page-actions">
          <button
            className="analysis-text-action"
            onClick={() => history.setOpen(true)}
            disabled={busy}
          >
            <ClockCounterClockwise size={17} aria-hidden="true" />
            {t("拆解记录")}
          </button>
          {hasSource && (
            <button
              className="analysis-text-action"
              disabled={busy}
              onClick={controller.clear}
            >
              <ArrowLeft size={17} aria-hidden="true" />
              {t("重新选择")}
            </button>
          )}
        </div>
      </header>
      {!hasSource ? (
        <AnalysisEntry
          controller={controller}
          busy={busy}
          onBrowse={onBrowse}
        />
      ) : (
        <div className="analysis-workspace">
          <section className="analysis-source" aria-label={t("参考视频")}>
            <header className="analysis-source-heading">
              <h2>{state.work?.title ?? state.video?.name}</h2>
              <p>
                {state.work ? state.work.credit : t("你的参考视频")}
                {state.video &&
                  state.video.duration > 0 &&
                  ` · ${timecode(state.video.duration)}`}
              </p>
            </header>
            <div className="analysis-player">
              {state.video?.missing ? (
                <div className="analysis-video-missing">
                  {t("原视频暂不可用，拆解结果仍可查看。")}
                </div>
              ) : state.video ? (
                <video
                  ref={player}
                  key={state.video.id}
                  src={mediaUrl(state.video.path)}
                  controls
                  playsInline
                  preload="metadata"
                  onTimeUpdate={(event) =>
                    setCurrentTime(event.currentTarget.currentTime)
                  }
                  onLoadedMetadata={(event) => {
                    const video = event.currentTarget;
                    if (!native)
                      controller.previewMetadata(
                        state.video!.id,
                        video.duration,
                        video.videoWidth,
                        video.videoHeight,
                      );
                  }}
                  onError={() =>
                    setPlaybackError(
                      t("无法播放此视频，请尝试 MP4、MOV 或 WebM 文件。"),
                    )
                  }
                  aria-label={state.work?.title ?? state.video.name}
                />
              ) : (
                state.work && (
                  <iframe
                    src={state.work.embed}
                    title={t("完整作品 {title}", { title: state.work.title })}
                    allow="autoplay; fullscreen; picture-in-picture"
                    allowFullScreen
                    referrerPolicy="strict-origin-when-cross-origin"
                  />
                )
              )}
            </div>
            <div className="analysis-source-toolbar">
              <details className="analysis-replace">
                <summary>
                  {t("更换视频")}
                  <CaretDown size={12} aria-hidden="true" />
                </summary>
                <SourcePicker controller={controller} busy={busy} compact />
              </details>
              <div>
                {state.work ? (
                  <a href={state.work.source} target="_blank" rel="noreferrer">
                    {t("作品来源")}
                    <ArrowUpRight size={14} aria-hidden="true" />
                  </a>
                ) : (
                  state.video &&
                  state.video.width > 0 && (
                    <span>
                      {state.video.width} × {state.video.height}
                    </span>
                  )
                )}
                {state.video && (
                  <span className="analysis-playhead">
                    {timecode(currentTime)} / {timecode(state.video.duration)}
                  </span>
                )}
              </div>
            </div>
            <AnalysisRequest
              state={state}
              controller={controller}
              models={models}
              selected={selected}
              select={setModelId}
              analyze={() =>
                void controller.analyze(
                  selected,
                  state.question,
                  language,
                  models.find((model) => model.id === selected)?.name,
                )
              }
              onSettings={onSettings}
            />
            <ErrorNotice error={modelError} fallback="MODEL_UNAVAILABLE" />
            <ErrorNotice error={playbackError} />
          </section>
          {state.busy && state.progress ? (
            <AnalysisProgress
              progress={state.progress}
              cancel={controller.cancel}
            />
          ) : (
            <div className="analysis-output">
              <ErrorNotice
                error={state.error}
                fallback="VIDEO_ANALYSIS_FAILED"
              />
              {(state.result || !state.error) && (
                <AnalysisResults
                  key={state.recordId ?? state.video?.id}
                  onHistory={() => history.setOpen(true)}
                  onCreate={onCreate}
                  result={state.result}
                  seek={seek}
                  currentTime={currentTime}
                />
              )}
              {state.recordId && (
                <p className="analysis-record-saved">
                  <CheckCircle size={15} aria-hidden="true" />
                  {t("已保存到拆解记录")}
                </p>
              )}
              {state.recordError && (
                <p role="status">
                  {t("拆解已完成，但记录未保存：{error}", {
                    error: state.recordError,
                  })}
                </p>
              )}
            </div>
          )}
        </div>
      )}
      {busy && !hasSource && state.progress && (
        <AnalysisProgress
          progress={state.progress}
          cancel={controller.cancel}
          standalone
        />
      )}
      {!hasSource && <ErrorNotice error={state.error || modelError} />}
      <AnalysisHistory
        history={history}
        controller={controller}
        state={state}
      />
    </main>
  );
}
