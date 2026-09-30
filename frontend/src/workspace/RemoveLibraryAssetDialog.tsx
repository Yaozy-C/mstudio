import { AlertDialog } from "@radix-ui/themes";
import { t } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import type { Asset } from "../model";
export function RemoveLibraryAssetDialog({
  removing,
  scope,
  disabled,
  error,
  onClose,
  onConfirm,
}: {
  removing: Asset | null;
  scope: "project" | "global" | "effects";
  disabled: boolean;
  error: string;
  onClose: () => void;
  onConfirm: () => Promise<void>;
}) {
  return (
    <AlertDialog.Root
      open={!!removing}
      onOpenChange={(open) => {
        if (!open && !disabled) onClose();
      }}
    >
      <AlertDialog.Content className="modal small" aria-busy={disabled}>
        <AlertDialog.Title>
          {t("移除")}
          {scope === "project" ? t("项目") : t("公共")}
          {t("素材")}
        </AlertDialog.Title>
        <AlertDialog.Description>
          {t("将「")}
          {removing?.name}
          {t("」移出")}
          {scope === "project" ? t("项目") : t("公共")}
          {t("素材库？已用于分镜、时间线和对话的内容会保留")}
          {scope === "project"
            ? t("，公共素材不受影响")
            : t("，其他项目中已选用的素材不受影响")}
          。
        </AlertDialog.Description>
        {error && <ErrorNotice error={error} fallback="OPERATION_FAILED" />}
        <footer>
          <AlertDialog.Cancel>
            <button disabled={disabled}>{t("取消")}</button>
          </AlertDialog.Cancel>
          <button className="primary" disabled={disabled} onClick={onConfirm}>
            {disabled ? t("正在移除…") : t("确认移除")}
          </button>
        </footer>
      </AlertDialog.Content>
    </AlertDialog.Root>
  );
}
