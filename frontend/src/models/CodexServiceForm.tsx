import { AsyncButton } from "../ui/AsyncState";
import { useState } from "react";
import { MagnifyingGlass, SignIn } from "@phosphor-icons/react";
import { bridge, native } from "../bridge";
import { t } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { serviceChanged, type ServiceConnection } from "./connectionStore";

export function CodexServiceForm({
  connection,
  saved,
  cancel,
  onBusyChange,
}: {
  connection: ServiceConnection;
  saved: (connection: ServiceConnection) => void;
  cancel: () => void;
  onBusyChange?: (busy: boolean) => void;
}) {
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("idle");
  const [error, setError] = useState("");
  async function connect() {
    setBusy(true);
    onBusyChange?.(true);
    setError("");
    try {
      if (status === "login_required") await bridge("codex_login");
      const result = await bridge<{
        status: string;
        connection?: ServiceConnection;
      }>("connect_codex_service", { connection });
      setStatus(result.status);
      if (result.connection) {
        serviceChanged();
        saved(result.connection);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
      onBusyChange?.(false);
    }
  }
  return (
    <div className="codex-service-setup">
      <p role="status">
        {t(
          !native
            ? "请在桌面应用中检测本机 Codex。"
            : busy
              ? "正在连接 Codex，若打开浏览器请完成登录。"
              : status === "login_required"
                ? "登录 ChatGPT 后即可继续检测。"
                : status === "missing"
                  ? "未找到 Codex，安装后点击重新检测。"
                  : status === "unavailable"
                    ? "暂时无法连接 Codex，请确认 Codex 可正常运行后重试。"
                    : "自动连接本机 Codex，同步对话模型和可用的生图能力。",
        )}
      </p>
      {error && <ErrorNotice error={error} fallback="VALIDATION_FAILED" />}
      <footer>
        <button type="button" disabled={busy} onClick={cancel}>
          {t("取消")}
        </button>
        {status === "missing" && (
          <button
            type="button"
            onClick={() =>
              void bridge("open_codex_download").catch((e) =>
                setError(String(e)),
              )
            }
          >
            {t("安装 Codex")}
          </button>
        )}
        <AsyncButton
          busy={busy}
          busyLabel={t("正在连接…")}
          type="button"
          className="primary"
          disabled={busy || !native}
          onClick={() => void connect()}
        >
          {status === "login_required" ? (
            <SignIn size={16} />
          ) : (
            <MagnifyingGlass size={16} />
          )}
          {t(status === "login_required" ? "登录 ChatGPT" : "一键连接")}
        </AsyncButton>
      </footer>
    </div>
  );
}
