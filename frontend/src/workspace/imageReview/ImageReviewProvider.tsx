import type { ReactNode } from "react";
import type { Project } from "../../model";
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
  };
  draft: AttachmentDraft;
  children: ReactNode;
}) {
  return (
    <ImageReviewContext.Provider
      value={imageReviewController(workspace.get, workspace.change, draft)}
    >
      {children}
    </ImageReviewContext.Provider>
  );
}
