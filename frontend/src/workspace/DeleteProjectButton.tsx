import { Trash as DialogTrash, X as DialogX } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useRef, useState } from "react";
import { AlertDialog, DropdownMenu } from "@radix-ui/themes";
import { Trash, DotsThree, FolderOpen } from "@phosphor-icons/react";
import type { ProjectEntry } from "../model";

export function DeleteProjectButton({
  project,
  onDelete,
  onOpen,
}: {
  project: Pick<ProjectEntry, "id" | "name">;
  onDelete: (id: string) => Promise<void>;
  onOpen: () => void;
}) {
  useLanguage();
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const pending = useRef(false);
  const trigger = useRef<HTMLButtonElement>(null);
  async function remove() {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setError("");
    try {
      await onDelete(project.id);
      setOpen(false);
    } catch (e) {
      setError(t("删除失败：{v0}", { v0: String(e) }));
    } finally {
      pending.current = false;
      setBusy(false);
    }
  }
  return (
    <AlertDialog.Root
      open={open}
      onOpenChange={(next) => {
        if (pending.current) return;
        setError("");
        setOpen(next);
      }}
    >
      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          <button
            ref={trigger}
            type="button"
            className="icon-button project-actions-trigger"
            title={t("项目操作")}
            aria-label={t("项目操作 {v0}", { v0: project.name })}
          >
            <DotsThree />
          </button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content
          className="studio-menu project-actions-menu"
          align="end"
          sideOffset={6}
        >
          <DropdownMenu.Item onSelect={onOpen}>
            <FolderOpen />
            {t("打开项目")}
          </DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item color="red" onSelect={() => setOpen(true)}>
            <Trash />
            {t("删除项目")}
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
      <AlertDialog.Content
        className="modal small"
        aria-busy={busy}
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          if (trigger.current?.isConnected) trigger.current.focus();
        }}
      >
        <AlertDialog.Title>
          <DialogTrash aria-hidden="true" />
          {t("删除项目")}
        </AlertDialog.Title>
        <AlertDialog.Description>
          {t("删除「")}
          {project.name}
          {t(
            "」及其素材、生成结果和对话记录？此操作无法撤销。其他项目共用的素材和导入前的原文件会保留。",
          )}
        </AlertDialog.Description>
        {error && <ErrorNotice error={error} fallback="OPERATION_FAILED" />}
        <footer>
          <AlertDialog.Cancel>
            <button type="button" disabled={busy}>
              <DialogX aria-hidden="true" />
              {t("取消")}
            </button>
          </AlertDialog.Cancel>
          <button
            type="button"
            className="primary danger"
            disabled={busy}
            onClick={() => void remove()}
          >
            <DialogTrash aria-hidden="true" />
            {busy ? t("正在删除…") : t("删除项目")}
          </button>
        </footer>
      </AlertDialog.Content>
    </AlertDialog.Root>
  );
}
