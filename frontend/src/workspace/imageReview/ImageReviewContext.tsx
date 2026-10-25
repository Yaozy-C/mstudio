import { createContext } from "react";
import type { Asset } from "../../model";
import type { ColorCorrection } from "./processingOptions";
import type { ImageComment } from "./annotations";

export type ImageOperation =
  | {
      kind: "restore";
      scale: 1;
      seed: number;
      denoise: number;
      vaeTile: number;
      quantize8?: boolean;
      colorCorrection?: ColorCorrection;
    }
  | { kind: "upscale"; scale: 2 | 4; tileSize: 128 | 256 | 512 }
  | { kind: "lut"; cube: string; strength: number };

export type ImageVersion = {
  id: string;
  image: Asset;
  operation?: ImageOperation | null;
};

export const ImageReviewContext = createContext<{
  projectId: string;
  hasAsset: (id: string) => boolean;
  submit: (asset: Asset, file: File, comments: ImageComment[]) => Promise<void>;
  process: (
    asset: Asset,
    operation: ImageOperation,
    name: string,
    versionId?: string,
  ) => Promise<ImageVersion>;
  versions: (imageId: string) => Promise<ImageVersion[]>;
  selectReference: (assetId: string, versionId: string) => Promise<Asset>;
} | null>(null);
