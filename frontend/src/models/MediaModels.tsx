import { AsyncButton, LoadingState } from "../ui/AsyncState";
import { presets } from "./mediaModelPresets";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { catalogMediaModel } from "./catalogMediaModel";
import { CodexImageForm } from "./CodexImageForm";
import { ModelMark } from "../ui/Identity";
import { localEndpoint } from "./types";
import { useState } from "react";
import { AlertDialog } from "@radix-ui/themes";
import {
  Image,
  FilmStrip,
  Waveform,
  Plus,
  PencilSimple,
  Trash,
} from "@phosphor-icons/react";
import { native } from "../bridge";
import { ModelLibrary } from "./ModelLibrary";
import { MediaModelForm } from "./MediaModelForm";
import {
  mediaLabels,
  type MediaKind,
  type MediaModel,
  type useMediaModels,
} from "./mediaRegistry";
const icons = { image: Image, video: FilmStrip, audio: Waveform };

export function MediaModels({
  kind,
  hub,
}: {
  kind: MediaKind;
  hub: ReturnType<typeof useMediaModels>;
}) {
  useLanguage();
  const [adding, setAdding] = useState(false);
  const [edit, setEdit] = useState<MediaModel | null>(null);
  const [query, setQuery] = useState("");
  const [remove, setRemove] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const Icon = icons[kind];
  const items = hub.models.filter(
    (m) =>
      m.kind === kind &&
      `${m.name} ${m.endpoint}`.toLowerCase().includes(query.toLowerCase()),
  );
  const create = (preset = false) =>
    setEdit({
      id: crypto.randomUUID(),
      plugin: kind === "image" && !preset ? "gemini-native" : "fal",
      enabled: true,
      kind,
      ...(preset && kind !== "audio"
        ? presets[kind]
        : kind === "image"
          ? {
              name: "",
              endpoint: "https://generativelanguage.googleapis.com",
              params: { model: "gemini-3.1-flash-image" },
            }
          : { name: "", endpoint: "", params: {} }),
    });
  async function act(fn: () => Promise<void>) {
    setBusy(true);
    setError("");
    try {
      await fn();
      setRemove(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  if (edit?.plugin === "codex-image")
    return (
      <CodexImageForm
        initial={edit}
        save={hub.save}
        cancel={() => {
          setEdit(null);
          setAdding(false);
        }}
      />
    );
  if (edit)
    return (
      <MediaModelForm
        initial={edit}
        save={async (...args) => {
          await hub.save(...args);
          setAdding(false);
        }}
        cancel={() => setEdit(null)}
      />
    );
  if (adding && kind !== "audio")
    return (
      <ModelLibrary
        kind={kind}
        cancel={() => setAdding(false)}
        custom={() => create()}
        configure={(spec, editing) => {
          if (!spec.endpoint) return;
          setEdit(catalogMediaModel(spec, kind, editing));
        }}
      />
    );
  return (
    <section>
      <div className="hub-section-head">
        <div>
          <h3>{t("{kind}模型", { kind: t(mediaLabels[kind]) })}</h3>
        </div>
        <button
          className="primary"
          disabled={!native || hub.loading || busy}
          onClick={() => (kind === "audio" ? create() : setAdding(true))}
        >
          <Plus />
          {t("添加模型")}
        </button>
      </div>
      <input
        className="model-search"
        aria-label={t("搜索{v0}模型", { v0: t(mediaLabels[kind]) })}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder={t("搜索模型名称或端点…")}
      />
      {((!remove && error) || hub.error) && (
        <ErrorNotice error={(!remove && error) || hub.error}>
          {hub.error && (
            <AsyncButton busy={hub.loading} onClick={() => void hub.refresh()}>
              {t("重试")}
            </AsyncButton>
          )}
        </ErrorNotice>
      )}
      {hub.loading && !hub.models.length && (
        <LoadingState label={t("正在读取…")} />
      )}
      <div className="hub-model-grid">
        {items.map((m) => (
          <article className="hub-model-card" key={m.id}>
            <div className="hub-card-top">
              <span className={`hub-capability ${kind}`}>
                <ModelMark identity={`${m.name} ${m.endpoint}`} size={26} />
              </span>
              <span
                className={`hub-status ${m.enabled && (m.hasKey || m.plugin === "codex-image" || localEndpoint(m.endpoint)) ? "ready" : ""}`}
              >
                {!m.enabled
                  ? t("已停用")
                  : m.plugin === "codex-image"
                    ? t("本机 Codex")
                    : m.hasKey
                      ? t("密钥已配置")
                      : localEndpoint(m.endpoint)
                        ? t("本机连接")
                        : t("待配置密钥")}
              </span>
            </div>
            <h3>{m.name}</h3>
            <p className="hub-endpoint" title={m.endpoint}>
              {m.endpoint}
            </p>
            <div className="hub-tags">
              <span>{t("{kind}生成", { kind: t(mediaLabels[kind]) })}</span>
              <span>
                {m.plugin === "codex-image"
                  ? t("Codex · 型号未验证")
                  : m.plugin === "fal"
                    ? t("fal · 第三方")
                    : m.plugin === "dashscope"
                      ? t("百炼 · 官方直连")
                      : m.plugin === "gemini-native"
                        ? t("Google · 官方直连")
                        : t("自定义 HTTP")}
              </span>
            </div>
            <footer>
              <button
                disabled={busy}
                onClick={() =>
                  void act(() => hub.save({ ...m, enabled: !m.enabled }))
                }
              >
                {m.enabled ? t("停用") : t("启用")}
              </button>
              <button
                disabled={busy}
                onClick={() => setEdit(m)}
                aria-label={t("配置 {v0}", { v0: m.name })}
              >
                <PencilSimple />
                {t("配置")}
              </button>
              <AlertDialog.Root
                open={remove === m.id}
                onOpenChange={(open) => {
                  if (busy) return;
                  setError("");
                  setRemove(open ? m.id : null);
                }}
              >
                <AlertDialog.Trigger>
                  <button
                    disabled={busy}
                    aria-label={t("移除 {v0}", { v0: m.name })}
                  >
                    <Trash />
                  </button>
                </AlertDialog.Trigger>
                <AlertDialog.Content className="modal small" aria-busy={busy}>
                  <AlertDialog.Title>{t("移除模型")}</AlertDialog.Title>
                  <AlertDialog.Description>
                    {t("移除「")}
                    {m.name}
                    {t("」的模型配置？已生成的素材和任务会保留。")}
                  </AlertDialog.Description>
                  {error && (
                    <ErrorNotice error={error} fallback="OPERATION_FAILED" />
                  )}
                  <footer>
                    <AlertDialog.Cancel>
                      <button type="button" disabled={busy}>
                        {t("取消")}
                      </button>
                    </AlertDialog.Cancel>
                    <button
                      type="button"
                      className="primary"
                      disabled={busy}
                      onClick={() => void act(() => hub.remove(m.id))}
                    >
                      {busy ? t("正在移除…") : t("确认移除")}
                    </button>
                  </footer>
                </AlertDialog.Content>
              </AlertDialog.Root>
            </footer>
          </article>
        ))}
      </div>
      {!hub.loading && !hub.error && !items.length && (
        <div className="hub-empty">
          <Icon size={34} />
          <h3>
            {hub.loading
              ? t("读取模型…")
              : query
                ? t("没有找到匹配模型")
                : t("添加你的第一个{v0}模型", { v0: t(mediaLabels[kind]) })}
          </h3>
          <p>
            {query
              ? t("试试其他名称或清空搜索。")
              : t("选择一个模型起步，也可以接入自定义端点。")}
          </p>
          {!query && kind !== "audio" && (
            <button
              disabled={!native || busy || hub.loading}
              onClick={() => create(true)}
            >
              {t("配置")} {presets[kind].name}
            </button>
          )}
        </div>
      )}
      {kind === "audio" && (
        <div className="hub-local-note">
          <Waveform />
          <div>
            <strong>{t("也可以使用本机配音")}</strong>
            <p>
              {t("无需 API Key。在制作台的「配音」中选择已安装的系统声音。")}
            </p>
          </div>
        </div>
      )}
      <p className="model-hint">
        {t(
          "保存后可在生成面板选择此模型。启用仅代表允许选择；实际可用性取决于服务权限和模型参数。",
        )}
      </p>
    </section>
  );
}
