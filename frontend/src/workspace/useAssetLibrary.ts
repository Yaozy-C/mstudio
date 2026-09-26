import { errorText } from "../errors/catalog";
import { useEffect, useRef, useState } from "react";
import { bridge } from "../bridge";
import type { Asset, Project } from "../model";
import type { ImportedFiles } from "../assistant/fileImports";
import { collectAsset } from "./assetLibrary";

export function useAssetLibrary(
  projectId: string,
  change: (fn: (p: Project) => Project) => void,
) {
  const [assets, setAssets] = useState<Asset[]>([]);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const pending = useRef(false);
  const lastAction = useRef<(() => Promise<void>) | null>(null);
  const [retryLabel, setRetryLabel] = useState("刷新素材列表");
  async function refresh() {
    // Once the mutation succeeded, a refresh failure must not replay it.
    lastAction.current = refresh;
    setRetryLabel("刷新素材列表");
    setAssets(await bridge<Asset[]>("list_global_assets"));
  }
  useEffect(() => {
    let active = true;
    bridge<Asset[]>("list_global_assets")
      .then(
        (items) => {
          if (active) setAssets(items);
        },
        (e) => {
          if (active) setError(String(e));
        },
      )
      .finally(() => {
        if (active) setLoading(false);
      });
    const check = () => {
      bridge<Asset[]>("list_global_assets")
        .then((items) => {
          if (active) setAssets(items);
        })
        .catch(() => {});
    };
    const timer = window.setInterval(check, 5000);
    window.addEventListener("focus", check);
    return () => {
      active = false;
      clearInterval(timer);
      window.removeEventListener("focus", check);
    };
  }, []);
  async function run(action: () => Promise<void>, label = "重试此操作") {
    if (pending.current) return false;
    pending.current = true;
    lastAction.current = action;
    setRetryLabel(label);
    setBusy(true);
    setError("");
    setMessage("");
    try {
      await action();
      return true;
    } catch (e) {
      setError(String(e));
      return false;
    } finally {
      pending.current = false;
      setBusy(false);
    }
  }
  async function upload() {
    const result = await bridge<ImportedFiles>("import_global_media");
    await refresh();
    if (result.errors.length) {
      lastAction.current = upload;
      setRetryLabel("重新选择失败文件");
      setError(errorText(result.errors.join("\n"), "ASSET_IMPORT_FAILED"));
    }
    if (result.assets.length)
      setMessage(`已上传 ${result.assets.length} 个全局素材`);
  }
  return {
    assets,
    busy,
    loading,
    error,
    message,
    retryLabel,
    retry: () => run(lastAction.current ?? refresh, retryLabel),
    upload: () => run(upload, "重新选择文件"),
    promote: (asset: Asset) =>
      run(async () => {
        await bridge("add_global_asset", { id: asset.id });
        await refresh();
        setMessage("已添加到全局素材库");
      }),
    use: (asset: Asset) =>
      run(async () => {
        const saved = await bridge<Asset>("use_global_asset", {
          projectId,
          id: asset.id,
        });
        change((p) => collectAsset(p, saved));
        setMessage("已加入项目素材");
      }),
    remove: (asset: Asset) =>
      run(async () => {
        await bridge("remove_global_asset", { id: asset.id });
        await refresh();
        setMessage("已从全局素材库移除");
      }),
  };
}
