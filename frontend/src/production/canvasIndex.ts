import type { Clip } from "../model";

// Membership is a property of the current edit, never a saved approval flag.
export function timelineAssetIds(clips: Clip[]): Set<string> {
  return new Set(clips.map((clip) => clip.assetId));
}
