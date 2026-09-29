import type { WorkContext } from "./workContext";
import { TaskTarget } from "./TaskTarget";
import { AttachmentChips } from "./AttachmentChips";
import { AttachmentSupport } from "./AttachmentSupport";
import { type Attachment } from "./attachments";
import type { Project } from "../model";
import type { AttachmentDraft } from "./useAttachments";
import type { ModelConnection } from "../models/types";
export function ComposerContext({
  project,
  work,
  draft,
  attachments,
  model,
  settings,
}: {
  project: Project;
  work?: WorkContext;
  draft: AttachmentDraft;
  attachments: Attachment[];
  model?: ModelConnection;
  settings: () => void;
}) {
  const refs = attachments.filter(
    (a) =>
      a.kind === "node" &&
      project.nodes.some(
        (n) => n.id === a.id && (n.kind === "screenplay" || n.kind === "shot"),
      ),
  );
  const id =
    draft.targetNodeId ??
    (refs.length === 1
      ? refs[0].id
      : draft.omitWork
        ? null
        : work?.screenplayId);
  const paragraph =
    !draft.omitWork && id === work?.screenplayId && work?.paragraphId
      ? project.nodes
          .find((n) => n.id === id)
          ?.screenplay?.script?.find((s) => s.id === work.paragraphId)?.title
      : undefined;
  return (
    <>
      <TaskTarget
        project={project}
        id={id}
        remove={id ? () => draft.dismissTarget(id) : undefined}
        label={paragraph}
      />
      <AttachmentChips
        items={attachments}
        project={project}
        remove={draft.remove}
      />
      <AttachmentSupport
        items={attachments}
        model={model}
        settings={settings}
      />
    </>
  );
}
