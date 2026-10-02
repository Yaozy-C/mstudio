import { AsyncButton, StatusMessage } from "../ui/AsyncState";
import { progressRatio } from "./exportProgress";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { X, Export as ExportIcon } from "@phosphor-icons/react";
import type { Project } from "../model";
import { formatTime } from "../model";
import { endTime } from "../timeline/document";
import { prepareProjectCaptions } from "../creation/prepareProjectCaptions";
import { bridge, mediaUrl, native } from "../bridge";
import { runtime } from "../plugins/runtime";
export function ExportDialog({
  project,
  onClose,
}: {
  project: Project;
  onClose: () => void;
}) {
  useLanguage();
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);
  const rendering = useRef(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [path, setPath] = useState("");
  const [saved, setSaved] = useState("");
  useEffect(() => {
    if (!native) return;
    const off = listen<{ done: number; total: number }>(
      "render-progress",
      (e) => {
        if (rendering.current) setProgress(progressRatio(e.payload));
      },
    );
    return () => {
      void off.then((f) => f());
    };
  }, []);
  async function render() {
    if (busy) return;
    setBusy(true);
    setError("");
    setProgress(null);
    try {
      const captions = await prepareProjectCaptions(project);
      rendering.current = true;
      const result = await runtime.execute<{ path: string }>("render_video", {
        projectId: project.id,
        spec: {
          tracks: project.tracks,
          captions,
          clips: project.clips,
          width: project.width,
          height: project.height,
          fps: project.fps,
        },
      });
      setPath(result.path);
    } catch (e) {
      setError(String(e));
    } finally {
      rendering.current = false;
      setBusy(false);
    }
  }
  async function save() {
    if (saving) return;
    setSaving(true);
    setError("");
    try {
      const target = await bridge<string | null>("save_export", { path });
      if (target) setSaved(target);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }
  return (
    <div className="modal-backdrop">
      <section className="modal export-modal">
        <header>
          <div>
            <h2>{path ? t("成片已就绪") : t("导出成片")}</h2>
          </div>
          <button
            className="icon-button"
            aria-label={t("关闭")}
            disabled={busy || saving}
            onClick={onClose}
          >
            <X />
          </button>
        </header>
        {path ? (
          <video controls src={mediaUrl(path)} />
        ) : (
          <div className="export-summary">
            <ExportIcon size={40} />
            <h3>{project.name}</h3>
            <p>
              {project.width} × {project.height} · {project.fps} fps ·{" "}
              {formatTime(endTime(project))}
            </p>
            <p>{t("H.264 / MP4 · AAC 立体声")}</p>
          </div>
        )}
        {busy && (
          <>
            {progress !== null && (
              <div
                className="progress"
                role="progressbar"
                aria-label={t("正在本机渲染")}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={Math.round(progress * 100)}
              >
                <span style={{ width: `${progress * 100}%` }} />
              </div>
            )}
            <StatusMessage>
              {progress === null
                ? t("正在准备导出…")
                : `${t("正在本机渲染")} ${Math.round(progress * 100)}${t("% · 请保持应用打开")}`}
            </StatusMessage>
          </>
        )}
        {error && (
          <ErrorNotice
            error={error}
            fallback={path ? "EXPORT_SAVE_FAILED" : "EXPORT_FAILED"}
          />
        )}
        {saved && (
          <StatusMessage kind="success">
            {t("已保存至")} {saved}
          </StatusMessage>
        )}
        <footer>
          {path ? (
            <AsyncButton
              className="primary"
              busy={saving}
              busyLabel={t("保存中…")}
              onClick={() => void save()}
            >
              {t("另存为 MP4")}
            </AsyncButton>
          ) : (
            <AsyncButton
              busy={busy}
              busyLabel={t("正在渲染…")}
              className="primary"
              disabled={busy || !project.clips.length}
              onClick={() => void render()}
            >
              <ExportIcon />
              {error ? t("重试导出") : t("开始导出")}
            </AsyncButton>
          )}
        </footer>
      </section>
    </div>
  );
}
