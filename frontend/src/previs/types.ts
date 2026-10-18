import type { Asset, BoardNode, Project } from "../model";
export type PrevisFile = {
  id: string;
  kind: "model" | "shot";
  name: string;
  description: string;
  code: string;
  revision: number;
  assetIds: string[];
  bindings: Record<string, string>;
  duration: number;
  width: number;
  height: number;
  fps: number;
};
export type PrevisExport = {
  id: string;
  fileId: string;
  fileRevision: number;
  modelRevisions: Record<string, number>;
  signature: string;
  asset: Asset;
  savedAssetId: string | null;
};
export type PrevisWorkspace = {
  files: PrevisFile[];
  messages: { fileId: string; role: "user" | "assistant"; text: string }[];
  exports: PrevisExport[];
};
export const emptyWorkspace: PrevisWorkspace = {
  files: [],
  messages: [],
  exports: [],
};
export function newModel(name: string): PrevisFile {
  return {
    id: crypto.randomUUID(),
    kind: "model",
    name,
    description: "",
    code: "",
    revision: 0,
    assetIds: [],
    bindings: {},
    duration: 5,
    width: 720,
    height: 720,
    fps: 24,
  };
}
export function newShot(node: BoardNode, project: Project): PrevisFile {
  const landscape = project.width >= project.height;
  return {
    ...newModel(node.title || "镜头"),
    id: node.id,
    kind: "shot",
    description: node.text,
    duration: Math.max(1 / 24, Math.min(node.shot?.duration || 5, 60)),
    width: landscape ? 1280 : 720,
    height: landscape ? 720 : 1280,
  };
}
export function isStale(video: PrevisExport, files: PrevisFile[]) {
  const current = files.find((f) => f.id === video.fileId);
  return (
    !current ||
    current.revision !== video.fileRevision ||
    Object.entries(video.modelRevisions).some(
      ([id, revision]) => files.find((f) => f.id === id)?.revision !== revision,
    )
  );
}
export function fileIdentity(file: PrevisFile, files: PrevisFile[]) {
  return JSON.stringify([
    file.id,
    file.revision,
    ...file.assetIds.map((id) => [
      id,
      files.find((f) => f.id === id)?.revision,
    ]),
  ]);
}
