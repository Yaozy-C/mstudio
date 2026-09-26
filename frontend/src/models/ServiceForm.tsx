import { ErrorNotice } from "../errors/ErrorNotice";
import { useState } from "react";
import { bridge } from "../bridge";
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
  const [service, setService] = useState(initial);
  const [key, setKey] = useState("");
  const [clear, setClear] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const presets = servicePresets.filter((p) => kinds.includes(p.kind));
  async function save() {
    setBusy(true);
    setError("");
    try {
      await bridge("save_service_connection", {
        connection: service,
        key: key.trim() || null,
        clearKey: clear,
      });
      serviceChanged();
      saved({ ...service, hasKey: !!key.trim() || (!clear && initial.hasKey) });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="model-form service-form">
      <h3>{existing ? "编辑服务连接" : "添加服务连接"}</h3>
      <fieldset disabled={busy}>
        <div className="model-form-grid">
          {!existing && (
            <label>
              服务
              <StudioSelect
                label="服务预设"
                value={
                  presets.find(
                    (p) =>
                      p.endpoint === service.endpoint &&
                      p.kind === service.kind,
                  )?.name || ""
                }
                placeholder="选择服务或手动填写"
                options={presets.map((p) => ({ value: p.name, label: p.name }))}
                onValueChange={(name) => {
                  const p = presets.find((p) => p.name === name);
                  if (p) {
                    setService({ ...service, ...p });
                    setKey("");
                    setClear(false);
                  }
                }}
              />
            </label>
          )}
          <label>
            连接名称
            <input
              required
              maxLength={80}
              value={service.name}
              onChange={(e) => setService({ ...service, name: e.target.value })}
              placeholder="例如：Google 工作账号"
            />
          </label>
          <label className="model-field-wide">
            服务地址
            <input
              required
              type="url"
              disabled={service.kind === "fal"}
              value={service.endpoint}
              onChange={(e) =>
                setService({ ...service, endpoint: e.target.value })
              }
              placeholder="https://… 或本机服务地址"
            />
          </label>
          <label>
            接口协议
            <StudioSelect
              label="服务协议"
              disabled={existing || service.kind === "fal"}
              value={service.kind}
              options={kinds.map((kind) => ({
                value: kind,
                label: serviceLabels[kind],
              }))}
              onValueChange={(value) =>
                setService({
                  ...service,
                  kind: value as ServiceKind,
                  endpoint:
                    value === "fal"
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
              value={key}
              onChange={(e) => setKey(e.target.value)}
              placeholder={
                initial.hasKey
                  ? "已保存，留空保持原密钥"
                  : "本机无认证服务可留空"
              }
            />
          </label>
          {initial.hasKey && (
            <label className="model-check">
              <input
                type="checkbox"
                checked={clear}
                onChange={(e) => setClear(e.target.checked)}
              />
              移除已保存密钥
            </label>
          )}
        </div>
        <p className="model-hint">
          使用此连接的模型共用地址与密钥。保存不会发起生成请求。
        </p>
        {error && <ErrorNotice error={error} fallback="VALIDATION_FAILED" />}
        <footer>
          <button type="button" onClick={cancel}>
            取消
          </button>
          <button
            type="button"
            className="primary"
            disabled={busy || !service.name.trim() || !service.endpoint.trim()}
            onClick={() => void save()}
          >
            {busy ? "保存中…" : "保存连接"}
          </button>
        </footer>
      </fieldset>
    </section>
  );
}
