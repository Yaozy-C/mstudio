import type { ModelSpec } from "./catalogSpecs";
import type { MediaKind, MediaModel } from "./mediaRegistry";
export function catalogMediaModel(
  spec: ModelSpec,
  kind: MediaKind,
  editing = false,
): MediaModel {
  const plugin =
    spec.plugin ??
    (spec.id === "codex-image"
      ? "codex-image"
      : spec.id === "gemini-image-direct"
        ? "gemini-native"
        : "fal");
  const falEdit = editing && plugin === "fal";
  const { prompt: _prompt, ...params } = spec.request;
  return {
    id: crypto.randomUUID(),
    name: spec.name + (editing ? " · 编辑" : ""),
    kind,
    plugin,
    endpoint: falEdit
      ? spec.endpoint!.replace(/\/text-to-image$/, "") + "/edit"
      : spec.endpoint!,
    params: falEdit ? { image_urls: [] } : params,
    // The editing endpoint usually accepts the source image where generation does not.
    capabilities:
      (editing ? spec.editCapabilities : undefined) ?? spec.capabilities,
    enabled: true,
  };
}
