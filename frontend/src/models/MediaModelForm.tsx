import { AsyncButton } from "../ui/AsyncState";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { ServicePicker } from "./ServicePicker";
import {
  imageServiceKinds,
  videoServiceKinds,
  type ServiceKind,
} from "./connectionStore";
import { StudioSelect } from "../ui/StudioSelect";
import { HttpModelFields, defaultHttpMapping } from "./HttpModelFields";
import { parseCapabilities } from "./capabilities";
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
  useLanguage();
  const [model, setModel] = useState(initial);
  const [params, setParams] = useState(JSON.stringify(initial.params, null, 2));
  const [mapping, setMapping] = useState(
    JSON.stringify(initial.http ?? defaultHttpMapping, null, 2),
  );
  const [nativeModel, setNativeModel] = useState(
    String(initial.params.model || "gemini-3.1-flash-image"),
  );
  const [capabilities, setCapabilities] = useState(
    initial.capabilities ? JSON.stringify(initial.capabilities, null, 2) : "",
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
                capabilities: parseCapabilities(capabilities),
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
          {initial.name
            ? t("配置模型")
            : t("添加{v0}模型", { v0: t(mediaLabels[model.kind]) })}
        </h3>
      </div>
      <p className="model-hint">
        {t("保存模型连接。生成描述和参考素材在项目中填写。")}
      </p>
      <fieldset disabled={busy}>
        <div className="model-form-grid">
          <label>
            {t("模型名称")}
            <input
              autoFocus
              required
              maxLength={80}
              value={model.name}
              onChange={(e) => setModel({ ...model, name: e.target.value })}
              placeholder={t("给这个模型起个易识别的名字")}
            />
          </label>
          <label>
            {t("输出类型")}
            <StudioSelect
              label={t("输出类型")}
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
                label: t(label),
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
                {t("模型 ID")}
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
            <summary>{t("高级连接设置")}</summary>
            <label className="model-field-wide">
              {model.plugin === "fal"
                ? t("模型端点 ID")
                : model.plugin === "gemini-native"
                  ? t("Google 服务地址")
                  : t("提交接口 URL")}
              <input
                required
                readOnly={model.plugin === "gemini-native"}
                value={model.endpoint}
                onChange={(e) =>
                  setModel({ ...model, endpoint: e.target.value })
                }
                placeholder={
                  model.plugin === "fal"
                    ? t("例如 fal-ai/flux/schnell")
                    : "https://api.example.com/v1/images/generations"
                }
              />
              <small>
                {model.plugin === "fal"
                  ? t("填写 fal 的完整端点 ID，共用服务连接中的密钥。")
                  : model.plugin === "gemini-native"
                    ? t("使用 Google 官方服务地址，模型 ID 在上方填写。")
                    : t("填写完整 POST 地址，支持 HTTPS 或本机 HTTP。")}
              </small>
            </label>
            {model.plugin === "http-json" && (
              <HttpModelFields mapping={mapping} setMapping={setMapping} />
            )}
            <label className="model-field-wide">
              {t("参考输入与参数能力（可选）")}
              <textarea
                rows={6}
                spellCheck={false}
                value={capabilities}
                onChange={(e) => setCapabilities(e.target.value)}
                placeholder={JSON.stringify(
                  {
                    references: [
                      {
                        key: "/image_url",
                        kind: "image",
                        role: "first-frame",
                      },
                    ],
                    controls: {
                      duration: { path: "/duration", min: 2, max: 15 },
                    },
                  },
                  null,
                  2,
                )}
              />
              <small>
                {t(
                  "声明此模型接受的参考用途与可调参数，值为请求体中的 JSON Pointer；留空表示只用协议默认能力。参考字段会从生成面板的参考素材写入对应位置，控件决定面板显示哪些设置。",
                )}
              </small>
            </label>
            <label className="model-field-wide">
              {model.plugin !== "http-json"
                ? t("默认生成参数")
                : t("请求 JSON 模板")}
              <textarea
                rows={6}
                value={params}
                onChange={(e) => setParams(e.target.value)}
                spellCheck={false}
              />
              <small>
                {model.plugin === "fal"
                  ? t("按模型文档填写 JSON，提交时自动填入 prompt。")
                  : model.plugin === "gemini-native"
                    ? t(
                        "generationConfig 可设置画幅等选项（model 以上方模型 ID 为准），文字和图片由适配器自动处理。",
                      )
                    : t(
                        "按接口文档填写完整请求，可包含模型 ID 和嵌套输入。{{prompt}} 会替换为当前生成描述；参考 URL 可直接填写。密钥不要写在这里。",
                      )}
              </small>
            </label>
          </details>
        </div>
        {error && <ErrorNotice error={error} fallback="VALIDATION_FAILED" />}
        <footer>
          <button type="button" onClick={cancel}>
            {t("取消")}
          </button>
          <AsyncButton
            busy={busy}
            busyLabel={t("保存中…")}
            type="submit"
            className="primary"
            disabled={busy || !model.connectionId}
          >
            {t("保存模型")}
          </AsyncButton>
        </footer>
      </fieldset>
    </form>
  );
}
