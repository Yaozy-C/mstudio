import { AsyncButton } from "../ui/AsyncState";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useState } from "react";
import { bridge } from "../bridge";
import { CodexServiceForm } from "./CodexServiceForm";
import { StudioSelect } from "../ui/StudioSelect";
import {
  serviceChanged,
  serviceLabels,
  servicePresets,
  type ServiceConnection,
  type ServiceKind,
} from "./connectionStore";
export function ServiceForm({
  initial,
  kinds,
  cancel,
  saved,
  existing = false,
}: {
  initial: ServiceConnection;
  kinds: ServiceKind[];
  cancel: () => void;
  saved: (service: ServiceConnection) => void;
  existing?: boolean;
}) {
  useLanguage();
  const [service, setService] = useState(initial);
  const [key, setKey] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const presets = servicePresets.filter((p) => kinds.includes(p.kind));
  async function save() {
    setBusy(true);
    setError("");
    try {
      await bridge("save_service_connection", {
        connection: service,
        key: key?.trim() || null,
        clearKey: false,
      });
      serviceChanged();
      saved({ ...service, hasKey: !!key?.trim() || initial.hasKey });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="model-form service-form">
      <h3>{existing ? t("编辑服务连接") : t("添加服务连接")}</h3>
      <fieldset disabled={busy}>
        <div className="model-form-grid">
          {!existing && (
            <label>
              {t("服务")}
              <StudioSelect
                label={t("服务预设")}
                value={
                  presets.find(
                    (p) =>
                      p.endpoint === service.endpoint &&
                      p.kind === service.kind,
                  )?.name || ""
                }
                placeholder={t("选择服务或手动填写")}
                options={presets.map((p) => ({ value: p.name, label: p.name }))}
                onValueChange={(name) => {
                  const p = presets.find((p) => p.name === name);
                  if (p) {
                    setService({ ...service, ...p });
                    setKey(null);
                  }
                }}
              />
            </label>
          )}
          {service.kind !== "codex" && (
            <>
              <label>
                {t("连接名称")}
                <input
                  required
                  maxLength={80}
                  value={service.name}
                  onChange={(e) =>
                    setService({ ...service, name: e.target.value })
                  }
                  placeholder={t("例如：Google 工作账号")}
                />
              </label>
              <label className="model-field-wide">
                {t("服务地址")}
                <input
                  required
                  type="url"
                  disabled={service.kind === "fal"}
                  value={service.endpoint}
                  onChange={(e) =>
                    setService({ ...service, endpoint: e.target.value })
                  }
                  placeholder={t("https://… 或本机服务地址")}
                />
              </label>
              <label>
                {t("接口协议")}
                <StudioSelect
                  label={t("服务协议")}
                  disabled={existing || service.kind === "fal"}
                  value={service.kind}
                  options={kinds.map((kind) => ({
                    value: kind,
                    label: t(serviceLabels[kind]),
                  }))}
                  onValueChange={(value) =>
                    setService({
                      ...service,
                      kind: value as ServiceKind,
                      endpoint:
                        value === "codex"
                          ? "codex://local"
                          : value === "fal"
                            ? "https://queue.fal.run"
                            : service.endpoint,
                    })
                  }
                />
              </label>
              <label>
                API Key
                <input
                  type="password"
                  autoComplete="new-password"
                  value={key ?? (initial.hasKey ? "••••••••••••••••" : "")}
                  onFocus={() => {
                    if (key === null) setKey("");
                  }}
                  onBlur={() => {
                    if (!key?.trim()) setKey(null);
                  }}
                  onChange={(e) => setKey(e.target.value)}
                  placeholder={
                    initial.hasKey
                      ? t("输入新密钥以替换")
                      : t("本机无认证服务可留空")
                  }
                />
              </label>
            </>
          )}
        </div>
        {service.kind === "codex" ? (
          <CodexServiceForm
            connection={service}
            onBusyChange={setBusy}
            saved={saved}
            cancel={cancel}
          />
        ) : (
          <>
            {error && (
              <ErrorNotice error={error} fallback="VALIDATION_FAILED" />
            )}
            <footer>
              <button type="button" onClick={cancel}>
                {t("取消")}
              </button>
              <AsyncButton
                busy={busy}
                busyLabel={t("保存中…")}
                type="button"
                className="primary"
                disabled={
                  busy || !service.name.trim() || !service.endpoint.trim()
                }
                onClick={() => void save()}
              >
                {t("保存连接")}
              </AsyncButton>
            </footer>
          </>
        )}
      </fieldset>
    </section>
  );
}
