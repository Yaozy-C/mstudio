import { ChatText as MenuChatText } from "@phosphor-icons/react";
import { ImageProcessingPreview } from "./imageReview/ImageProcessingPreview";
import { useState, type ReactNode } from "react";
import { t, useLanguage } from "../i18n";
import { DocumentPreview } from "./DocumentPreview";
import { ObjectActions } from "../ui/ObjectActions";
import { Dialog } from "@radix-ui/themes";
import { X } from "@phosphor-icons/react";
import type { Asset } from "../model";
import { mediaUrl } from "../bridge";
import { MissingAsset } from "./MissingAsset";
export function MediaPreview({
  asset: source,
  close: onClose,
  place,
  add,
  reference,
  actions,
}: {
  asset?: Asset;
  close: () => void;
  place?: (a: Asset) => void;
  add?: (a: Asset) => void;
  reference?: (a: Asset) => void;
  actions?: (a: Asset) => ReactNode;
}) {
  useLanguage();
  const [selection, setSelection] = useState<{
    sourceId: string;
    asset: Asset;
  }>();
  const asset = selection?.sourceId === source?.id ? selection?.asset : source;
  const close = () => {
    setSelection(undefined);
    onClose();
  };
  return (
    <Dialog.Root
      open={!!asset}
      onOpenChange={(open) => {
        if (!open) close();
      }}
    >
      <Dialog.Content
        className={`media-preview-dialog${asset && ["image", "video"].includes(asset.kind) && !asset.missing ? " visual-preview-dialog" : ""}`}
        aria-describedby={undefined}
      >
        <header>
          <Dialog.Title>{asset?.name}</Dialog.Title>
          {asset && !asset.missing && reference && (
            <ObjectActions
              actions={[
                {
                  icon: MenuChatText,
                  label: t("引用到对话"),
                  run: () => {
                    reference(asset);
                    close();
                  },
                },
              ]}
            />
          )}
          <Dialog.Close>
            <button aria-label={t("关闭素材预览")}>
              <X />
            </button>
          </Dialog.Close>
        </header>
        {asset && (
          <>
            <div className="media-preview-stage">
              {asset.missing ? (
                <MissingAsset asset={asset} />
              ) : asset.kind === "text" || asset.kind === "document" ? (
                <DocumentPreview asset={asset} />
              ) : asset.kind === "image" ? (
                <ImageProcessingPreview
                  key={source!.id}
                  asset={source!}
                  close={close}
                  onAssetChange={(asset) =>
                    setSelection({ sourceId: source!.id, asset })
                  }
                />
              ) : asset.kind === "video" ? (
                <video src={mediaUrl(asset.path)} controls />
              ) : (
                <audio src={mediaUrl(asset.path)} controls />
              )}
            </div>
            {!asset.missing && (place || add || actions) && (
              <footer>
                {place && (
                  <button
                    onClick={() => {
                      place(asset);
                      close();
                    }}
                  >
                    {t("放到画布")}
                  </button>
                )}
                {add && (
                  <button
                    disabled={
                      asset.kind === "text" || asset.kind === "document"
                    }
                    onClick={() => {
                      add(asset);
                      close();
                    }}
                  >
                    {t("加入时间线")}
                  </button>
                )}
                {actions?.(asset)}
              </footer>
            )}
          </>
        )}
      </Dialog.Content>
    </Dialog.Root>
  );
}
