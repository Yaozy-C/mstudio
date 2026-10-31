import { t, useLanguage } from "../i18n";
import { selectMediaModel } from "./frameInputs";
import { generationMethod, modelGroupKey } from "../models/modelGroups";
import { Images, ImageSquare, TextT } from "@phosphor-icons/react";
import type { MediaModel } from "../models/mediaRegistry";
import type { ProductionTask } from "./types";
import "./composer-generation.css";
export function GenerationMethod({
  draft,
  models,
  change,
}: {
  draft: ProductionTask;
  models: MediaModel[];
  change: (value: Partial<ProductionTask>) => void;
}) {
  useLanguage();
  if (draft.kind !== "video") return null;
  const available = models.filter((m) => m.kind === "video" && m.enabled);
  const current = available.find((m) => m.id === draft.modelId);
  const variants = current
    ? available.filter((m) => modelGroupKey(m) === modelGroupKey(current))
    : [];
  const icons = { references: Images, frames: ImageSquare, text: TextT };
  return (
    <div className="generation-setting-field">
      <span>{t("生成方式")}</span>
      <div className="composer-model-tabs">
        {(["references", "frames", "text"] as const)
          .filter((value) =>
            variants.some((m) => generationMethod(m) === value),
          )
          .map((value) => {
            const target = variants.find((m) => generationMethod(m) === value);
            const Icon = icons[value];
            return (
              <button
                type="button"
                key={value}
                disabled={!target}
                aria-pressed={!!current && generationMethod(current) === value}
                onClick={() => {
                  if (!target || target.id === current?.id) return;
                  change(selectMediaModel(draft, target));
                }}
              >
                <Icon size={15} />
                {value === "references"
                  ? t("全参考")
                  : value === "frames"
                    ? t("首尾帧")
                    : t("文生视频")}
              </button>
            );
          })}
      </div>
    </div>
  );
}
