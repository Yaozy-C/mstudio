import { StudioSelect } from "../ui/StudioSelect";
import { ModelMark } from "../ui/Identity";
import { t } from "../i18n";
import type { MediaModel } from "./mediaRegistry";
import { groupMediaModels, modelGroupKey, modelInGroup } from "./modelGroups";

export function MediaModelSelect({
  models,
  value,
  change,
}: {
  models: MediaModel[];
  value: string;
  change: (model: MediaModel) => void;
}) {
  const current = models.find((m) => m.id === value);
  const groups = groupMediaModels(models);
  return (
    <StudioSelect
      label={t("生成模型")}
      placeholder={value && !current ? t("原模型不可用") : t("选择模型")}
      value={current ? modelGroupKey(current) : ""}
      options={groups.map((group) => ({
        value: group.key,
        label: group.name,
        disabled: !group.variants.some((m) => m.enabled),
        icon: (
          <ModelMark
            identity={`${group.name} ${group.variants[0].plugin}`}
            size={18}
          />
        ),
      }))}
      onValueChange={(key) => {
        const group = groups.find((g) => g.key === key);
        if (group) change(modelInGroup(group, current));
      }}
    />
  );
}
