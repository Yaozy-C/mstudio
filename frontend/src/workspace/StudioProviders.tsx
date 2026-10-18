import type { ComponentProps } from "react";
import type { useProject } from "./useProject";
import { ImageReviewProvider } from "./imageReview/ImageReviewProvider";
import { PrevisProvider } from "../previs/PrevisProvider";

type Props = Omit<ComponentProps<typeof ImageReviewProvider>, "workspace"> & {
  workspace: ReturnType<typeof useProject>;
};
export function StudioProviders({ workspace, draft, children }: Props) {
  return (
    <PrevisProvider project={workspace.project} flush={workspace.flush}>
      <ImageReviewProvider workspace={workspace} draft={draft}>
        {children}
      </ImageReviewProvider>
    </PrevisProvider>
  );
}
