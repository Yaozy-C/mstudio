import { t, useLanguage } from "../i18n";
import { selectMediaModel } from "./frameInputs";
import { mediaAdapter } from "../models/adapters";
import type { MediaModel } from "../models/mediaRegistry";
import type { ProductionTask } from "./types";
function method(model: MediaModel) {
  const fields = mediaAdapter(model).fields;
  return fields.some((f) => f.role === "first-frame")
    ? "frames"
    : fields.some((f) => f.role === "reference")
      ? "references"
      : "text";
}
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
  return (
    <div className="generation-setting-field">
      <span>{t("生成方式")}</span>
      <div className="composer-model-tabs">
        {(["references", "frames", "text"] as const).map((value) => {
          const target = available.find((m) => method(m) === value);
          return (
            <button
              type="button"
              key={value}
              disabled={!target}
              aria-pressed={!!current && method(current) === value}
              onClick={() => {
                if (!target) return;
                change(selectMediaModel(draft, target));
              }}
            >
              {value === "references"
                ? t("参考图/视频")
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
