import type { VideoPlan, PlannedShot } from "./creative/types";
export type Asset = {
  generated?: boolean;
  missing?: boolean;
  inLibrary?: boolean;
  id: string;
  name: string;
  kind: "video" | "image" | "audio" | "text" | "document";
  path: string;
  preview: string;
  duration: number;
  width: number;
  height: number;
  hasAudio: boolean;
};
export type Visual = {
  brightness: number;
  contrast: number;
  saturation: number;
  temperature: number;
  effect: "none" | "grayscale" | "sepia" | "blur" | "vignette";
};
export type Clip = {
  transition?: import("./timeline/transitions").Transition;
  visual?: Visual;
  id: string;
  assetId: string;
  trimIn: number;
  trimOut: number;
  speed: number;
  volume: number;
  start: number;
  trackId: string;
  x?: number;
  y?: number;
  scale?: number;
  opacity?: number;
  fadeIn?: number;
  fadeOut?: number;
  shotId?: string;
};
export type Track = {
  sourceTrackId?: string;
  id: string;
  name: string;
  kind: "video" | "audio";
  muted?: boolean;
  hidden?: boolean;
};
export type Caption = {
  font?: "sans" | "serif" | "mono";
  color?: string;
  fontSize?: number;
  x?: number;
  y?: number;
  background?: boolean;
  id: string;
  start: number;
  end: number;
  text: string;
  assetId?: string;
};
export type Reference = {
  assetId: string;
  purpose: string;
  start?: number;
  end?: number;
};
export type Creation = {
  intent: string;
  essential: string;
  preserve: string;
  stage: "planning" | "production" | "editing";
};
export type BoardNode = {
  id: string;
  kind: "asset" | "note" | "shot" | "text" | "plan";
  plan?: VideoPlan;
  shot?: PlannedShot;
  assetId?: string;
  x: number;
  y: number;
  title: string;
  text: string;
  references?: Reference[];
  resultAssetId?: string;
  width?: number;
  height?: number;
};
export type Project = {
  production?: import("./production/types").ProductionState;
  revision?: number;
  id: string;
  name: string;
  assets: Asset[];
  removedAssetIds?: string[];
  nodes: BoardNode[];
  clips: Clip[];
  tracks: Track[];
  captions: Caption[];
  creation?: Creation;
  width: number;
  height: number;
  fps: number;
  viewport: { x: number; y: number; scale: number };
  brief: string;
  updated: number;
};
export type ProjectEntry = {
  id: string;
  name: string;
  updated: number;
  document: Project;
};
export const uid = () => crypto.randomUUID();
export const duration = (c: Clip) => (c.trimOut - c.trimIn) / c.speed;
export const formatTime = (s: number) =>
  `${Math.floor(s / 60)
    .toString()
    .padStart(2, "0")}:${(s % 60).toFixed(1).padStart(4, "0")}`;
export function newProject(name: string): Project {
  return {
    id: uid(),
    name,
    assets: [],
    nodes: [],
    clips: [],
    tracks: [
      { id: "v1", name: "主画面", kind: "video" },
      { id: "a1", name: "音轨 1", kind: "audio" },
    ],
    captions: [],
    width: 1080,
    height: 1920,
    fps: 30,
    viewport: { x: 50, y: 55, scale: 1 },
    brief: "",
    updated: Date.now(),
  };
}
export function makeClip(asset: Asset): Clip {
  return {
    id: uid(),
    assetId: asset.id,
    trimIn: 0,
    trimOut: asset.kind === "image" ? 3 : asset.duration,
    speed: 1,
    volume: 1,
    start: 0,
    trackId: asset.kind === "audio" ? "a1" : "v1",
  };
}
export function splitClip(
  clips: Clip[],
  id: string,
  position: number,
  fps = 30,
): Clip[] {
  const index = clips.findIndex((c) => c.id === id);
  if (index < 0) return clips;
  const clip = clips[index];
  const before = clip.start;
  const cut = clip.trimIn + (position - before) * clip.speed;
  if (
    cut <= clip.trimIn + clip.speed / fps / 2 ||
    cut >= clip.trimOut - clip.speed / fps / 2
  )
    return clips;
  return [
    ...clips.slice(0, index),
    { ...clip, trimOut: cut },
    {
      ...clip,
      id: uid(),
      trimIn: cut,
      start: position,
    },
    ...clips.slice(index + 1),
  ];
}
