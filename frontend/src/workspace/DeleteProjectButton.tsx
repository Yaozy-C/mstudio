import { ErrorNotice } from "../errors/ErrorNotice";
import { useRef, useState } from "react";
import { AlertDialog } from "@radix-ui/themes";
import { Trash } from "@phosphor-icons/react";
import type { ProjectEntry } from "../model";

export function DeleteProjectButton({
  project,
  onDelete,
}: {
  project: Pick<ProjectEntry, "id" | "name">;
  onDelete: (id: string) => Promise<void>;
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const pending = useRef(false);
  async function remove() {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setError("");
    try {
      await onDelete(project.id);
      setOpen(false);
    } catch (e) {
      setError(`删除失败：${String(e)}`);
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
      <AlertDialog.Trigger>
        <button
          type="button"
          className="icon-button"
          title="删除项目"
          aria-label={`删除项目 ${project.name}`}
        >
          <Trash />
        </button>
      </AlertDialog.Trigger>
      <AlertDialog.Content className="modal small" aria-busy={busy}>
        <AlertDialog.Title>删除项目</AlertDialog.Title>
        <AlertDialog.Description>
          删除「{project.name}
          」及其素材、生成结果和对话记录？此操作无法撤销。其他项目共用的素材和导入前的原文件会保留。
        </AlertDialog.Description>
        {error && <ErrorNotice error={error} fallback="OPERATION_FAILED" />}
        <footer>
          <AlertDialog.Cancel>
            <button type="button" disabled={busy}>
              取消
            </button>
          </AlertDialog.Cancel>
          <button
            type="button"
            className="primary"
            disabled={busy}
            onClick={() => void remove()}
          >
            {busy ? "正在删除…" : "删除项目"}
          </button>
        </footer>
      </AlertDialog.Content>
    </AlertDialog.Root>
  );
}
