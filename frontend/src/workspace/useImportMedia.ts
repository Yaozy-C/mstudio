import { errorText } from "../errors/catalog";
import { useState } from "react";
import { bridge } from "../bridge";
import type { ImportedFiles } from "../assistant/fileImports";
import type { Project } from "../model";
export function useImportMedia(
  projectId: string,
  change: (fn: (p: Project) => Project) => void,
  setError: (s: string) => void,
  usage: "material" | "asset" = "material",
) {
  const [busy, setBusy] = useState(false);
  async function importMedia() {
    setBusy(true);
    setError("");
    try {
      const { assets, errors } = await bridge<ImportedFiles>("import_media", {
        projectId,
      });
      if (errors.length)
        setError(errorText(errors.join("\n"), "ASSET_IMPORT_FAILED"));
      change((p) => ({
        ...p,
        assets: [
          ...p.assets,
          ...assets.map((a) => ({
            ...a,
            usage:
              usage === "asset" && ["text", "document"].includes(a.kind)
                ? ("material" as const)
                : usage,
            inMaterials: true,
            assetStatus:
              usage === "asset" && ["image", "video", "audio"].includes(a.kind)
                ? ("confirmed" as const)
                : undefined,
            inLibrary: true,
          })),
        ],
      }));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return { busy, importMedia };
}
