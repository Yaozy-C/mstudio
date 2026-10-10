import configured from "./config/presets.json";
import type { CapabilityDeclaration } from "./capabilities";
export const presets = configured as Record<
  "image" | "video",
  {
    name: string;
    endpoint: string;
    params: Record<string, unknown>;
    capabilities?: CapabilityDeclaration;
  }
>;
