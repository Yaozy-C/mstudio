import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useEffect, useState } from "react";
import { Plus, PencilSimple, Trash, X } from "@phosphor-icons/react";
import { bridge, native } from "../bridge";
import { ModelMark } from "../ui/Identity";
import { ServiceForm } from "./ServiceForm";
import {
  serviceChanged,
  serviceLabels,
  useServiceConnections,
  type ServiceConnection,
  type ServiceKind,
} from "./connectionStore";
const kinds = Object.keys(serviceLabels) as ServiceKind[];
export function ServiceConnections() {
  useLanguage();
  const hub = useServiceConnections();
  const [edit, setEdit] = useState<ServiceConnection | null>(null);
  const [remove, setRemove] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(() => setNotice(""), 5000);
    return () => window.clearTimeout(timer);
  }, [notice]);
  if (edit)
    return (
      <ServiceForm
        initial={edit}
        kinds={kinds}
        existing={hub.connections.some((s) => s.id === edit.id)}
        cancel={() => setEdit(null)}
        saved={() => {
          setEdit(null);
          setNotice(t("服务连接已保存"));
        }}
      />
    );
  async function destroy(id: string) {
    setBusy(true);
    setError("");
    try {
      await bridge("remove_service_connection", { id });
      serviceChanged();
      setRemove(null);
      setNotice(t("服务连接已移除"));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section>
      <div className="hub-section-head">
        <div>
          <h3>{t("服务连接")}</h3>
          <p>{t("连接一次，供多个对话、图像或视频模型使用。")}</p>
        </div>
        <button
          className="primary"
          disabled={!native || busy}
          onClick={() =>
            setEdit({
              id: crypto.randomUUID(),
              name: "",
              kind: "openai-responses",
              endpoint: "https://api.openai.com/v1",
              hasKey: false,
              modelCount: 0,
            })
          }
        >
          <Plus />
          {t("添加连接")}
        </button>
      </div>
      {(error || hub.error) && (
        <ErrorNotice error={error || hub.error} fallback="OPERATION_FAILED" />
      )}
      {notice && (
        <div className="model-toast" role="status">
          <span>{notice}</span>
          <button
            type="button"
            aria-label={t("关闭提示")}
            onClick={() => setNotice("")}
          >
            <X />
          </button>
        </div>
      )}
      <div className="model-list">
        {hub.connections.map((service) => (
          <article className="model-row" key={service.id}>
            <div className="model-logo">
              <ModelMark identity={`${service.name} ${service.endpoint}`} />
            </div>
            <div className="model-row-copy">
              <strong>{service.name}</strong>
              <p>{service.endpoint}</p>
              <small>
                {t(serviceLabels[service.kind])} · {service.modelCount}{" "}
                {t("个模型 ·")}{" "}
                {service.hasKey ? t("密钥已配置") : t("未配置密钥")}
              </small>
            </div>
            <div className="model-row-actions">
              <button
                disabled={busy}
                aria-label={t("编辑 {v0}", { v0: service.name })}
                onClick={() => setEdit(service)}
              >
                <PencilSimple />
              </button>
              <button
                disabled={busy}
                aria-label={t("移除 {v0}", { v0: service.name })}
                onClick={() => setRemove(service.id)}
              >
                <Trash />
              </button>
            </div>
            {remove === service.id && (
              <div className="model-remove">
                <span>
                  {service.modelCount
                    ? t(
                        "还有 {v0} 个模型使用此连接，请先更换连接或移除模型。",
                        { v0: service.modelCount },
                      )
                    : t("移除「{v0}」及本机密钥？", { v0: service.name })}
                </span>
                <button type="button" onClick={() => setRemove(null)}>
                  {t("取消")}
                </button>
                {!service.modelCount && (
                  <button
                    disabled={busy}
                    onClick={() => void destroy(service.id)}
                  >
                    {t("确认移除")}
                  </button>
                )}
              </div>
            )}
          </article>
        ))}
      </div>
      {!hub.loading && !hub.connections.length && (
        <p className="model-hint">
          {t("先添加一个服务连接，再从模型库选择型号。")}
        </p>
      )}
    </section>
  );
}
