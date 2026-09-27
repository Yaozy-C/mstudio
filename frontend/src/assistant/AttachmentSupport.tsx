import { t, useLanguage } from "../i18n";
import type { ModelConnection } from "../models/types";
import { supportsInput } from "../models/inputCapabilities";
import type { Attachment } from "./attachments";
const frameReadable = (model: ModelConnection, kind?: string) =>
  kind === "video" && supportsInput(model, "image");
export function unsupportedAttachments(
  items: Attachment[],
  model?: ModelConnection,
) {
  return model
    ? items.filter(
        (a) =>
          a.mediaKind &&
          !supportsInput(model, a.mediaKind) &&
          !frameReadable(model, a.mediaKind),
      )
    : [];
}
export function AttachmentSupport({
  items,
  model,
  settings,
}: {
  items: Attachment[];
  model?: ModelConnection;
  settings: () => void;
}) {
  useLanguage();
  if (!items.length) return null;
  const unsupported = unsupportedAttachments(items, model);
  return (
    <div className="agent-attachment-hint" role="status">
      {!model ? (
        t("请选择模型以检查资料读取能力。")
      ) : unsupported.length ? (
        <>
          {t("当前模型或接口无法读取：")}
          {unsupported.map((a) => a.title).join("、")}。
          <button type="button" onClick={settings}>
            {t("配置模型输入能力")}
          </button>
        </>
      ) : items.some(
          (a) =>
            frameReadable(model, a.mediaKind) && !supportsInput(model, "video"),
        ) ? (
        t("视频将按需抽帧分析，不包含声音；其他资料按模型支持的方式读取。")
      ) : (
        t("本轮将发送所选资料的原始内容；发送成功不代表模型已完整理解。")
      )}
    </div>
  );
}
