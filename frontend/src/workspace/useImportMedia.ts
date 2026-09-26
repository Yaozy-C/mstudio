import { errorText } from "../errors/catalog";
import { useState } from "react";
import { bridge } from "../bridge";
import type { ImportedFiles } from "../assistant/fileImports";
import type { Project } from "../model";
import { canvasAsset } from "./canvasAsset";
export function useImportMedia(
  projectId: string,
  change: (fn: (p: Project) => Project) => void,
  setError: (s: string) => void,
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
        assets: [...p.assets, ...assets],
        nodes: [
          ...p.nodes,
          ...assets.map((a, i) => canvasAsset(a, p.nodes.length + i)),
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
