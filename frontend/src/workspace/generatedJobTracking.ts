import type { ProductionSource, ProductionTask } from "../production/types";
export type GeneratedJob = {
  id: string;
  backgroundManaged?: boolean;
  sync?: {
    stage: string;
    attempt?: number;
    nextAt?: number;
    error?: string;
    paused?: boolean;
  };
  reconciliation?: { error: string };
  status: string;
  requestId?: string;
  shot: ProductionSource;
  error?: string;
  progress?: ProductionTask["progress"];
  asset?: { id: string };
  outputCount?: number;
  assets?: ({ id: string } | null)[];
};
export function terminalOutputCount(job: GeneratedJob): number {
  if (!["COMPLETED", "FAILED", "CANCELLED"].includes(job.status)) return 0;
  if (job.outputCount === undefined && job.status !== "COMPLETED") return 0;
  if (
    !Number.isSafeInteger(job.outputCount) ||
    job.outputCount! < (job.status === "COMPLETED" ? 1 : 0)
  )
    throw new Error("生成任务缺少有效的实际输出数量");
  return job.outputCount!;
}
