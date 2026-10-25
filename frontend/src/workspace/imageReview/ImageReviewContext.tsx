import { createContext } from "react";
import type { Asset } from "../../model";
import type { ImageComment } from "./annotations";

export type ImageOperation =
  | { kind: "restore"; scale: 1 | 2 | 4; seed: number; vaeTile: number }
  | { kind: "lut"; cube: string; strength: number };

export const ImageReviewContext = createContext<{
  projectId: string;
  hasAsset: (id: string) => boolean;
  submit: (asset: Asset, file: File, comments: ImageComment[]) => Promise<void>;
  process: (
    asset: Asset,
    operation: ImageOperation,
    name: string,
  ) => Promise<Asset>;
} | null>(null);
