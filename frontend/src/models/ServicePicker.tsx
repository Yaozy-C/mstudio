import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useState } from "react";
import { StudioSelect } from "../ui/StudioSelect";
import { ServiceForm } from "./ServiceForm";
import {
  useServiceConnections,
  type ServiceConnection,
  type ServiceKind,
} from "./connectionStore";
export function ServicePicker({
  id,
  kinds,
  suggested,
  onChange,
}: {
  id?: string | null;
  kinds: ServiceKind[];
  suggested?: { kind: ServiceKind; endpoint: string };
  onChange: (service: ServiceConnection) => void;
}) {
  useLanguage();
  const { connections, error, loading } = useServiceConnections();
  const [adding, setAdding] = useState(false);
  const available = connections.filter((s) => kinds.includes(s.kind));
  const selected = available.find((s) => s.id === id);
  const kind =
    suggested && kinds.includes(suggested.kind) ? suggested.kind : kinds[0];
  return (
    <div className="model-field-wide service-picker">
      <label>
        {t("服务连接")}
        <StudioSelect
          label={t("模型服务连接")}
          value={id || ""}
          placeholder={loading ? t("加载中…") : t("选择已有连接")}
          disabled={loading}
          options={available.map((s) => ({
            value: s.id,
            label: s.name + (s.hasKey ? "" : t(" · 未配置密钥")),
          }))}
          onValueChange={(id) => {
            const service = available.find((s) => s.id === id);
            if (service) onChange(service);
          }}
        />
      </label>
      {selected && (
        <p className="model-hint">
          {selected.endpoint} {t("· 地址和密钥在“服务连接”中统一管理")}
        </p>
      )}
      {error && <ErrorNotice error={error} fallback="OPERATION_FAILED" />}
      {!adding && (
        <button type="button" onClick={() => setAdding(true)}>
          {t("添加服务连接")}
        </button>
      )}
      {adding && (
        <ServiceForm
          key={kind}
          kinds={kinds}
          initial={{
            id: crypto.randomUUID(),
            name: "",
            kind,
            endpoint:
              (kind === "fal"
                ? "https://queue.fal.run"
                : suggested?.endpoint) ||
              (kind === "fal" ? "https://queue.fal.run" : ""),
            hasKey: false,
            modelCount: 0,
          }}
          cancel={() => setAdding(false)}
          saved={(service) => {
            onChange(service);
            setAdding(false);
          }}
        />
      )}
    </div>
  );
}
