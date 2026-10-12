import { createContext } from "react";
import type { Asset } from "../../model";
import type { ImageComment } from "./annotations";

export const ImageReviewContext = createContext<{
  projectId: string;
  hasAsset: (id: string) => boolean;
  submit: (asset: Asset, file: File, comments: ImageComment[]) => Promise<void>;
} | null>(null);
