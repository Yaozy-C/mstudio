import { AsyncButton } from "../ui/AsyncState";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useEffect, useId, useRef, useState } from "react";
import { normalizeError } from "../errors/catalog";
import {
  validateService,
  type ServiceErrors,
  type ServiceField,
} from "./serviceValidation";
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
  const [key, setKey] = useState("");
  const [clearKey, setClearKey] = useState(false);
  const [fields, setFields] = useState<ServiceErrors>({});
  const fieldId = useId();
  const inputs = useRef<Partial<Record<ServiceField, HTMLInputElement | null>>>(
    {},
  );
  function showFields(next: ServiceErrors) {
    setFields(next);
    const first = (["name", "endpoint", "key"] as const).find(
      (field) => next[field],
    );
    if (first) inputs.current[first]?.focus();
  }
  function changed(field: ServiceField) {
    setFields((current) => ({ ...current, [field]: undefined }));
    setError("");
  }
  function fieldError(field: ServiceField) {
    return fields[field] ? (
      <small
        id={`${fieldId}-${field}`}
        className="model-field-error"
        role="alert"
      >
        {t(fields[field]!)}
      </small>
    ) : null;
  }
  function fieldProps(field: ServiceField) {
    return {
      ref: (node: HTMLInputElement | null) => {
        inputs.current[field] = node;
      },
      "aria-invalid": !!fields[field],
      "aria-describedby": fields[field] ? `${fieldId}-${field}` : undefined,
    };
  }
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    if (busy) return;
    const first = (["name", "endpoint", "key"] as const).find(
      (field) => fields[field],
    );
    if (first) inputs.current[first]?.focus();
  }, [busy, fields]);
  const presets = servicePresets.filter((p) => kinds.includes(p.kind));
  async function save() {
    setError("");
    const issues = validateService(service, initial, key, clearKey);
    showFields(issues);
    if (Object.keys(issues).length) return;
    setBusy(true);
    try {
      await bridge("save_service_connection", {
        connection: service,
        key: key.trim() || null,
        clearKey,
      });
      serviceChanged();
      saved({
        ...service,
        hasKey: !!key.trim() || (!clearKey && initial.hasKey),
      });
    } catch (e) {
      const message = normalizeError(e, "VALIDATION_FAILED").message;
      const field = /密钥|API Key/.test(message)
        ? "key"
        : /地址|URL|url/.test(message)
          ? "endpoint"
          : /连接名称/.test(message)
            ? "name"
            : null;
      if (field) showFields({ [field]: message });
      else setError(String(e));
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
                    setKey("");
                    setClearKey(false);
                    setFields({});
                  }
                }}
              />
            </label>
          )}
          {service.kind === "dashscope" && (
            <p className="model-hint model-field-wide">
              填写业务空间专属地址，例如
              https://你的业务空间ID.cn-beijing.maas.aliyuncs.com；API Key
              须属于同一地域。
            </p>
          )}
          {service.kind !== "codex" && (
            <>
              <label>
                {t("连接名称")}
                <input
                  {...fieldProps("name")}
                  required
                  maxLength={80}
                  value={service.name}
                  onChange={(e) => {
                    setService({ ...service, name: e.target.value });
                    changed("name");
                  }}
                  placeholder={t("例如：Google 工作账号")}
                />
                {fieldError("name")}
              </label>
              <label className="model-field-wide">
                {t("服务地址")}
                <input
                  required
                  {...fieldProps("endpoint")}
                  type="url"
                  disabled={service.kind === "fal"}
                  value={service.endpoint}
                  onChange={(e) => {
                    setService({ ...service, endpoint: e.target.value });
                    changed("endpoint");
                    changed("key");
                  }}
                  placeholder={t("https://… 或本机服务地址")}
                />
                {fieldError("endpoint")}
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
                  {...fieldProps("key")}
                  value={key}
                  disabled={clearKey}
                  onChange={(e) => {
                    setKey(e.target.value);
                    changed("key");
                  }}
                  placeholder={
                    initial.hasKey
                      ? t("已保存，留空保持原密钥")
                      : t("本机无认证服务可留空")
                  }
                />
                {fieldError("key")}
              </label>
              {initial.hasKey && (
                <label className="service-clear-key">
                  <input
                    type="checkbox"
                    checked={clearKey}
                    onChange={(e) => {
                      setClearKey(e.target.checked);
                      setKey("");
                      changed("key");
                    }}
                  />
                  {t("移除已保存密钥")}
                </label>
              )}
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
                disabled={busy}
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
