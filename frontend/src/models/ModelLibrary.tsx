import { t, useLanguage } from "../i18n";
import { StudioSelect } from "../ui/StudioSelect";
import { ModelMark } from "../ui/Identity";
import { useState } from "react";
import { ArrowLeft, Plus } from "@phosphor-icons/react";
import { native } from "../bridge";
import { modelLibrary, type ModelSpec, type OutputKind } from "./catalogSpecs";
import "../styles/model-library.css";
export function ModelLibrary({
  kind,
  configure,
  cancel,
  custom,
}: {
  kind: OutputKind;
  configure: (spec: ModelSpec, edit?: boolean) => void;
  cancel: () => void;
  custom: () => void;
}) {
  useLanguage();
  const [query, setQuery] = useState("");
  const [provider, setProvider] = useState("");
  const available = modelLibrary.filter(
    (m) => m.kind === kind && (m.endpoint || m.connection),
  );
  const providers = [
    ...new Set(available.map((m) => m.provider.split(" / ")[0])),
  ];
  const items = available.filter(
    (m) =>
      (!provider || m.provider.split(" / ")[0] === provider) &&
      `${t(m.name)} ${t(m.provider)}`
        .toLowerCase()
        .includes(query.toLowerCase()),
  );
  return (
    <section className="model-library" aria-label={t("添加模型")}>
      <div className="model-add-heading">
        <button
          aria-label={t("返回模型列表")}
          title={t("返回")}
          onClick={cancel}
        >
          <ArrowLeft />
        </button>
        <h3>
          {t("添加")}
          {kind === "text"
            ? t("对话")
            : kind === "image"
              ? t("图像")
              : t("视频")}
          {t("模型")}
        </h3>
        <button className="model-custom primary" onClick={custom}>
          <Plus size={16} />
          {t("添加其他模型")}
        </button>
      </div>
      <div className="model-picker-filter">
        <StudioSelect
          label={t("筛选厂商")}
          value={provider}
          onValueChange={setProvider}
          options={[
            { value: "", label: t("全部厂商") },
            ...providers.map((p) => ({ value: p, label: p })),
          ]}
        />
        <input
          className="model-search"
          aria-label={t("搜索型号")}
          placeholder={t("搜索型号…")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>
      <div className="spec-grid">
        {items.map((m) => (
          <article key={m.id} className="spec-card">
            <header>
              <small className="provider-label">
                <ModelMark identity={t(m.provider)} size={20} />
                {t(m.provider)}
              </small>
            </header>
            <h3>{t(m.name)}</h3>
            <p>{m.inputs.map((value) => t(value)).join(" · ")}</p>
            {kind !== "text" && (
              <small>
                {m.id === "codex-image"
                  ? t("本机 Codex · 无需 API Key · 型号未验证")
                  : m.id === "gemini-image-direct"
                    ? t("Google 官方直连")
                    : t("通过 fal 接入")}
              </small>
            )}
            <footer>
              <button
                className="primary"
                disabled={!native}
                onClick={() => configure(m)}
              >
                {m.kind === "image" ? t("添加生图模型") : t("选择")}
              </button>
              {m.kind === "image" && (
                <button disabled={!native} onClick={() => configure(m, true)}>
                  {t("添加图片编辑")}
                </button>
              )}
            </footer>
          </article>
        ))}
      </div>
      {!items.length && (
        <p className="model-hint">
          {t("没有找到此型号，可点击“添加其他模型”。")}
        </p>
      )}
    </section>
  );
}
