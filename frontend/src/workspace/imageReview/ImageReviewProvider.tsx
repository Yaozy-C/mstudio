import type { ReactNode } from "react";
import type { Asset, Project } from "../../model";
import { bridge } from "../../bridge";
import type { AttachmentDraft } from "../../assistant/useAttachments";
import { ImageReviewContext, type ImageVersion } from "./ImageReviewContext";
import { imageReviewController } from "./imageReviewController";
import {
  uncollectAsset,
  unconfirmStoryAsset,
  isStoryAsset,
} from "../assetLibrary";

export function ImageReviewProvider({
  workspace,
  draft,
  children,
}: {
  workspace: {
    get: () => Project;
    change: (fn: (p: Project) => Project) => void;
    flush: () => Promise<void>;
  };
  draft: AttachmentDraft;
  children: ReactNode;
}) {
  return (
    <ImageReviewContext.Provider
      value={{
        ...imageReviewController(workspace.get, workspace.change, draft),
        versions: (imageId) =>
          bridge<ImageVersion[]>("list_image_versions", {
            projectId: workspace.get().id,
            imageId,
          }),
        clearReference: async (assetId) => {
          workspace.change((p) =>
            p.assets.some((a) => a.id === assetId && isStoryAsset(a))
              ? unconfirmStoryAsset(p, assetId)
              : uncollectAsset(p, assetId),
          );
          await workspace.flush();
        },
        selectReference: async (assetId, versionId) => {
          await workspace.flush();
          const asset = await bridge<Asset>("select_image_reference", {
            projectId: workspace.get().id,
            assetId,
            versionId,
          });
          workspace.change((p) => ({
            ...p,
            assets: p.assets.map((a) =>
              a.id === assetId ? { ...a, ...asset } : a,
            ),
          }));
          return asset;
        },
        process: async (asset, operation, name, versionId) => {
          await workspace.flush();
          const projectId = workspace.get().id;
          const receipt = await bridge<{ versions: ImageVersion[] }>(
            "process_image",
            {
              projectId,
              request: {
                inputAssetId: asset.id,
                inputVersionId: versionId,
                name,
                operation,
              },
            },
          );
          const saved = await bridge<Project>("get_project", { projectId });
          const logical = saved.assets.find((a) => a.id === asset.id);
          workspace.change((p) => ({
            ...p,
            assets: p.assets.map((a) =>
              a.id === asset.id && logical ? { ...a, ...logical } : a,
            ),
          }));
          return receipt.versions[0];
        },
      }}
    >
      {children}
    </ImageReviewContext.Provider>
  );
}
