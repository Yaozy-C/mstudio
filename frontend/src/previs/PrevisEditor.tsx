import { Dialog } from "@radix-ui/themes";
import { Cube, X, DownloadSimple } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import { readyModel } from "../models/types";
import { usePrevisEditor } from "./usePrevisEditor";
import { PrevisVisual } from "./PrevisVisual";
import { PrevisChat } from "./PrevisChat";

export function PrevisEditor({
  fileId,
  close,
}: {
  fileId: string;
  close: () => void;
}) {
  useLanguage();
  const state = usePrevisEditor(fileId, close);
  const { busy, dismiss, file, draft, model, ready, render } = state;
  return (
    <Dialog.Root
      open
      onOpenChange={(v) => {
        if (!v) dismiss();
      }}
    >
      <Dialog.Content
        className="previs-dialog"
        aria-describedby={undefined}
        onEscapeKeyDown={(e) => {
          if (busy) e.preventDefault();
        }}
      >
        <header className="previs-heading">
          <div>
            <Cube size={20} />
            <Dialog.Title title={file.name}>
              {file.kind === "model" ? t("共用模型") : t("镜头预演")} ·{" "}
              {file.name}
            </Dialog.Title>
            <span className="previs-version">v{file.revision}</span>
          </div>
          <div className="previs-heading-actions">
            {file.kind === "shot" && (
              <button
                className="primary"
                type="button"
                disabled={
                  !!busy ||
                  (!draft.code.trim() && (!model || !readyModel(model))) ||
                  (!!file.code && draft.code === file.code && !ready)
                }
                onClick={() => void render()}
              >
                <DownloadSimple />
                {draft.code.trim() ? t("生成预演视频") : t("一键生成预演")}
              </button>
            )}
            <button
              className="previs-close"
              type="button"
              aria-label={t("关闭预演")}
              onClick={dismiss}
              disabled={!!busy}
            >
              <X />
            </button>
          </div>
        </header>
        <div className="previs-editor-grid">
          <PrevisVisual state={state} />
          <PrevisChat state={state} />
        </div>
      </Dialog.Content>
    </Dialog.Root>
  );
}
