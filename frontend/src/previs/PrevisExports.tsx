import { Check, Link } from "@phosphor-icons/react";
import { AsyncButton } from "../ui/AsyncState";
import { mediaUrl } from "../bridge";
import { t } from "../i18n";
import type { PrevisEditorState } from "./usePrevisEditor";

export function PrevisExports({ state }: { state: PrevisEditorState }) {
  const { exports, project, busy, addReference, linkingId, fileId } = state;
  if (!exports.length) return null;
  const target = project.nodes.find((node) => node.id === fileId && node.shot);
  return (
    <section className="previs-exports">
      <h3>{t("预演视频")}</h3>
      {exports.slice(0, 1).map((video) => {
        const linked =
          !!video.sharedAssetId &&
          !!target?.references?.some(
            (ref) => ref.assetId === video.sharedAssetId,
          );
        return (
          <article key={video.id}>
            <video
              key={video.asset.path}
              src={mediaUrl(video.asset.path)}
              controls
              preload="metadata"
              playsInline
            />
            <div>
              <strong>{video.asset.name}</strong>
              <AsyncButton
                className={linked ? "" : "primary"}
                busy={linkingId === video.id}
                busyLabel={t("正在设置参考…")}
                disabled={!!busy || linked || !target}
                onClick={() => void addReference(video)}
              >
                {linked ? <Check /> : <Link />}
                {linked ? t("已设为参考") : t("设为参考")}
              </AsyncButton>
            </div>
          </article>
        );
      })}
    </section>
  );
}
