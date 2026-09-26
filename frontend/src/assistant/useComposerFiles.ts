import { useState, type ClipboardEvent, type DragEvent } from "react";
import { native } from "../bridge";
import { transferFiles } from "./fileImports";
import type { AttachmentRef } from "./attachments";
import type { AttachmentDraft } from "./useAttachments";
export function useComposerFiles(
  draft: AttachmentDraft,
  receive?: (refs: AttachmentRef[]) => void,
) {
  const [dragging, setDragging] = useState(false);
  const isFileDrag = (e: DragEvent) =>
    Array.from(e.dataTransfer.types).some(
      (type) => type === "Files" || type === "application/x-mstudio-reference",
    );
  return {
    dragging,
    onPaste(e: ClipboardEvent) {
      const files = transferFiles(e.clipboardData);
      if (files.length) {
        e.preventDefault();
        e.stopPropagation();
        void draft.importDroppedFiles(files, receive);
      } else if (
        native &&
        (!e.clipboardData.getData("text/plain") ||
          (e.clipboardData.types.includes("text/uri-list") &&
            e.clipboardData.getData("text/uri-list").startsWith("file:")))
      ) {
        // Finder file copies may expose file URLs rather than browser File objects.
        e.preventDefault();
        e.stopPropagation();
        void draft.importFiles(receive, true);
      }
    },
    onDragOver(e: DragEvent) {
      if (!isFileDrag(e)) return;
      e.preventDefault();
      e.stopPropagation();
      e.dataTransfer.dropEffect = "copy";
      setDragging(true);
    },
    onDragLeave(e: DragEvent) {
      if (!e.currentTarget.contains(e.relatedTarget as Node | null))
        setDragging(false);
    },
    onDrop(e: DragEvent) {
      if (!isFileDrag(e)) return;
      e.preventDefault();
      e.stopPropagation();
      setDragging(false);
      const raw = e.dataTransfer.getData("application/x-mstudio-reference");
      if (raw) {
        try {
          const ref = JSON.parse(raw) as AttachmentRef;
          if (
            ["asset", "node", "clip"].includes(ref.kind) &&
            typeof ref.id === "string"
          )
            (receive ?? draft.add)([ref]);
        } catch {
          /* Ignore unrelated drag payloads. */
        }
        return;
      }
      void draft.importDroppedFiles(transferFiles(e.dataTransfer), receive);
    },
  };
}
