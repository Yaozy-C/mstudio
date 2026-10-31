import type { MediaModel } from "./mediaRegistry";
import { mediaAdapter } from "./adapters";
import { modelLibrary } from "./catalogSpecs";

export type GenerationMethod = "references" | "frames" | "text";
export type ModelGroup = {
  key: string;
  name: string;
  variants: MediaModel[];
};

export function generationMethod(model: MediaModel): GenerationMethod {
  const fields = mediaAdapter(model).fields;
  return fields.some((f) => f.role === "first-frame" || f.role === "last-frame")
    ? "frames"
    : fields.some((f) => f.role === "reference")
      ? "references"
      : "text";
}

export function modelGroupKey(model: MediaModel) {
  const family = catalogModelFamily(model);
  if (family)
    return JSON.stringify([model.plugin, model.connectionId, family.id]);
  return model.id;
}

export function catalogModelFamily(model: MediaModel) {
  return modelLibrary.find(
    (spec) =>
      spec.family &&
      (spec.plugin ?? "fal") === model.plugin &&
      (spec.endpoint === model.endpoint ||
        (model.plugin === "dashscope" &&
          spec.request.model === model.params.model)),
  )?.family;
}

export function modelDisplayName(model: MediaModel) {
  return catalogModelFamily(model)
    ? model.name.replace(/ · (首帧|首尾帧|全参考|文生视频|参考生成)$/, "")
    : model.name;
}

/** Entry points remain available for saved jobs, while a model appears once in the UI. */
export function groupMediaModels(models: MediaModel[]): ModelGroup[] {
  const groups = new Map<string, ModelGroup>();
  for (const model of models) {
    const key = modelGroupKey(model);
    const group = groups.get(key);
    if (group) group.variants.push(model);
    else
      groups.set(key, {
        key,
        name: modelDisplayName(model),
        variants: [model],
      });
  }
  return [...groups.values()];
}

export function modelInGroup(group: ModelGroup, current?: MediaModel) {
  const available = group.variants.filter((m) => m.enabled);
  return (
    available.find((m) => m.id === current?.id) ??
    available.find(
      (m) => current && generationMethod(m) === generationMethod(current),
    ) ??
    available[0] ??
    group.variants[0]
  );
}
