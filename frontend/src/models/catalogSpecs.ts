import type { CapabilityDeclaration } from "./capabilities";
import type { ModelConnection } from "./types";
import spec0 from "./config/catalog/codex-image.json";
import spec1 from "./config/catalog/gemini-image-direct.json";
import spec2 from "./config/catalog/deepseek-flash.json";
import spec3 from "./config/catalog/openai-text.json";
import spec4 from "./config/catalog/gemini-text.json";
import spec5 from "./config/catalog/claude-native.json";
import spec6 from "./config/catalog/minimax-m3.json";
import spec7 from "./config/catalog/gpt-image-2.5-sunburst.json";
import spec8 from "./config/catalog/gpt-image-2.5-flare.json";
import spec9 from "./config/catalog/nano-banana-2.json";
import spec10 from "./config/catalog/midjourney.json";
import spec11 from "./config/catalog/h3-max.json";
import spec12 from "./config/catalog/h3.json";
import spec13 from "./config/catalog/h3-image-to-video.json";
import spec14 from "./config/catalog/h3-reference-to-video.json";
import spec15 from "./config/catalog/seedance-2.5.json";
import spec16 from "./config/catalog/omni.json";
import spec17 from "./config/catalog/veo.json";
import added0 from "./config/catalog/mimo-v2.6-pro.json";
import added1 from "./config/catalog/mimo-v2.6-flash.json";
import added2 from "./config/catalog/mimo-v2.6-pro-ultraspeed.json";
import added3 from "./config/catalog/wan-3.0-text-to-video.json";
import added4 from "./config/catalog/wan-3.0-image-to-video.json";
import added5 from "./config/catalog/wan-3.0-reference-to-video.json";
export type OutputKind = "text" | "image" | "video";
export type ModelSpec = {
  id: string;
  name: string;
  provider: string;
  kind: OutputKind;
  inputs: string[];
  outputs: string[];
  note: string;
  source: string;
  protocol: string;
  request: Record<string, unknown>;
  response: string;
  connection?: Pick<
    ModelConnection,
    "endpoint" | "adapter" | "model" | "inputs" | "contextWindow"
  >;
  endpoint?: string;
  plugin?: string;
  serviceEndpoint?: string;
  /**
   * What the model accepts and exposes, declared once and carried into every model added
   * from this library. See models/capabilities.ts.
   */
  capabilities?: CapabilityDeclaration;
  /** Editing endpoints usually accept the source image; most generation endpoints do not. */
  editCapabilities?: CapabilityDeclaration;
};

/** A declaration already published by the built-in library, for preset shortcuts. */
export function libraryDeclaration(
  plugin: string,
  endpoint: string,
): CapabilityDeclaration | undefined {
  const belongs = (spec: ModelSpec) =>
    (spec.plugin ?? spec.connection?.adapter ?? "fal") === plugin;
  const published = modelLibrary.find(
    (spec) => spec.endpoint === endpoint && belongs(spec),
  );
  if (published) return published.capabilities;
  // Editing variants derive their endpoint exactly like catalogMediaModel does.
  const editing = modelLibrary.find(
    (spec) =>
      spec.endpoint !== undefined &&
      editingEndpoint(spec.endpoint) === endpoint &&
      belongs(spec),
  );
  return editing?.editCapabilities;
}

/** The library publishes one entry per family; editing endpoints are derived at add time. */
export function editingEndpoint(endpoint: string): string {
  return endpoint.replace(/\/text-to-image$/, "") + "/edit";
}
export const modelLibrary = [
  added0,
  added1,
  added2,
  added3,
  added4,
  added5,

  spec0,
  spec1,
  spec2,
  spec3,
  spec4,
  spec5,
  spec6,
  spec7,
  spec8,
  spec9,
  spec10,
  spec11,
  spec12,
  spec13,
  spec14,
  spec15,
  spec16,
  spec17,
] as ModelSpec[];
