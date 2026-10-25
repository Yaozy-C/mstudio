import { t } from "../../i18n";
import type { Asset, Project } from "../../model";
import { importBrowserFile } from "../../assistant/fileImports";
import type { AttachmentDraft } from "../../assistant/useAttachments";
import { attachmentKey, MAX_ATTACHMENTS } from "../../assistant/attachments";
import { requestCreativeTask } from "../../creative/aiTasks";
import { imageReviewTask, type ImageComment } from "./annotations";

export function imageReviewController(
  get: () => Project,
  change: (fn: (p: Project) => Project) => void,
  draft: Pick<AttachmentDraft, "items">,
  dependencies = {
    importFile: importBrowserFile,
    request: requestCreativeTask,
  },
) {
  return {
    projectId: get().id,
    hasAsset: (id: string) => get().assets.some((a) => a.id === id),
    submit: async (asset: Asset, file: File, comments: ImageComment[]) => {
      const project = get();
      const original = project.assets.find(
        (a) => a.id === asset.id && a.kind === "image" && !a.missing,
      );
      if (!original) throw new Error(t("原图已移除，请重新打开图片"));
      if (
        original.path !== asset.path ||
        original.imageVersionId !== asset.imageVersionId
      )
        throw new Error(t("参考版本已变更，请重新打开图片后提交批注"));
      const keys = new Set([
        ...draft.items.map(attachmentKey),
        `asset:${asset.id}`,
      ]);
      if (keys.size > MAX_ATTACHMENTS)
        throw new Error(t("本轮附件已满，请先移除部分附件再提交批注"));
      const marked = {
        ...(await dependencies.importFile(project.id, file, () => {})),
        annotationSourceId: original.id,
      };
      // The project may have changed while the file was importing.
      if (
        get().id !== project.id ||
        !get().assets.some((a) => a.id === original.id)
      )
        throw new Error(t("原图已移除，请重新打开图片"));
      if (
        !get().assets.some(
          (a) =>
            a.id === original.id &&
            a.path === original.path &&
            a.imageVersionId === original.imageVersionId,
        )
      )
        throw new Error(t("参考版本已变更，请重新打开图片后提交批注"));
      change((p) => ({
        ...p,
        assets: p.assets.some((a) => a.id === marked.id)
          ? p.assets.map((a) => (a.id === marked.id ? marked : a))
          : [...p.assets, marked],
      }));
      dependencies.request(imageReviewTask(original, marked, comments));
    },
  };
}
