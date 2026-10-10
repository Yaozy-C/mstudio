import { validateMediaModel } from "./validateMediaModel";
export { validateMediaModel } from "./validateMediaModel";
import { useEffect, useState } from "react";
import { bridge } from "../bridge";
import type { CapabilityDeclaration } from "./capabilities";
export type MediaKind = "image" | "video" | "audio";
export type MediaModel = {
  id: string;
  name: string;
  connectionId?: string | null;
  kind: MediaKind;
  plugin: string;
  endpoint: string;
  params: Record<string, unknown>;
  enabled: boolean;
  http?: Record<string, unknown>;
  /** Model-declared reference inputs and generation controls. See models/capabilities.ts. */
  capabilities?: CapabilityDeclaration;
  hasKey?: boolean;
};
export const mediaLabels = { image: "图像", video: "视频", audio: "音频" };
export function useMediaModels() {
  const [models, setModels] = useState<MediaModel[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    let revision = 0;
    const refresh = () => {
      const current = ++revision;
      setLoading(true);
      setError("");
      void bridge<MediaModel[]>("media_model_catalog")
        .then((next) => {
          if (active && current === revision) {
            setModels(next);
            setError("");
          }
        })
        .catch((e) => {
          if (active && current === revision) setError(String(e));
        })
        .finally(() => {
          if (active && current === revision) setLoading(false);
        });
    };
    refresh();
    window.addEventListener("media-models-changed", refresh);
    return () => {
      active = false;
      window.removeEventListener("media-models-changed", refresh);
    };
  }, []);
  async function save(model: MediaModel, key?: string, clearKey = false) {
    validateMediaModel(model);
    await bridge("save_media_model", { model, key: key || null, clearKey });
    window.dispatchEvent(new Event("media-models-changed"));
  }
  async function remove(id: string) {
    await bridge("remove_media_model", { id });
    window.dispatchEvent(new Event("media-models-changed"));
  }
  return {
    models,
    loading,
    error,
    save,
    remove,
    refresh: () => window.dispatchEvent(new Event("media-models-changed")),
  };
}
