import { useState } from "react";
import { Dialog, DropdownMenu } from "@radix-ui/themes";
import { Plus, UploadSimple, Link } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import { bridge } from "../bridge";
import type { AttachmentRef } from "./attachments";
import type { AttachmentDraft } from "./useAttachments";
import "../styles/video-url-import.css";

export function AddAttachments({
  draft,
  receive,
  disabled,
}: {
  draft: AttachmentDraft;
  receive?: (refs: AttachmentRef[]) => void;
  disabled?: boolean;
}) {
  useLanguage();
  const [open, setOpen] = useState(false);
  const [url, setUrl] = useState("");
  const [error, setError] = useState("");
  const [downloadId, setDownloadId] = useState<string>();
  const [cancelling, setCancelling] = useState(false);
  const busy = !!downloadId;
  async function download() {
    if (!url.trim() || busy) return;
    const id = crypto.randomUUID();
    setDownloadId(id);
    setError("");
    const failure = await draft.importVideoUrl(url, id, receive);
    setDownloadId(undefined);
    setCancelling(false);
    if (failure) setError(failure);
    else {
      setOpen(false);
      setUrl("");
    }
  }
  return (
    <>
      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          <button
            type="button"
            className="composer-icon-button"
            aria-label={t("添加附件")}
            title={t("添加附件")}
            disabled={disabled || draft.busy}
          >
            <Plus size={20} />
          </button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="start">
          <DropdownMenu.Item onSelect={() => void draft.importFiles(receive)}>
            <UploadSimple size={16} />
            {t("从电脑添加附件")}
          </DropdownMenu.Item>
          <DropdownMenu.Item
            onSelect={() => {
              setError("");
              setOpen(true);
            }}
          >
            <Link size={16} />
            {t("从链接添加视频")}
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
      <Dialog.Root
        open={open}
        onOpenChange={(value) => {
          if (!busy) setOpen(value);
        }}
      >
        <Dialog.Content maxWidth="480px" className="video-url-import">
          <Dialog.Title>{t("从链接添加视频")}</Dialog.Title>
          <Dialog.Description>
            {t("粘贴视频直链或抖音、TikTok、YouTube 视频链接。")}
          </Dialog.Description>
          <label>
            <span>{t("视频链接")}</span>
            <input
              type="text"
              inputMode="url"
              autoFocus
              placeholder="https://…"
              value={url}
              disabled={busy}
              onChange={(e) => setUrl(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.nativeEvent.isComposing) {
                  e.preventDefault();
                  void download();
                }
              }}
            />
          </label>
          {busy && (
            <p role="status">{t("正在下载视频，首次使用会准备下载组件…")}</p>
          )}
          {error && (
            <p role="alert" className="video-url-error">
              {error}
            </p>
          )}
          <footer>
            <button
              type="button"
              disabled={cancelling}
              onClick={() => {
                if (!downloadId) {
                  setOpen(false);
                  return;
                }
                setCancelling(true);
                void bridge("cancel_video_import", { id: downloadId }).catch(
                  (e) => {
                    setError(String(e));
                    setCancelling(false);
                  },
                );
              }}
            >
              {cancelling ? t("正在取消…") : t("取消")}
            </button>
            <button
              type="button"
              className="primary"
              disabled={busy || !url.trim()}
              onClick={() => void download()}
            >
              {busy ? t("下载中…") : t("下载并添加")}
            </button>
          </footer>
        </Dialog.Content>
      </Dialog.Root>
    </>
  );
}
