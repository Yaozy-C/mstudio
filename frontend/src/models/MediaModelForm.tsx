import { ErrorNotice } from "../errors/ErrorNotice";
import { ServicePicker } from "./ServicePicker";
import {
  imageServiceKinds,
  videoServiceKinds,
  type ServiceKind,
} from "./connectionStore";
import { StudioSelect } from "../ui/StudioSelect";
import { HttpModelFields, defaultHttpMapping } from "./HttpModelFields";
import { useState } from "react";
import { mediaLabels, type MediaModel } from "./mediaRegistry";
export function MediaModelForm({
  initial,
  save,
  cancel,
}: {
  initial: MediaModel;
  save: (model: MediaModel, key?: string, clearKey?: boolean) => Promise<void>;
  cancel: () => void;
}) {
  const [model, setModel] = useState(initial);
  const [params, setParams] = useState(JSON.stringify(initial.params, null, 2));
  const [mapping, setMapping] = useState(
    JSON.stringify(initial.http ?? defaultHttpMapping, null, 2),
  );
  const [nativeModel, setNativeModel] = useState(
    String(initial.params.model || "gemini-3.1-flash-image"),
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  return (
    <form
      className="model-form"
      onSubmit={(e) => {
        e.preventDefault();
        if (!model.connectionId) return;
        setBusy(true);
        setError("");
        void (async () => {
          try {
            await save(
              {
                ...model,
                name: model.name.trim(),
                endpoint: model.endpoint.trim(),
                params:
                  model.plugin === "gemini-native"
                    ? { ...JSON.parse(params), model: nativeModel }
                    : JSON.parse(params),
                http:
                  model.plugin === "http-json"
                    ? JSON.parse(mapping)
                    : undefined,
              },
              undefined,
              false,
            );
            cancel();
          } catch (e) {
            setError(String(e));
          } finally {
            setBusy(false);
          }
        })();
      }}
    >
      <div className="model-section-heading">
        <h3>
          {initial.name ? "配置模型" : `添加${mediaLabels[model.kind]}模型`}
        </h3>
      </div>
      <p className="model-hint">
        保存模型连接。生成描述和参考素材在项目中填写。
      </p>
      <fieldset disabled={busy}>
        <div className="model-form-grid">
          <label>
            模型名称
            <input
              autoFocus
              required
              maxLength={80}
              value={model.name}
              onChange={(e) => setModel({ ...model, name: e.target.value })}
              placeholder="给这个模型起个易识别的名字"
            />
          </label>
          <label>
            输出类型
            <StudioSelect
              label="输出类型"
              disabled={busy}
              value={model.kind}
              onValueChange={(kind) =>
                setModel({
                  ...model,
                  kind: kind as MediaModel["kind"],
                  connectionId: undefined,
                  hasKey: false,
                })
              }
              options={Object.entries(mediaLabels).map(([value, label]) => ({
                value,
                label,
              }))}
            />
          </label>

          <ServicePicker
            id={model.connectionId}
            kinds={
              model.kind === "image" ? imageServiceKinds : videoServiceKinds
            }
            suggested={{
              kind: model.plugin as ServiceKind,
              endpoint:
                model.plugin === "fal"
                  ? "https://queue.fal.run"
                  : model.plugin === "http-json"
                    ? ""
                    : model.endpoint,
            }}
            onChange={(service) => {
              const changed = service.kind !== model.plugin;
              setModel({
                ...model,
                connectionId: service.id,
                plugin: service.kind,
                hasKey: service.hasKey,
                endpoint:
                  service.kind === "gemini-native"
                    ? service.endpoint
                    : changed
                      ? ""
                      : model.endpoint,
              });
              if (changed)
                setParams(
                  JSON.stringify(
                    service.kind === "http-json"
                      ? { prompt: "{{prompt}}" }
                      : service.kind === "gemini-native"
                        ? { model: nativeModel }
                        : {},
                  ),
                );
            }}
          />
          {model.plugin === "gemini-native" && (
            <>
              <label className="model-field-wide">
                模型 ID
                <input
                  required
                  value={nativeModel}
                  onChange={(e) => setNativeModel(e.target.value)}
                  placeholder="gemini-3.1-flash-image"
                />
              </label>
            </>
          )}
          <details
            className="model-field-wide"
            open={!initial.endpoint || undefined}
          >
            <summary>高级连接设置</summary>
            <label className="model-field-wide">
              {model.plugin === "fal"
                ? "模型端点 ID"
                : model.plugin === "gemini-native"
                  ? "Google 服务地址"
                  : "提交接口 URL"}
              <input
                required
                readOnly={model.plugin === "gemini-native"}
                value={model.endpoint}
                onChange={(e) =>
                  setModel({ ...model, endpoint: e.target.value })
                }
                placeholder={
                  model.plugin === "fal"
                    ? "例如 fal-ai/flux/schnell"
                    : "https://api.example.com/v1/images/generations"
                }
              />
              <small>
                {model.plugin === "fal"
                  ? "填写 fal 的完整端点 ID，共用服务连接中的密钥。"
                  : model.plugin === "gemini-native"
                    ? "使用 Google 官方服务地址，模型 ID 在上方填写。"
                    : "填写完整 POST 地址，支持 HTTPS 或本机 HTTP。"}
              </small>
            </label>
            {model.plugin === "http-json" && (
              <HttpModelFields mapping={mapping} setMapping={setMapping} />
            )}
            <label className="model-field-wide">
              {model.plugin !== "http-json" ? "默认生成参数" : "请求 JSON 模板"}
              <textarea
                rows={6}
                value={params}
                onChange={(e) => setParams(e.target.value)}
                spellCheck={false}
              />
              <small>
                {model.plugin === "fal"
                  ? "按模型文档填写 JSON，提交时自动填入 prompt。"
                  : model.plugin === "gemini-native"
                    ? "generationConfig 可设置画幅等选项（model 以上方模型 ID 为准），文字和图片由适配器自动处理。"
                    : "按接口文档填写完整请求，可包含模型 ID 和嵌套输入。{{prompt}} 会替换为当前生成描述；参考 URL 可直接填写。密钥不要写在这里。"}
              </small>
            </label>
          </details>
        </div>
        {error && <ErrorNotice error={error} fallback="VALIDATION_FAILED" />}
        <footer>
          <button type="button" onClick={cancel}>
            取消
          </button>
          <button className="primary" disabled={busy || !model.connectionId}>
            {busy ? "保存中…" : "保存模型"}
          </button>
        </footer>
      </fieldset>
    </form>
  );
}
