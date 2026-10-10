import { t, useLanguage } from "../i18n";
import type { ModelConnection } from "./types";
import { inputLabels } from "./inputCapabilities";

export function ModelInputs({
  profile,
  update,
}: {
  profile: ModelConnection;
  update: (value: Partial<ModelConnection>) => void;
}) {
  useLanguage();
  return (
    <section className="model-field-wide model-inputs">
      <strong>{t("输入类型")}</strong>
      <div className="model-input-options">
        {(Object.keys(inputLabels) as (keyof ModelConnection["inputs"])[]).map(
          (kind) => (
            <label className="model-input-option" key={kind}>
              <input
                type="checkbox"
                checked={profile.inputs[kind]}
                onChange={(e) =>
                  update({
                    inputs: { ...profile.inputs, [kind]: e.target.checked },
                  })
                }
              />
              <span>{t(inputLabels[kind])}</span>
            </label>
          ),
        )}
      </div>
    </section>
  );
}
