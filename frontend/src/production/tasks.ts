import type { Project } from "../model";
import {
  type ProductionInput,
  type ProductionItem,
  type ProductionTask,
} from "./types";
export function taskKey(ids: string[]) {
  return JSON.stringify([...ids].sort());
}
export function createTask(
  p: Project,
  selected: ProductionItem[],
  kind?: ProductionTask["kind"],
): ProductionTask {
  const video = selected.find((n) => n.kind === "video" && n.nodeId);
  const image = selected.find(
    (n) => p.assets.find((a) => a.id === n.assetId)?.kind === "image",
  );
  const output = kind ?? (video ? "video" : "image");
  const wanted = new Map(selected.map((n) => [n.key, n]));
  const unique = [...wanted.values()].filter(
    (n, i, all) =>
      !n.assetId || all.findIndex((v) => v.assetId === n.assetId) === i,
  );
  const visual = unique.filter((n) => n.assetId);
  const mode = visual.some((n) => n.kind === "video")
    ? "mixed"
    : visual.length > 1
      ? "multi"
      : "single";
  const inputs: ProductionInput[] = unique.map((n) => ({
    key: n.key,
    assetId: n.assetId ?? "",
    nodeId: n.nodeId,
    frame: n.kind === "image" && !!p.nodes.find((v) => v.id === n.nodeId)?.shot,
    purpose: n.kind === "script" || n.kind === "note" ? n.text : "保持内容一致",
    role: !n.assetId
      ? "script"
      : n.kind === "video"
        ? "video-reference"
        : output === "image" && n.key === image?.key
          ? "edit"
          : output === "video" && mode === "single"
            ? "first-frame"
            : "reference",
    ...(n.kind === "video"
      ? {
          start: 0,
          end: Math.min(
            5,
            p.assets.find((a) => a.id === n.assetId)?.duration ?? 0,
          ),
        }
      : {}),
  }));
  return {
    key: taskKey(selected.map((n) => n.key)),

    kind: output,
    mode,
    inputs,
    prompt: "",
    modelId: "",
  };
}
export function withMode(task: ProductionTask, mode: ProductionTask["mode"]) {
  return { ...task, mode };
}
