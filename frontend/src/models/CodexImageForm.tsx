import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useState } from "react";
import { bridge, native } from "../bridge";
import { MagnifyingGlass, SignIn } from "@phosphor-icons/react";
import type { MediaModel } from "./mediaRegistry";

type Status =
  | "idle"
  | "ready"
  | "missing"
  | "login_required"
  | "unsupported"
  | "unavailable"
  | "timeout";
const messages: Record<Status, string> = {
  idle: "自动查找 Codex、检查登录与生图能力，无需填写。",
  ready: "Codex 已就绪。",
  missing: "未找到 Codex，安装后点击重新检测。",
  login_required: "登录 ChatGPT 后即可继续检测。",
  unsupported: "当前 Codex 未提供生图能力，请更新后重试。",
  unavailable: "暂时无法连接 Codex，请确认 Codex 可正常运行后重试。",
  timeout: "连接 Codex 超时，请重试。",
};
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
  const [busy, setBusy] = useState(false);
  const [loggingIn, setLoggingIn] = useState(false);
  const [status, setStatus] = useState<Status>("idle");
  const [error, setError] = useState("");
  async function connect(login = false) {
    setBusy(true);
    setLoggingIn(login);
    setError("");
    try {
      if (login) await bridge("codex_login");
      setLoggingIn(false);
      const result = await bridge<{ status: Status }>("codex_image_status");
      setStatus(result.status);
      if (result.status === "ready") {
        await save({
          ...initial,
          name: initial.name.trim() || "Codex",
          params: { model: "codex-image", n: 1 },
          connectionId: null,
          enabled: true,
        });
        cancel();
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
      setLoggingIn(false);
    }
  }
  async function download() {
    setError("");
    try {
      await bridge("open_codex_download");
    } catch (e) {
      setError(String(e));
    }
  }
  return (
    <section className="model-form" aria-busy={busy}>
      <h3>{t("连接 Codex 生图")}</h3>
      <p role="status" aria-live="polite">
        {!native
          ? t("请在桌面应用中检测本机 Codex。")
          : loggingIn
            ? t("请在浏览器完成登录，完成后将自动检测。")
            : busy
              ? t("正在检测本机 Codex…")
              : t(messages[status])}
      </p>
      {error && <ErrorNotice error={error} fallback="VALIDATION_FAILED" />}
      <footer>
        <button disabled={busy} onClick={cancel}>
          {t("取消")}
        </button>
        {(status === "missing" || status === "unsupported") && (
          <button disabled={busy} onClick={() => void download()}>
            {t(status === "missing" ? "安装 Codex" : "更新 Codex")}
          </button>
        )}
        <button
          className="primary"
          disabled={busy || !native}
          onClick={() => void connect(status === "login_required")}
        >
          {status === "login_required" ? (
            <SignIn size={16} />
          ) : (
            <MagnifyingGlass size={16} />
          )}
          {busy
            ? t("正在连接…")
            : t(
                status === "login_required"
                  ? "登录 ChatGPT"
                  : status === "idle"
                    ? "一键检测并启用"
                    : "重新检测",
              )}
        </button>
      </footer>
    </section>
  );
}
