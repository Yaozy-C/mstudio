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
const presets = {
  image: {
    name: "FLUX.1 Schnell",
    endpoint: "fal-ai/flux/schnell",
    params: { num_images: 1 },
  },
  video: {
    name: "MiniMax H3",
    endpoint: "minimax/h3/text-to-video",
    params: { duration: 5 },
  },
};
export function MediaModels({
  kind,
  hub,
}: {
  kind: MediaKind;
  hub: ReturnType<typeof useMediaModels>;
}) {
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
          <h3>{mediaLabels[kind]}模型</h3>
          <p>
            {kind === "image"
              ? "参考画面、分镜静帧与视觉探索。"
              : kind === "video"
                ? "让分镜变成镜头，让创意开始运动。"
                : "为画面添加声音、音乐与氛围。"}
          </p>
        </div>
        <button
          className="primary"
          disabled={!native || hub.loading || busy}
          onClick={() => (kind === "audio" ? create() : setAdding(true))}
        >
          <Plus />
          添加模型
        </button>
      </div>
      <input
        className="model-search"
        aria-label={`搜索${mediaLabels[kind]}模型`}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="搜索模型名称或端点…"
      />
      {((!remove && error) || hub.error) && (
        <ErrorNotice error={(!remove && error) || hub.error} />
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
                  ? "已停用"
                  : m.plugin === "codex-image"
                    ? "本机 Codex"
                    : m.hasKey
                      ? "密钥已配置"
                      : localEndpoint(m.endpoint)
                        ? "本机连接"
                        : "待配置密钥"}
              </span>
            </div>
            <h3>{m.name}</h3>
            <p className="hub-endpoint" title={m.endpoint}>
              {m.endpoint}
            </p>
            <div className="hub-tags">
              <span>{mediaLabels[kind]}生成</span>
              <span>
                {m.plugin === "codex-image"
                  ? "Codex · 型号未验证"
                  : m.plugin === "fal"
                    ? "fal · 第三方"
                    : m.plugin === "gemini-native"
                      ? "Google · 官方直连"
                      : "自定义 HTTP"}
              </span>
            </div>
            <footer>
              <button
                disabled={busy}
                onClick={() =>
                  void act(() => hub.save({ ...m, enabled: !m.enabled }))
                }
              >
                {m.enabled ? "停用" : "启用"}
              </button>
              <button
                disabled={busy}
                onClick={() => setEdit(m)}
                aria-label={`配置 ${m.name}`}
              >
                <PencilSimple />
                配置
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
                  <button disabled={busy} aria-label={`移除 ${m.name}`}>
                    <Trash />
                  </button>
                </AlertDialog.Trigger>
                <AlertDialog.Content className="modal small" aria-busy={busy}>
                  <AlertDialog.Title>移除模型</AlertDialog.Title>
                  <AlertDialog.Description>
                    移除「{m.name}」的模型配置？已生成的素材和任务会保留。
                  </AlertDialog.Description>
                  {error && (
                    <ErrorNotice error={error} fallback="OPERATION_FAILED" />
                  )}
                  <footer>
                    <AlertDialog.Cancel>
                      <button type="button" disabled={busy}>
                        取消
                      </button>
                    </AlertDialog.Cancel>
                    <button
                      type="button"
                      className="primary"
                      disabled={busy}
                      onClick={() => void act(() => hub.remove(m.id))}
                    >
                      {busy ? "正在移除…" : "确认移除"}
                    </button>
                  </footer>
                </AlertDialog.Content>
              </AlertDialog.Root>
            </footer>
          </article>
        ))}
      </div>
      {!items.length && (
        <div className="hub-empty">
          <Icon size={34} />
          <h3>
            {hub.loading
              ? "读取模型…"
              : query
                ? "没有找到匹配模型"
                : `添加你的第一个${mediaLabels[kind]}模型`}
          </h3>
          <p>
            {query
              ? "试试其他名称或清空搜索。"
              : "选择一个模型起步，也可以接入自定义端点。"}
          </p>
          {!query && kind !== "audio" && (
            <button
              disabled={!native || busy || hub.loading}
              onClick={() => create(true)}
            >
              配置 {presets[kind].name}
            </button>
          )}
        </div>
      )}
      {kind === "audio" && (
        <div className="hub-local-note">
          <Waveform />
          <div>
            <strong>也可以使用本机配音</strong>
            <p>无需 API Key。在制作台的「配音」中选择已安装的系统声音。</p>
          </div>
        </div>
      )}
      <p className="model-hint">
        保存后可在生成面板选择此模型。启用仅代表允许选择；实际可用性取决于服务权限和模型参数。
      </p>
    </section>
  );
}
