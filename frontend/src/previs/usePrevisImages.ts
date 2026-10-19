import { useRef, useState } from "react";
import { importBrowserFile } from "../assistant/fileImports";
import { MAX_ATTACHMENTS } from "../assistant/attachments";
import type { Asset, Project } from "../model";
import { t } from "../i18n";

export function usePrevisImages(
  project: Project,
  change: (fn: (p: Project) => Project) => void,
) {
  const [items, setItems] = useState<Asset[]>([]);
  const current = useRef(items);
  const [busy, setBusy] = useState(false);
  const importing = useRef(false);
  const [notice, setNotice] = useState("");
  function update(next: Asset[]) {
    current.current = next;
    setItems(next);
  }
  function add(asset: Asset) {
    if (current.current.some((a) => a.id === asset.id)) return;
    if (current.current.length >= MAX_ATTACHMENTS) {
      setNotice(t("本次最多引用 12 张图片"));
      return;
    }
    update([...current.current, asset]);
    setNotice("");
  }
  async function importFiles(files: File[]) {
    if (importing.current || !files.length) return;
    importing.current = true;
    setBusy(true);
    const errors: string[] = [];
    try {
      for (const file of files) {
        if (
          !/\.(png|jpe?g|webp|gif)$/i.test(file.name) &&
          !["image/png", "image/jpeg", "image/webp", "image/gif"].includes(
            file.type,
          )
        ) {
          errors.push(t("请添加 PNG、JPEG、WebP 或 GIF 图片"));
          continue;
        }
        if (current.current.length >= MAX_ATTACHMENTS) {
          errors.push(t("本次最多引用 12 张图片"));
          break;
        }
        try {
          setNotice(t("正在导入附件…"));
          const asset = await importBrowserFile(project.id, file, () => {});
          change((p) => ({
            ...p,
            assets: p.assets.some((a) => a.id === asset.id)
              ? p.assets
              : [...p.assets, asset],
          }));
          add(asset);
        } catch (e) {
          errors.push(String(e));
        }
      }
    } finally {
      importing.current = false;
      setBusy(false);
      setNotice(errors.join("\n"));
    }
  }
  return {
    items,
    busy,
    notice,
    add,
    importFiles,
    remove: (id: string) => update(current.current.filter((a) => a.id !== id)),
    restore: (sent: Asset[]) =>
      update(
        [
          ...new Map(
            [...sent, ...current.current].map((a) => [a.id, a]),
          ).values(),
        ].slice(0, MAX_ATTACHMENTS),
      ),
    clear: () => {
      update([]);
      setNotice("");
    },
  };
}
export type PrevisImages = ReturnType<typeof usePrevisImages>;
