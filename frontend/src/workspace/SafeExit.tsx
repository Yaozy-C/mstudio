import { ErrorNotice } from "../errors/ErrorNotice";
import { AlertDialog } from "@radix-ui/themes";
import { useSafeExit } from "./useSafeExit";

export function ExitSaveDialog({
  error,
  busy,
  retry,
  dismiss,
}: ReturnType<typeof useSafeExit>) {
  return (
    <AlertDialog.Root
      open={!!error}
      onOpenChange={(open) => {
        if (!open) dismiss();
      }}
    >
      <AlertDialog.Content className="modal small" aria-busy={busy}>
        <AlertDialog.Title>暂时无法退出</AlertDialog.Title>
        <AlertDialog.Description>
          项目还没有保存成功。可以重试保存，或返回继续编辑。
        </AlertDialog.Description>
        <ErrorNotice error={error} fallback="SAVE_FAILED" />
        <footer>
          <AlertDialog.Cancel>
            <button type="button" disabled={busy}>
              返回编辑
            </button>
          </AlertDialog.Cancel>
          <button
            type="button"
            className="primary"
            disabled={busy}
            onClick={() => void retry()}
          >
            {busy ? "正在保存…" : "重试保存并退出"}
          </button>
        </footer>
      </AlertDialog.Content>
    </AlertDialog.Root>
  );
}

export function SafeExit() {
  return <ExitSaveDialog {...useSafeExit()} />;
}
