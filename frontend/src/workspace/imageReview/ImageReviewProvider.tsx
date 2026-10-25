import type { ReactNode } from "react";
import type { Asset, Project } from "../../model";
import { bridge } from "../../bridge";
import type { AttachmentDraft } from "../../assistant/useAttachments";
import { ImageReviewContext } from "./ImageReviewContext";
import { imageReviewController } from "./imageReviewController";

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
        process: async (asset, operation, name) => {
          await workspace.flush();
          const projectId = workspace.get().id;
          const receipt = await bridge<{ assets: Asset[] }>("process_image", {
            projectId,
            request: { inputAssetId: asset.id, name, operation },
          });
          const assets = receipt.assets.map((a) => ({ ...a, inLibrary: true }));
          workspace.change((p) => ({
            ...p,
            assets: [
              ...p.assets.filter((a) => !assets.some((v) => v.id === a.id)),
              ...assets,
            ],
          }));
          return assets[0];
        },
      }}
    >
      {children}
    </ImageReviewContext.Provider>
  );
}
