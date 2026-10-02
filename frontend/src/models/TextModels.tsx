import { AsyncButton, LoadingState } from "../ui/AsyncState";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { inputSummary } from "./inputCapabilities";
import { StudioSelect } from "../ui/StudioSelect";
import { ModelMark } from "../ui/Identity";
import { useEffect, useState } from "react";
import { Plus, Cube, PencilSimple, Trash, X } from "@phosphor-icons/react";
import { bridge, native } from "../bridge";
import { useModels, modelsChanged } from "./useModels";
import {
  newConnection,
  readyModel,
  localEndpoint,
  type ModelConnection,
} from "./types";
import { ModelLibrary } from "./ModelLibrary";
import { ModelForm } from "./ModelForm";
import "../styles/model-center.css";
export function TextModels() {
  useLanguage();
  const [adding, setAdding] = useState(false);
  const hub = useModels();
  const [edit, setEdit] = useState<ModelConnection | null>(null);
  const [remove, setRemove] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [activeTest, setActiveTest] = useState<string | null>(null);
  const [toast, setToast] = useState<{ text: string; error: boolean } | null>(
    null,
  );
  const [error, setError] = useState("");
  const [query, setQuery] = useState("");
  useEffect(() => {
    if (!toast || toast.error) return;
    const timeout = window.setTimeout(() => setToast(null), 5000);
    return () => window.clearTimeout(timeout);
  }, [toast]);
  async function act(command: string, id: string) {
    setBusy(true);
    setActiveTest(command === "test_model" ? id : null);
    setError("");
    setToast(null);
    try {
      const result = await bridge<string>(command, { id });
      if (command === "test_model") {
        const model = hub.catalog.profiles.find((p) => p.id === id);
        setToast({
          text: `${model?.name ?? t("模型")}：${result}`,
          error: false,
        });
      } else {
        modelsChanged();
        setRemove(null);
      }
    } catch (e) {
      if (command === "test_model") {
        setToast({ text: String(e), error: true });
      } else {
        setError(String(e));
      }
    } finally {
      setBusy(false);
      setActiveTest(null);
    }
  }
  if (edit)
    return (
      <ModelForm
        key={edit.id}
        initial={edit}
        cancel={() => setEdit(null)}
        saved={() => {
          setEdit(null);
          setAdding(false);
          modelsChanged();
          setToast({ text: t("模型连接已保存"), error: false });
        }}
      />
    );
  if (adding)
    return (
      <ModelLibrary
        kind="text"
        cancel={() => setAdding(false)}
        custom={() => setEdit(newConnection())}
        configure={(spec) => {
          if (spec.connection)
            setEdit({
              ...newConnection(),
              ...spec.connection,
              name: spec.name,
            });
        }}
      />
    );
  const profiles = hub.catalog.profiles.filter((p) =>
    `${p.name} ${p.model} ${p.endpoint}`
      .toLowerCase()
      .includes(query.toLowerCase()),
  );
  return (
    <section className="model-center">
      <div className="model-default">
        <div>
          <strong>{t("默认对话模型")}</strong>
          <p>
            {t(
              "对话使用项目选择的对话模型，未选择时使用此默认模型。切换 Agent 不改变模型。",
            )}
          </p>
        </div>
        <StudioSelect
          label={t("默认对话模型")}
          disabled={busy || hub.loading}
          value={hub.catalog.defaultId || ""}
          placeholder={t("选择默认模型")}
          onValueChange={(id) => void act("default_agent_model", id)}
          options={hub.catalog.profiles.map((p) => ({
            value: p.id,
            label: p.name + (readyModel(p) ? "" : t(" · 待配置密钥")),
            disabled: !readyModel(p),
          }))}
        />
      </div>
      <div className="model-section-heading">
        <h3>
          {t("对话模型")} <span>{hub.catalog.profiles.length}</span>
        </h3>
        <button
          className="primary"
          disabled={!native || busy || hub.loading}
          onClick={() => {
            setError("");
            setToast(null);
            setAdding(true);
          }}
        >
          <Plus />
          {t("添加模型")}
        </button>
      </div>
      {hub.catalog.profiles.length > 3 && (
        <input
          className="model-search"
          aria-label={t("搜索模型")}
          placeholder={t("搜索名称、模型或地址…")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      )}
      {(error || hub.error) && (
        <ErrorNotice error={error || hub.error} fallback="OPERATION_FAILED">
          {hub.error && (
            <AsyncButton busy={hub.loading} onClick={() => void hub.refresh()}>
              {t("重试")}
            </AsyncButton>
          )}
        </ErrorNotice>
      )}
      {toast && (
        <div
          className={`model-toast${toast.error ? " model-toast-error" : ""}`}
          role={toast.error ? "alert" : "status"}
        >
          {toast.error ? (
            <ErrorNotice error={toast.text} />
          ) : (
            <span>{toast.text}</span>
          )}
          <button aria-label={t("关闭提示")} onClick={() => setToast(null)}>
            <X size={16} />
          </button>
        </div>
      )}
      {hub.loading && !hub.catalog.profiles.length && (
        <LoadingState label={t("正在读取…")} />
      )}
      <div className="model-list">
        {profiles.map((p) => (
          <article className="model-row" key={p.id}>
            <div className="model-logo">
              <ModelMark identity={`${p.endpoint} ${p.model}`} />
            </div>
            <div className="model-row-copy">
              <strong>
                {p.name}
                {hub.catalog.defaultId === p.id && (
                  <span className="model-badge">{t("默认")}</span>
                )}
              </strong>
              <p title={p.endpoint}>
                {p.model} <span>·</span>{" "}
                {p.endpoint.replace(/^https?:\/\//, "").split("/")[0]}
              </p>
              <small>
                {inputSummary(p)} ·{" "}
                {p.adapter === "codex"
                  ? t("ChatGPT 账号登录")
                  : p.hasKey
                    ? t("密钥已配置")
                    : localEndpoint(p.endpoint)
                      ? t("本机连接")
                      : t("待配置密钥")}
              </small>
            </div>
            <div className="model-row-actions">
              <AsyncButton
                busy={activeTest === p.id}
                busyLabel={t("正在连接…")}
                disabled={busy || !readyModel(p)}
                onClick={() => void act("test_model", p.id)}
              >
                {t("测试连接")}
              </AsyncButton>
              <button
                aria-label={t("编辑模型 {v0}", { v0: p.name })}
                title={t("编辑模型")}
                disabled={busy}
                onClick={() => setEdit(p)}
              >
                <PencilSimple />
              </button>
              <button
                aria-label={t("移除模型 {v0}", { v0: p.name })}
                title={t("移除模型")}
                disabled={busy}
                onClick={() => setRemove(p.id)}
              >
                <Trash />
              </button>
            </div>
            {remove === p.id && (
              <div className="model-remove">
                <span>
                  {t("移除「")}
                  {p.name}
                  {t("」？聊天记录和服务连接会保留。")}
                </span>
                <button disabled={busy} onClick={() => setRemove(null)}>
                  {t("取消")}
                </button>
                <AsyncButton
                  busy={busy}
                  busyLabel={t("正在移除…")}
                  disabled={busy}
                  onClick={() => void act("remove_model", p.id)}
                >
                  {t("确认移除")}
                </AsyncButton>
              </div>
            )}
          </article>
        ))}
        {!hub.loading && !hub.error && !profiles.length && (
          <div className="model-empty">
            <Cube size={32} />
            <strong>
              {query ? t("没有匹配的模型") : t("连接你的第一个对话模型")}
            </strong>
            <p>
              {native
                ? t(
                    "支持 OpenAI 兼容服务、Responses、Gemini、Claude 原生接口及本机模型。",
                  )
                : t("请在桌面应用中配置模型，浏览器仅提供界面预览。")}
            </p>
          </div>
        )}
      </div>
      <p className="model-hint">
        {t(
          "密钥仅保存在本机，不进入项目或聊天。测试连接只读取模型列表，不生成内容。",
        )}
      </p>
    </section>
  );
}
