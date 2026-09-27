import { t, useLanguage } from "../i18n";
import type { ModelConnection } from "./types";
import { inputLabels, protocolSupports } from "./inputCapabilities";
import { verifiedInputs, recommendedInputs } from "./verifiedInputs";
export function ModelInputs({
  profile,
  update,
}: {
  profile: ModelConnection;
  update: (value: Partial<ModelConnection>) => void;
}) {
  useLanguage();
  const verified = verifiedInputs(profile);
  function option(kind: keyof ModelConnection["inputs"]) {
    const connected = protocolSupports(profile.adapter, kind);
    const supported = verified?.inputs[kind] !== false;
    const available = connected && supported;
    const reason = !supported
      ? t("官方未列为支持")
      : !connected
        ? t("当前接口未接入")
        : profile.inputs[kind]
          ? t("已启用")
          : t("未启用");
    return (
      <label className="model-input-option" key={kind}>
        <input
          type="checkbox"
          checked={available && profile.inputs[kind]}
          disabled={!available}
          onChange={(e) =>
            update({ inputs: { ...profile.inputs, [kind]: e.target.checked } })
          }
        />
        <span>
          {kind === "document" ? t("PDF 文件") : t(inputLabels[kind])}
        </span>
        <small>{reason}</small>
      </label>
    );
  }
  return (
    <section className="model-field-wide model-inputs">
      <strong>{t("此连接启用的输入")}</strong>
      <p className="model-hint">
        {t(
          "文字始终可用。以下选项控制可发送的参考资料，不表示生成图片或音视频的能力。",
        )}
      </p>
      {verified ? (
        <div className="model-input-evidence">
          <p>
            {t(verified.label)}{" "}
            <a href={verified.source} target="_blank" rel="noreferrer">
              {t("官方说明")}
            </a>
          </p>
          {profile.model === "gpt-6-astra" && (
            <a
              href="https://developers.openai.com/api/docs/guides/file-inputs"
              target="_blank"
              rel="noreferrer"
            >
              {t("PDF 文件输入说明")}
            </a>
          )}
          <button
            type="button"
            onClick={() => update({ inputs: recommendedInputs(profile)! })}
          >
            {t("按已核对能力填入")}
          </button>
        </div>
      ) : (
        <p className="model-hint">
          {t(
            "此型号尚未核对，请按提供方说明设置。接口可传入某类资料，不代表每个型号都能理解。",
          )}
        </p>
      )}
      <div className="model-input-group">
        <small>{t("多模态理解")}</small>
        <div className="model-input-options">
          {(["image", "audio", "video"] as const).map(option)}
        </div>
      </div>
      <div className="model-input-group">
        <small>{t("文件读取")}</small>
        <div className="model-input-options">{option("document")}</div>
      </div>
      <p className="model-hint">
        {t(
          "PDF 读取可能包含页面图像和文字，不代表支持 Word、Excel 等所有文件。UTF-8 文本资料按文字发送。",
        )}
      </p>
      <small>
        {t(
          "当前应用限制：单素材 ≤ 12 MiB，文本 ≤ 120 KB。音视频传原文件，不自动转录或抽帧。",
        )}
      </small>
    </section>
  );
}
