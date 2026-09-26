import type { Asset, Project } from "../model";
import { receiveProductionResult } from "../production/document";
import type { ProductionSource } from "../production/types";
export function receiveGeneratedResult(
  p: Project,
  asset: Asset,
  source: ProductionSource,
): Project {
  if (!source?.canvasGeneration)
    throw new Error("生成任务缺少制作上下文，未写入项目");
  return receiveProductionResult(p, asset, source);
}
