import { newProject, type Asset } from "../model";
import type { MediaModel } from "../models/mediaRegistry";
export const asset = (id: string, kind: Asset["kind"] = "image"): Asset => ({
  id,
  kind,
  name: id,
  path: `/${id}`,
  preview: `/${id}.png`,
  duration: kind === "video" ? 20 : 0,
  width: 1080,
  height: 1920,
  hasAudio: false,
});
export function fixture() {
  const p = newProject("canvas");
  p.assets = [asset("a"), asset("b"), asset("reference"), asset("v", "video")];
  p.nodes = [
    {
      id: "screenplay",
      kind: "screenplay",
      title: "Lunch",
      text: "Idea",
      x: 0,
      y: 0,
      screenplay: {
        script: [
          {
            id: "paragraph",
            title: "Pack",
            action: "Open bag",
            dialogue: "Hi",
            sound: "Zip",
            duration: 5,
          },
        ],
      },
    },
    {
      id: "shot",
      kind: "shot",
      title: "Pack",
      text: "Open bag",
      x: 0,
      y: 0,
      references: [{ assetId: "reference", purpose: "product" }],
      shot: {
        screenplayId: "screenplay",
        scriptId: "paragraph",
        order: 1,
        duration: 5,
        dialogue: "Hi",
        frames: [
          { assetId: "a", title: "First frame", prompt: "first" },
          { assetId: "b", title: "Last frame", prompt: "last" },
        ],
        takes: [{ assetId: "v", trimIn: 0, trimOut: 5 }],
      },
    },
  ];
  return p;
}
export const model = (
  endpoint: string,
  kind: "image" | "video" = "video",
): MediaModel => ({
  id: "model",
  name: "Model",
  kind,
  plugin: "fal",
  endpoint,
  enabled: true,
  params: {},
});
