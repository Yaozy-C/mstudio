import {
  Check as DialogCheck,
  FloppyDisk as DialogFloppyDisk,
} from "@phosphor-icons/react";
import { CheckCircle, FolderOpen } from "@phosphor-icons/react";
import { AsyncButton } from "../ui/AsyncState";
import { t, useLanguage } from "../i18n";

export function ExportResult({
  saved,
  saving,
  revealing,
  onSave,
  onReveal,
  onClose,
}: {
  saved: string;
  saving: boolean;
  revealing: boolean;
  onSave: () => void;
  onReveal: () => void;
  onClose: () => void;
}) {
  useLanguage();
  const separator = Math.max(saved.lastIndexOf("/"), saved.lastIndexOf("\\"));
  const filename = saved.slice(separator + 1);
  const directory = saved.slice(0, separator + 1);
  const folder = saved.slice(0, separator).split(/[\\/]/).pop() || directory;
  return (
    <>
      {saved ? (
        <div className="export-saved-file">
          <CheckCircle size={24} aria-hidden="true" />
          <div className="export-file-details" role="status" aria-atomic="true">
            <strong>{t("视频已保存")}</strong>
            <span className="export-filename" title={filename}>
              {filename}
            </span>
            <small
              title={directory}
              aria-label={t("保存位置：{v0}", { v0: directory })}
            >
              {t("保存位置：{v0}", { v0: folder })}
            </small>
          </div>
          <AsyncButton
            busy={revealing}
            disabled={saving}
            busyLabel={t("正在打开…")}
            onClick={onReveal}
          >
            <FolderOpen size={18} />
            {t("打开所在文件夹")}
          </AsyncButton>
        </div>
      ) : (
        <p className="export-save-hint">
          {t("预览确认后，将 MP4 保存到你选择的位置。")}
        </p>
      )}
      <footer>
        <AsyncButton
          className={saved ? "" : "primary"}
          busy={saving}
          disabled={revealing}
          busyLabel={t("保存中…")}
          onClick={onSave}
        >
          <DialogFloppyDisk aria-hidden="true" />
          {saved ? t("另存一份…") : t("保存 MP4…")}
        </AsyncButton>
        {saved && (
          <button
            className="primary"
            disabled={saving || revealing}
            onClick={onClose}
          >
            <DialogCheck aria-hidden="true" />
            {t("完成")}
          </button>
        )}
      </footer>
    </>
  );
}
