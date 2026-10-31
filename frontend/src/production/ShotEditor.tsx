import { ShotReferences } from "./ShotReferences";
import { editReferences, mergeReferenceDraft } from "./editReferences";
import "./shot-editor.css";
import { ActionButton } from "../ui/ActionButton";
import { t, useLanguage } from "../i18n";
import { actionText, editShotText } from "../creative/shotText";
import { useState } from "react";
import { Dialog } from "@radix-ui/themes";
import { X, FloppyDisk, FilmSlate } from "@phosphor-icons/react";
import type { BoardNode, Project } from "../model";
import type { ChangeProject } from "./types";
export function ShotEditor({
  node,
  project,
  change,
  close,
}: {
  node: BoardNode;
  project: Project;
  change: ChangeProject;
  close: () => void;
}) {
  useLanguage();
  const [text, setText] = useState(actionText(node));
  const [dialogue, setDialogue] = useState(node?.shot?.dialogue ?? "");
  const [referenceDraft, setReferenceDraft] = useState(() => ({
    base: node?.references ?? [],
    value: node?.references ?? [],
  }));
  const initialReferences = node?.references ?? [];
  const references = mergeReferenceDraft(
    referenceDraft.base,
    referenceDraft.value,
    initialReferences,
  );
  const [error, setError] = useState("");
  const save = () =>
    change((p) =>
      node?.shot
        ? editShotText(
            editShotText(
              editReferences(p, node.id, initialReferences, references),
              node.id,
              "action",
              text,
            ),
            node.id,
            "dialogue",
            dialogue,
          )
        : {
            ...p,
            nodes: p.nodes.map((n) => (n.id === node?.id ? { ...n, text } : n)),
          },
    );
  return (
    <Dialog.Root
      open
      onOpenChange={(v) => {
        if (!v) close();
      }}
    >
      <Dialog.Content
        className="media-preview-dialog canvas-preview shot-editor-dialog"
        aria-describedby={undefined}
      >
        <header>
          <Dialog.Title>
            <FilmSlate size={22} />
            {node.title}
          </Dialog.Title>
          <Dialog.Close>
            <button aria-label={t("关闭预览")}>
              <X />
            </button>
          </Dialog.Close>
        </header>
        <form
          className="shot-editor-form"
          onSubmit={(e) => {
            e.preventDefault();
            try {
              save();
              close();
            } catch (error) {
              setError(String(error));
            }
          }}
        >
          <div className="shot-editor-fields">
            <label>
              {t("镜头画面与动作")}
              <textarea
                value={text}
                onChange={(e) => setText(e.target.value)}
                rows={6}
              />
            </label>
            {node?.shot && (
              <label>
                {t("台词 / 旁白")}
                <textarea
                  value={dialogue}
                  onChange={(e) => setDialogue(e.target.value)}
                  rows={3}
                />
              </label>
            )}
            {node?.shot && (
              <ShotReferences
                project={project}
                references={references}
                onChange={(value) =>
                  setReferenceDraft({ base: initialReferences, value })
                }
              />
            )}
          </div>
          {error && <p role="alert">{error}</p>}
          <footer className="shot-editor-actions">
            <ActionButton icon={X} onClick={close}>
              {t("取消")}
            </ActionButton>
            <button className="primary" type="submit">
              <FloppyDisk size={18} />
              {t("保存镜头内容")}
            </button>
          </footer>
        </form>
      </Dialog.Content>
    </Dialog.Root>
  );
}
