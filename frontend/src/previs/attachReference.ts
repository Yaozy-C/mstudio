import { t } from "../i18n";
import type { Asset, Project } from "../model";

export function withPrevisReference(
  project: Project,
  fileId: string,
  asset: Asset,
): Project {
  if (!project.nodes.some((node) => node.id === fileId && node.shot))
    throw Error(t("镜头已删除"));
  if (
    asset.kind !== "video" ||
    asset.missing ||
    asset.discarded ||
    !Number.isFinite(asset.duration) ||
    asset.duration <= 0
  )
    throw Error(t("预演视频不可用，请重新生成。"));
  return {
    ...project,
    assets: project.assets.some((a) => a.id === asset.id)
      ? project.assets
      : [...project.assets, asset],
    nodes: project.nodes.map((node) =>
      node.id === fileId &&
      node.shot &&
      !node.references?.some((ref) => ref.assetId === asset.id)
        ? {
            ...node,
            references: [
              ...(node.references ?? []),
              {
                assetId: asset.id,
                purpose: "预演动作、构图与运镜参考；商品外观以实物参考为准",
                start: 0,
                end: asset.duration,
              },
            ],
          }
        : node,
    ),
  };
}

/** Import the media copy and save its reference in one user action; retries reuse the copy. */
export async function attachPrevisVideo({
  fileId,
  videoId,
  share,
  change,
  flush,
  refresh,
}: {
  fileId: string;
  videoId: string;
  share: (videoId: string) => Promise<{ asset: Asset }>;
  change: (fn: (project: Project) => Project) => void;
  flush: () => Promise<unknown>;
  refresh: () => Promise<void>;
}) {
  await flush();
  const { asset } = await share(videoId);
  if (!asset) throw Error(t("预演视频不可用，请重新生成。"));
  change((project) => withPrevisReference(project, fileId, asset));
  await flush();
  await refresh();
}
