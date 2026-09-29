import { ZoomableImage } from "./ZoomableImage";
import { t, useLanguage } from "../i18n";
import { DocumentPreview } from "./DocumentPreview";
import { ObjectActions } from "../ui/ObjectActions";
import { Dialog } from "@radix-ui/themes";
import { X } from "@phosphor-icons/react";
import type { Asset } from "../model";
import { mediaUrl } from "../bridge";
import { MissingAsset } from "./MissingAsset";
export function MediaPreview({
  asset,
  close,
  place,
  add,
  reference,
}: {
  asset?: Asset;
  close: () => void;
  place?: (a: Asset) => void;
  add?: (a: Asset) => void;
  reference?: (a: Asset) => void;
}) {
  useLanguage();
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
                <ZoomableImage
                  key={asset.id}
                  src={mediaUrl(asset.path)}
                  alt={asset.name}
                />
              ) : asset.kind === "video" ? (
                <video src={mediaUrl(asset.path)} controls />
              ) : (
                <audio src={mediaUrl(asset.path)} controls />
              )}
            </div>
            {!asset.missing && (place || add) && (
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
              </footer>
            )}
          </>
        )}
      </Dialog.Content>
    </Dialog.Root>
  );
}
