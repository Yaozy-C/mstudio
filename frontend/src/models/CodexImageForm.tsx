import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useState } from "react";
import { bridge } from "../bridge";
import type { MediaModel } from "./mediaRegistry";
export function CodexImageForm({
  initial,
  save,
  cancel,
}: {
  initial: MediaModel;
  save: (model: MediaModel) => Promise<void>;
  cancel: () => void;
}) {
  useLanguage();
  const [name, setName] = useState(initial.name);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  async function check() {
    const result = await bridge<{
      loggedIn: boolean;
      imageGeneration: boolean;
    }>("codex_image_status");
    if (!result.loggedIn)
      throw new Error(t("请先在本机 Codex 登录 ChatGPT 账号。"));
    if (!result.imageGeneration)
      throw new Error(t("当前 Codex 不支持原生生图，请更新 Codex。"));
    setMessage(t("已连接本机 Codex，ChatGPT 登录有效，原生生图可用。"));
  }
  async function act(persist: boolean) {
    setBusy(true);
    setError("");
    setMessage("");
    try {
      await check();
      if (persist) {
        await save({
          ...initial,
          name: name.trim(),
          params: { model: "codex-image", n: 1 },
          connectionId: null,
        });
        cancel();
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="model-form">
      <h3>{t("配置 Codex 生图")}</h3>
      <p>{t("使用本机 Codex 的 ChatGPT 登录，无需 API Key。")}</p>
      <p className="model-hint">
        {t(
          "Image 2.5 型号未验证：底层图片模型由 Codex 管理，当前接口无法选择或核验 Sunburst / Flare。每次生成一张 PNG，支持最多 5 张项目参考图；尺寸和质量由 Codex 决定。",
        )}
      </p>
      <label>
        {t("模型名称")}
        <input
          value={name}
          maxLength={80}
          disabled={busy}
          onChange={(e) => setName(e.target.value)}
        />
      </label>
      {message && <p role="status">{message}</p>}
      {error && <ErrorNotice error={error} fallback="VALIDATION_FAILED" />}
      <footer>
        <button disabled={busy} onClick={cancel}>
          {t("取消")}
        </button>
        <button disabled={busy} onClick={() => void act(false)}>
          {t("检测本机 Codex")}
        </button>
        <button
          className="primary"
          disabled={busy || !name.trim()}
          onClick={() => void act(true)}
        >
          {busy ? t("正在连接…") : t("保存并使用")}
        </button>
      </footer>
    </section>
  );
}
