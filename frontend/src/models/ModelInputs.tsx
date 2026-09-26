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
  const verified = verifiedInputs(profile);
  function option(kind: keyof ModelConnection["inputs"]) {
    const connected = protocolSupports(profile.adapter, kind);
    const supported = verified?.inputs[kind] !== false;
    const available = connected && supported;
    const reason = !supported
      ? "官方未列为支持"
      : !connected
        ? "当前接口未接入"
        : profile.inputs[kind]
          ? "已启用"
          : "未启用";
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
        <span>{kind === "document" ? "PDF 文件" : inputLabels[kind]}</span>
        <small>{reason}</small>
      </label>
    );
  }
  return (
    <section className="model-field-wide model-inputs">
      <strong>此连接启用的输入</strong>
      <p className="model-hint">
        文字始终可用。以下选项控制可发送的参考资料，不表示生成图片或音视频的能力。
      </p>
      {verified ? (
        <div className="model-input-evidence">
          <p>
            {verified.label}{" "}
            <a href={verified.source} target="_blank" rel="noreferrer">
              官方说明
            </a>
          </p>
          {profile.model === "gpt-6-astra" && (
            <a
              href="https://developers.openai.com/api/docs/guides/file-inputs"
              target="_blank"
              rel="noreferrer"
            >
              PDF 文件输入说明
            </a>
          )}
          <button
            type="button"
            onClick={() => update({ inputs: recommendedInputs(profile)! })}
          >
            按已核对能力填入
          </button>
        </div>
      ) : (
        <p className="model-hint">
          此型号尚未核对，请按提供方说明设置。接口可传入某类资料，不代表每个型号都能理解。
        </p>
      )}
      <div className="model-input-group">
        <small>多模态理解</small>
        <div className="model-input-options">
          {(["image", "audio", "video"] as const).map(option)}
        </div>
      </div>
      <div className="model-input-group">
        <small>文件读取</small>
        <div className="model-input-options">{option("document")}</div>
      </div>
      <p className="model-hint">
        PDF 读取可能包含页面图像和文字，不代表支持 Word、Excel 等所有文件。UTF-8
        文本资料按文字发送。
      </p>
      <small>
        当前应用限制：单素材 ≤ 12 MiB，文本 ≤ 120
        KB。音视频传原文件，不自动转录或抽帧。
      </small>
    </section>
  );
}
