import { StudioSelect } from "../ui/StudioSelect";
import { ModelMark } from "../ui/Identity";
import { useState } from "react";
import { ArrowLeft } from "@phosphor-icons/react";
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
      `${m.name} ${m.provider}`.toLowerCase().includes(query.toLowerCase()),
  );
  return (
    <section className="model-library" aria-label="添加模型">
      <div className="model-add-heading">
        <button aria-label="返回模型列表" title="返回" onClick={cancel}>
          <ArrowLeft />
        </button>
        <h3>
          添加{kind === "text" ? "对话" : kind === "image" ? "图像" : "视频"}
          模型
        </h3>
        <button className="model-custom" onClick={custom}>
          接入其他模型
        </button>
      </div>
      <p className="model-hint">
        选择型号后填写连接信息。没有找到需要的型号，可以接入其他模型。
      </p>
      <div className="model-picker-filter">
        <StudioSelect
          label="筛选厂商"
          value={provider}
          onValueChange={setProvider}
          options={[
            { value: "", label: "全部厂商" },
            ...providers.map((p) => ({ value: p, label: p })),
          ]}
        />
        <input
          className="model-search"
          aria-label="搜索型号"
          placeholder="搜索型号…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>
      <div className="spec-grid">
        {items.map((m) => (
          <article key={m.id} className="spec-card">
            <header>
              <small className="provider-label">
                <ModelMark identity={m.provider} size={20} />
                {m.provider}
              </small>
            </header>
            <h3>{m.name}</h3>
            <p>{m.inputs.join(" · ")}</p>
            {kind !== "text" && (
              <small>
                {m.id === "codex-image"
                  ? "本机 Codex · 无需 API Key · 型号未验证"
                  : m.id === "gemini-image-direct"
                    ? "Google 官方直连"
                    : "通过 fal 接入"}
              </small>
            )}
            <footer>
              <button
                className="primary"
                disabled={!native}
                onClick={() => configure(m)}
              >
                {m.kind === "image" ? "添加生图模型" : "选择"}
              </button>
              {m.kind === "image" && (
                <button disabled={!native} onClick={() => configure(m, true)}>
                  添加图片编辑
                </button>
              )}
            </footer>
          </article>
        ))}
      </div>
      {!items.length && (
        <p className="model-hint">没有找到此型号，可点击“接入其他模型”。</p>
      )}
    </section>
  );
}
