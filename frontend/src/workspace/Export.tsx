import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { X, Export as ExportIcon, CheckCircle } from "@phosphor-icons/react";
import type { Project } from "../model";
import { formatTime } from "../model";
import { endTime } from "../timeline/document";
import { prepareCaptions } from "../creation/prepareCaptions";
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
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState("");
  const [path, setPath] = useState("");
  const [saved, setSaved] = useState("");
  useEffect(() => {
    if (!native) return;
    const off = listen<{ done: number; total: number }>(
      "render-progress",
      (e) => setProgress(e.payload.done / e.payload.total),
    );
    return () => {
      void off.then((f) => f());
    };
  }, []);
  async function render() {
    setBusy(true);
    setError("");
    setProgress(0);
    try {
      const captions = await prepareCaptions(
        project.captions ?? [],
        project.width,
        project.height,
        (data) =>
          bridge<string>("store_caption_image", {
            data,
            projectId: project.id,
          }),
      );
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
      setBusy(false);
    }
  }
  return (
    <div className="modal-backdrop">
      <section className="modal export-modal">
        <header>
          <div>
            <h2>{path ? t("成片已就绪") : t("导出成片")}</h2>
          </div>
          <button className="icon-button" disabled={busy} onClick={onClose}>
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
            <div className="progress">
              <span style={{ width: `${progress * 100}%` }} />
            </div>
            <p className="subtle">
              {t("正在本机渲染")} {Math.round(progress * 100)}
              {t("% · 请保持应用打开")}
            </p>
          </>
        )}
        {error && (
          <ErrorNotice
            error={error}
            fallback={path ? "EXPORT_SAVE_FAILED" : "EXPORT_FAILED"}
          />
        )}
        {saved && (
          <p className="notice">
            <CheckCircle />
            {t("已保存至")} {saved}
          </p>
        )}
        <footer>
          {path ? (
            <button
              className="primary"
              onClick={() =>
                void bridge<string | null>("save_export", { path })
                  .then((p) => {
                    if (p) setSaved(p);
                  })
                  .catch((e) => setError(String(e)))
              }
            >
              {t("另存为 MP4")}
            </button>
          ) : (
            <button
              className="primary"
              disabled={busy || !project.clips.length}
              onClick={() => void render()}
            >
              <ExportIcon />
              {busy ? t("正在渲染…") : error ? t("重试导出") : t("开始导出")}
            </button>
          )}
        </footer>
      </section>
    </div>
  );
}
