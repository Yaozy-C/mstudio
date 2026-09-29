import { errorText, normalizeError } from "../errors/catalog";
import { useEffect, useRef, useState } from "react";
import { uid, type Project } from "../model";
import { native } from "../bridge";
import { runtime } from "../plugins/runtime";
import type { useGenerationAgent } from "../creation/useGenerationAgent";
import type { UploadedReference } from "../creation/generationInput";
import { resultPlacement } from "./resultPlacement";
import { canvasSnapshot, inputFor, mediaReferences } from "./request";
import type { ProductionTask } from "./types";
export function useProductionSubmit({
  projectId,
  media,
  get,
  flush,
  update,
}: {
  projectId: string;
  media: ReturnType<typeof useGenerationAgent>;
  get: () => Project;
  flush: () => Promise<void>;
  update: (patch: Partial<ProductionTask>, key?: string) => void;
}) {
  const active = useRef(true);
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
    };
  }, []);
  const submittingRef = useRef(new Set<string>());
  const [slotVersion, setSlotVersion] = useState(0);
  const [submitting, setSubmitting] = useState(false);
  async function submit(requested: ProductionTask) {
    if (!active.current || submittingRef.current.has(requested.key)) return;
    const current = get().production?.drafts?.[requested.key] ?? requested;
    if (
      [
        "UPLOADING",
        "SUBMITTING",
        "IN_QUEUE",
        "IN_PROGRESS",
        "RECEIVING",
        "UNKNOWN",
        "CANCEL_REQUESTED",
        "COMPLETED",
        "CANCELLED",
      ].includes(current.status ?? "")
    )
      return;
    if (submittingRef.current.size >= 2) {
      update({ status: "READY", error: undefined }, requested.key);
      return;
    }
    submittingRef.current.add(requested.key);
    const draft = structuredClone(current),
      targetKey = draft.key;
    let started = false;
    try {
      if (!native)
        throw new Error("请在已打开的桌面应用中生成，浏览器可编辑画布");
      const model = media.models.find((m) => m.id === draft.modelId);
      if (!model || !media.agent?.enabled)
        throw new Error("请在设置中连接生成服务，并在这里选择模型");
      setSubmitting(true);
      await flush();
      if (!active.current) return;
      const p = structuredClone(get());
      const { x, y } = draft.position ?? resultPlacement(p, draft);
      const snapshot = canvasSnapshot(p, draft, { x, y });
      inputFor(p, draft, model);
      const refs = mediaReferences(draft);
      update({ status: "UPLOADING", error: undefined }, targetKey);
      const uploaded = refs.length
        ? await runtime.execute<UploadedReference[]>("upload_references", {
            projectId: p.id,
            references: refs,
            mediaModelId: model.id,
            approved: true,
          })
        : [];
      if (!active.current) return;
      if (get().production?.drafts?.[targetKey]?.status === "CANCELLED") return;
      // Check that the selected shot and images still exist after uploading.
      canvasSnapshot(get(), draft, { x, y });
      const input = inputFor(p, draft, model, uploaded);
      const id = uid();
      snapshot.task = {
        ...snapshot.task,
        jobId: id,
        submissionId: id,
        status: "SUBMITTING",
        error: undefined,
      };
      update({ submissionId: id, jobId: id, status: "SUBMITTING" }, targetKey);
      await flush();
      if (!active.current) return;
      started = true;
      const source = p.nodes.find((n) => n.id === draft.ownerId);
      const job = await runtime.execute<{
        id: string;
        status?: string;
        error?: string;
        requestId?: string;
      }>("submit_job", {
        projectId: p.id,
        endpoint: model.endpoint,
        mediaModelId: model.id,
        agentId: media.agent.id,
        agentRevision: media.agent.revision,
        input,
        approved: true,
        shot: {
          ...(source ?? { title: "画布生成" }),
          references: refs,
          canvasGeneration: snapshot,
          submissionId: id,
          projectRevision: p.revision,
        },
      });
      if (!active.current) return;
      update(
        {
          jobId: job.id,
          requestId: job.requestId,
          status: job.status ?? "IN_QUEUE",
          error: job.error,
        },
        targetKey,
      );
      window.dispatchEvent(
        new CustomEvent("studio-job-submitted", {
          detail: { projectId: p.id, id: job.id },
        }),
      );
    } catch (error) {
      if (!active.current) return;
      update(
        {
          status:
            started && normalizeError(error).outcome !== "rejected"
              ? "UNKNOWN"
              : "FAILED",
          error: errorText(
            error,
            started ? "SUBMISSION_UNKNOWN" : "VALIDATION_FAILED",
          ),
        },
        targetKey,
      );
      if (started)
        window.dispatchEvent(
          new CustomEvent("studio-job-submitted", {
            detail: {
              projectId: projectId,
              id: get().production?.drafts?.[targetKey]?.jobId,
            },
          }),
        );
    } finally {
      submittingRef.current.delete(requested.key);
      if (active.current) {
        setSubmitting(submittingRef.current.size > 0);
        setSlotVersion((v) => v + 1);
      }
    }
  }
  return {
    submit,
    submitting,
    slotVersion,
    available: () => 2 - submittingRef.current.size,
  };
}
