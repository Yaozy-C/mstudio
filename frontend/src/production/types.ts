import type { BoardNode, Project, Reference } from "../model";
export type Frame = {
  assetId: string;
  title: string;
  prompt?: string;
};
export type InputRole =
  | "edit"
  | "reference"
  | "first-frame"
  | "last-frame"
  | "video-reference"
  | "video-edit"
  | "script";
export type ProductionInput = Reference & {
  key: string;
  nodeId?: string;
  sourceRef?: import("../assistant/attachments").AttachmentRef;
  frame?: boolean;
  role: InputRole;
};
export type ProductionTask = {
  key: string;
  ownerId?: string;
  position?: { x: number; y: number };
  kind: "image" | "video";
  mode: "single" | "ends" | "multi" | "mixed";
  inputs: ProductionInput[];
  prompt: string;
  nextPrompt?: string;
  sourceTaskKey?: string;
  hiddenFromList?: boolean;
  modelId: string;
  parameters?: import("./parameters").GenerationParameters;
  jobId?: string;
  requestId?: string;
  status?: string;
  error?: string;
  trackingPaused?: boolean;
  progress?: { stage: string; message: string; updatedAt?: number };
  submissionId?: string;
  turnId?: string;
  createdAt?: number;
  instruction?: string;
  resultAssetId?: string;
  resultAssetIds?: string[];
};
export type GenerationPreferences = {
  image?: string;
  video?: string;
  execution?: "confirm" | "automatic";
};
export type ProductionState = {
  models?: GenerationPreferences;
  positions?: Record<string, { x: number; y: number }>;
  drafts?: Record<string, ProductionTask>;
  hidden?: string[];
  viewport?: Project["viewport"];
};
export type ProductionItem = {
  usages?: { shotId: string; title: string }[];
  key: string;
  kind: "script" | "reference" | "image" | "video" | "note";
  title: string;
  text: string;
  nodeId?: string;
  ownerId?: string;
  assetId?: string;
  x: number;
  y: number;
  width: number;
  height: number;
};
export type CanvasGeneration = {
  task: ProductionTask;
  x: number;
  y: number;
};
export type ProductionSource = BoardNode & {
  canvasGeneration?: CanvasGeneration;
};
export type ChangeProject = (
  fn: (p: Project) => Project,
  record?: boolean,
) => void;
export const itemKey = (...parts: string[]) =>
  parts.map(encodeURIComponent).join(":");
