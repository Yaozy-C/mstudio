import { useState } from "react";
import { StudioSelect } from "../ui/StudioSelect";
import { t } from "../i18n";
import { generationMethod } from "./modelGroups";
import { MediaModelForm } from "./MediaModelForm";
import type { MediaModel } from "./mediaRegistry";

/** Keep each mode's form mounted so switching modes retains unsaved edits. */
export function MediaModelModesForm({
  initial,
  variants,
  save,
  cancel,
}: {
  initial: MediaModel;
  variants: MediaModel[];
  save: (model: MediaModel) => Promise<void>;
  cancel: () => void;
}) {
  const [active, setActive] = useState(initial.id);
  return (
    <div>
      <label className="generation-setting-field">
        {t("生成方式")}
        <StudioSelect
          label={t("生成方式")}
          value={active}
          onValueChange={setActive}
          options={variants.map((m) => ({
            value: m.id,
            label:
              generationMethod(m) === "references"
                ? t("全参考")
                : generationMethod(m) === "frames"
                  ? t("首尾帧")
                  : t("文生视频"),
          }))}
        />
      </label>
      {variants.map((variant) => (
        <div key={variant.id} hidden={active !== variant.id}>
          <MediaModelForm
            initial={{ ...variant, name: initial.name }}
            save={save}
            cancel={cancel}
          />
        </div>
      ))}
    </div>
  );
}
