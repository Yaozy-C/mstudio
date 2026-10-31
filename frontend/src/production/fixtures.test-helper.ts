import { newProject, type Asset } from "../model";
import { libraryDeclaration } from "../models/catalogSpecs";
import type { CapabilityDeclaration } from "../models/capabilities";
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
/**
 * A model as the library would add it: capabilities come from the published declaration,
 * never from the endpoint string, so tests exercise the same data the app ships.
 */
export const model = (
  endpoint: string,
  kind: "image" | "video" = "video",
  capabilities?: CapabilityDeclaration,
): MediaModel => ({
  id: "model",
  name: "Model",
  kind,
  plugin: "fal",
  endpoint,
  enabled: true,
  params: {},
  capabilities: capabilities ?? libraryDeclaration("fal", endpoint),
});

export function draftFixture() {
  const p = fixture();
  p.nodes.push(
    ...p.assets
      .filter((a) => a.id !== "reference")
      .map((a, i) => ({
        id: `draft-${a.id}`,
        kind: "asset" as const,
        assetId: a.id,
        title: a.name,
        text: "",
        x: i * 300,
        y: 0,
      })),
  );
  return p;
}
export function shotSources(
  p: ReturnType<typeof fixture>,
): import("./types").DraftItem[] {
  return p.nodes
    .filter((n) => n.shot)
    .flatMap((n) => [
      ...(n.shot?.frames ?? []).map((f) => ({
        key: `frame:${n.id}:${f.assetId}`,
        kind: "image" as const,
        nodeId: n.id,
        ownerId: n.id,
        assetId: f.assetId,
        title: f.title,
        text: "",
        x: 0,
        y: 0,
        width: 250,
        height: 300,
      })),
      ...(n.shot?.takes ?? []).map((f) => ({
        key: `take:${n.id}:${f.assetId}`,
        kind: "video" as const,
        nodeId: n.id,
        ownerId: n.id,
        assetId: f.assetId,
        title: f.assetId,
        text: "",
        x: 0,
        y: 0,
        width: 250,
        height: 300,
      })),
    ]);
}
