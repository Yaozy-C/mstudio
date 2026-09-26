import { invoke } from "@tauri-apps/api/core";
import { bridge, native } from "../bridge";
import type { Asset } from "../model";
export type ImportedFiles = { assets: Asset[]; errors: string[] };
const extensions = new Set(
  "mp4 mov m4v webm mkv png jpg jpeg webp gif heic bmp tiff wav mp3 m4a aac flac ogg txt md csv json pdf".split(
    " ",
  ),
);
const mimeExtensions: Record<string, string> = {
  "image/png": "png",
  "image/jpeg": "jpg",
  "image/webp": "webp",
  "image/gif": "gif",
  "text/plain": "txt",
  "text/markdown": "md",
  "text/csv": "csv",
  "application/json": "json",
  "application/pdf": "pdf",
  "video/mp4": "mp4",
  "video/quicktime": "mov",
  "video/webm": "webm",
  "audio/mpeg": "mp3",
  "audio/wav": "wav",
  "audio/mp4": "m4a",
};
export function importFileName(file: Pick<File, "name" | "type" | "size">) {
  const raw = file.name || "粘贴的文件";
  const ext = raw.split(".").at(-1)?.toLowerCase() ?? "";
  const name = extensions.has(ext)
    ? raw
    : !raw.includes(".") && mimeExtensions[file.type]
      ? `${raw}.${mimeExtensions[file.type]}`
      : raw;
  const suffix = name.split(".").at(-1)!.toLowerCase();
  if (!extensions.has(suffix))
    throw new Error("不支持此格式，请添加图片、视频、音频、PDF 或文本文件");
  const limit = ["txt", "md", "csv", "json"].includes(suffix)
    ? 120_000
    : suffix === "pdf"
      ? 12 * 1024 * 1024
      : 1024 ** 3;
  if (!file.size || file.size > limit)
    throw new Error("文件为空或过大（文本 120 KB，PDF 12 MiB，媒体 1 GiB）");
  return name;
}
export function transferFiles(
  data: Pick<DataTransfer, "files" | "items">,
): File[] {
  const files = Array.from(data.files);
  if (files.length) return files;
  return Array.from(data.items)
    .filter((item) => item.kind === "file")
    .map((item) => item.getAsFile())
    .filter((file): file is File => !!file);
}
export async function importBrowserFile(
  projectId: string,
  file: File,
  progress: (percent: number) => void,
): Promise<Asset> {
  const name = importFileName(file);
  if (!native) throw new Error("请在 Mstudio 桌面应用中粘贴或拖入文件");
  const id = await bridge<string>("begin_import", {
    projectId,
    name,
    size: file.size,
  });
  try {
    const chunkSize = 1024 * 1024;
    for (let offset = 0; offset < file.size; offset += chunkSize) {
      const bytes = await file.slice(offset, offset + chunkSize).arrayBuffer();
      await invoke("append_import", bytes, {
        headers: {
          "x-project-id": projectId,
          "x-import-id": id,
          "x-import-offset": String(offset),
        },
      });
      progress(
        Math.round((Math.min(offset + chunkSize, file.size) / file.size) * 100),
      );
    }
    return await bridge<Asset>("finish_import", { projectId, id });
  } catch (error) {
    await bridge("cancel_import", { projectId, id }).catch(() => {});
    throw error;
  }
}
