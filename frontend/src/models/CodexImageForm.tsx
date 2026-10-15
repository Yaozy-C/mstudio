import { X as DialogX } from "@phosphor-icons/react";
import { useState } from "react";
import { AsyncButton } from "../ui/AsyncState";
import { ErrorNotice } from "../errors/ErrorNotice";
import { ServicePicker } from "./ServicePicker";
import { codexImageModel } from "./codexImageModel";
import type { MediaModel } from "./mediaRegistry";
import { t, useLanguage } from "../i18n";
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
  const [model, setModel] = useState(initial);
  const [modelId, setModelId] = useState(() => codexImageModel(initial.params));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  async function submit() {
    setBusy(true);
    setError("");
    try {
      await save({
        ...model,
        name: model.name.trim(),
        params: { ...model.params, model: modelId.trim() },
      });
      cancel();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <form
      className="model-form"
      onSubmit={(e) => {
        e.preventDefault();
        if (!busy && model.connectionId) void submit();
      }}
    >
      <h3>{t("配置模型")}</h3>
      <fieldset disabled={busy}>
        <div className="model-form-grid">
          <label>
            {t("模型名称")}
            <input
              autoFocus
              required
              maxLength={80}
              value={model.name}
              onChange={(e) => setModel({ ...model, name: e.target.value })}
            />
          </label>
          <ServicePicker
            id={model.connectionId}
            kinds={["codex"]}
            suggested={{ kind: "codex", endpoint: "codex://local" }}
            onChange={(service) =>
              setModel({ ...model, connectionId: service.id })
            }
          />
          <label className="model-field-wide">
            {t("模型 ID")}
            <input
              required
              maxLength={100}
              value={modelId}
              onChange={(e) => setModelId(e.target.value)}
              placeholder="gpt-image-2.5-sunburst"
            />
            <small>
              {t(
                "按此模型 ID 请求生成和编辑；实际可用性取决于 Codex 账号权限，失败时不会自动切换型号。",
              )}
            </small>
          </label>
        </div>
      </fieldset>
      {error && <ErrorNotice error={error} fallback="VALIDATION_FAILED" />}
      <footer>
        <button type="button" disabled={busy} onClick={cancel}>
          <DialogX aria-hidden="true" />
          {t("取消")}
        </button>
        <AsyncButton
          type="submit"
          className="primary"
          busy={busy}
          disabled={busy || !model.connectionId}
        >
          {t("保存")}
        </AsyncButton>
      </footer>
    </form>
  );
}
