import { bridge, mediaUrl } from "../bridge";
import { t } from "../i18n";
import { isStale } from "./types";
import type { PrevisEditorState } from "./usePrevisEditor";

export function PrevisExports({ state }: { state: PrevisEditorState }) {
  const {
    exports,
    project,
    workspace,
    busy,
    share,
    setError,
    attachReference,
    fileId,
  } = state;
  return (
    <>
      {!!exports.length && (
        <section className="previs-exports">
          <h3>{t("预演视频")}</h3>
          {exports.map((video) => {
            const shared =
              !!video.sharedAssetId &&
              project.assets.some((a) => a.id === video.sharedAssetId);
            return (
              <article key={video.id}>
                <video
                  src={mediaUrl(video.asset.path)}
                  controls
                  preload="metadata"
                  playsInline
                />
                <div>
                  <strong>{video.asset.name}</strong>
                  {isStale(video, workspace.files) && (
                    <span className="previs-stale">{t("场景已有修改")}</span>
                  )}
                  <label>
                    <input
                      type="checkbox"
                      checked={shared}
                      disabled={!!busy}
                      onChange={(e) => void share(video, e.target.checked)}
                    />
                    {t("提供给 Agent")}
                  </label>
                  <small>
                    {shared
                      ? t("已加入普通项目素材，可手动引用。")
                      : t("仅在预演区可见。")}
                  </small>
                  <button
                    type="button"
                    disabled={!!busy}
                    onClick={() =>
                      void bridge("save_export", {
                        path: video.asset.path,
                      }).catch((e) => setError(String(e)))
                    }
                  >
                    {t("保存视频到…")}
                  </button>
                  <button
                    type="button"
                    disabled={!!busy || !shared}
                    onClick={() =>
                      attachReference(fileId, video.sharedAssetId!)
                    }
                  >
                    {t("设为镜头动作参考")}
                  </button>
                </div>
              </article>
            );
          })}
        </section>
      )}
    </>
  );
}
