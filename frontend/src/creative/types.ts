export type ScriptParagraph = {
  duration?: number;
  id: string;
  title: string;
  action: string;
  onScreenText?: string;
  dialogue: string;
  sound: string;
};
export type ScriptDocument = {
  script?: ScriptParagraph[];
};
export type ShotTake = {
  production?: import("../production/types").ProductionTask;
  assetId: string;
  trimIn: number;
  trimOut: number;
  basis?: string;
};
export type PlannedShot = {
  frames?: import("../production/types").Frame[];
  screenplayId: string;
  scriptId?: string;
  scriptBasis?: string;
  order: number;
  duration: number;
  dialogue: string;
  prompt?: string;
  promptBasis?: string;
  framePrompt?: string;
  visualChanged?: boolean;
  takes?: ShotTake[];
};
