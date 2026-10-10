import { AsyncButton } from "../ui/AsyncState";
import { t, useLanguage } from "../i18n";
import { normalizeError } from "../errors/catalog";
import { ErrorNotice } from "../errors/ErrorNotice";
import { connectionIssue } from "./connectionIssue";
import { ModelInputs } from "./ModelInputs";
import { StudioSelect } from "../ui/StudioSelect";
import { useState } from "react";
import { bridge } from "../bridge";
import { type ModelConnection } from "./types";
import { ServicePicker } from "./ServicePicker";
import { textServiceKinds } from "./connectionStore";
export function ModelForm({
  initial,
  saved,
  cancel,
}: {
  initial: ModelConnection;
  saved: () => void;
  cancel: () => void;
}) {
  useLanguage();
  const [profile, setProfile] = useState(initial);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [discovering, setDiscovering] = useState(false);
  const [available, setAvailable] = useState<{ id: string; name: string }[]>(
    [],
  );
  const [discoveryNotice, setDiscoveryNotice] = useState("");
  const issue = connectionIssue(profile);
  const update = (value: Partial<ModelConnection>) =>
    setProfile((p) => ({ ...p, ...value }));
  async function discover() {
    if (!profile.connectionId) return;
    setDiscovering(true);
    setDiscoveryNotice("");
    try {
      const rows = await bridge<{ id: string; name: string }[]>(
        "discover_service_models",
        { id: profile.connectionId },
      );
      setAvailable(rows);
      setDiscoveryNotice(
        rows.length
          ? t("获取到 {v0} 个型号。输入能力仍需按服务实际支持情况设置。", {
              v0: rows.length,
            })
          : t("服务未返回型号，可以手动填写模型 ID。"),
      );
    } catch (e) {
      setDiscoveryNotice(
        t("{v0}，也可以手动填写模型 ID。", { v0: normalizeError(e).message }),
      );
    } finally {
      setDiscovering(false);
    }
  }
  return (
    <form
      className="model-form"
      onSubmit={(e) => {
        e.preventDefault();
        if (issue || !profile.connectionId) return;
        setBusy(true);
        setError("");
        void bridge("save_model", {
          profile,
          key: null,
          clearKey: false,
        })
          .then(saved)
          .catch((e) => setError(String(e)))
          .finally(() => setBusy(false));
      }}
    >
      <div className="model-section-heading">
        <h3>{initial.name ? t("编辑模型") : t("添加模型")}</h3>
        <span>{t("用于 Agent 对话、脚本和分镜创作")}</span>
      </div>
      <fieldset disabled={busy || discovering}>
        <div className="model-form-grid">
          <ServicePicker
            id={profile.connectionId}
            kinds={textServiceKinds}
            suggested={{ kind: profile.adapter, endpoint: profile.endpoint }}
            onChange={(service) => {
              update({
                connectionId: service.id,
                endpoint: service.endpoint,
                adapter: service.kind as ModelConnection["adapter"],
                hasKey: service.hasKey,
              });
              setAvailable([]);
              setDiscoveryNotice("");
            }}
          />
          <label>
            {t("模型名称")}
            <input
              autoFocus
              required
              maxLength={80}
              placeholder={t("例如：日常创作")}
              value={profile.name}
              onChange={(e) => update({ name: e.target.value })}
            />
          </label>
          <label>
            {t("模型 ID")}
            <input
              required
              maxLength={255}
              placeholder={t("填写服务提供的模型名称")}
              value={profile.model}
              onChange={(e) => update({ model: e.target.value })}
            />
          </label>
          <div className="model-field-wide">
            <button
              type="button"
              disabled={!profile.connectionId || discovering}
              onClick={() => void discover()}
            >
              {discovering ? t("正在获取…") : t("获取可用型号")}
            </button>
            {!!available.length && (
              <StudioSelect
                label={t("从服务选择型号")}
                value={profile.model}
                placeholder={t("选择一个型号")}
                options={available.map((m) => ({
                  value: m.id,
                  label: m.name === m.id ? m.id : `${m.name} · ${m.id}`,
                }))}
                onValueChange={(model) => update({ model })}
              />
            )}
            {discoveryNotice && (
              <p className="model-hint" role="status">
                {discoveryNotice}
              </p>
            )}
          </div>
          <label>
            {t("上下文窗口（token）")}
            <input
              type="number"
              min={8192}
              max={2000000}
              step={1024}
              value={profile.contextWindow ?? 32768}
              onChange={(e) =>
                update({ contextWindow: Number(e.target.value) })
              }
            />
            <small>{t("按模型提供方公布的窗口填写，用于提前压缩对话。")}</small>
          </label>
          <ModelInputs profile={profile} update={update} />
        </div>
        {(error || issue) && (
          <ErrorNotice error={error || issue} fallback="VALIDATION_FAILED" />
        )}
        <footer>
          <button type="button" onClick={cancel}>
            {t("取消")}
          </button>
          <AsyncButton
            busy={busy}
            busyLabel={t("保存中…")}
            type="submit"
            className="primary"
            disabled={
              busy ||
              !!issue ||
              !profile.connectionId ||
              !profile.name.trim() ||
              !profile.model.trim()
            }
          >
            {t("保存模型")}
          </AsyncButton>
        </footer>
      </fieldset>
    </form>
  );
}
