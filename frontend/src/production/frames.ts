import type { BoardNode } from "../model";
import type { Frame } from "./types";
export function framesOf(n?: BoardNode): Frame[] {
  return n?.shot?.frames ?? [];
}
