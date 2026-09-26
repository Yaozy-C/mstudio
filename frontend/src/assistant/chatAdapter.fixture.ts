import { expect } from "bun:test";
import { LocalRuntimeCore } from "@assistant-ui/core/internal";
import { defaultAgents } from "../agents/catalog";
import { newProject } from "../model";
import { newConnection } from "../models/types";
import { PlaybackClock } from "../timeline/clock";
import type { ProductionController } from "../production/useProduction";
import { createChatAdapter, type ChatContext } from "./chatAdapter";
import {
  attachmentKey,
  type Attachment,
  type AttachmentRef,
} from "./attachments";
import type { AttachmentDraft } from "./useAttachments";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
export async function until(check: () => boolean) {
  for (let i = 0; i < 100 && !check(); i++) await Bun.sleep(1);
  expect(check()).toBe(true);
}
export function fixture(
  canvas = false,
  initialAgent: string | null = "concept",
) {
  const project = newProject("Send lifecycle");
  project.assets = ["a", "b"].map((id) => ({
    id,
    name: id,
    kind: "image",
    path: `/${id}.png`,
    preview: "",
    duration: 0,
    width: 10,
    height: 10,
    hasAudio: false,
  }));
  const a: AttachmentRef = { kind: "asset", id: "a" };
  const b: AttachmentRef = { kind: "asset", id: "b" };
  const draft = {
    items: [a],
    targetNodeId: "shot-a",
    consume(refs: AttachmentRef[]) {
      this.items = this.items.filter(
        (r) => !refs.some((s) => attachmentKey(s) === attachmentKey(r)),
      );
    },
    consumeTarget(id: string | null) {
      if (this.targetNodeId === id) this.targetNodeId = null;
    },
  } as AttachmentDraft;
  const flush = deferred<void>();
  const context: ChatContext = {
    project,
    draft,
    nodeId: "shot-a",
    clipId: "clip-a",
    clock: new PlaybackClock(),
    flush: () => flush.promise,
  };
  if (canvas)
    context.canvas = {
      task: {
        key: "selected-a",
        ownerId: "shot-a",
        kind: "image",
        mode: "multi",
        inputs: [{ key: "a", assetId: "a", purpose: "修改对象", role: "edit" }],
        prompt: "原图",
        modelId: "image-a",
      },
      composerMode: "agent",
      modelPreferences: { image: "image-a" },
      update(patch) {
        this.task = { ...this.task!, ...patch };
      },
    } as ProductionController;
  const model = { ...newConnection(), endpoint: "http://localhost:1" };
  const sent: Record<string, Attachment[]> = {};
  const calls: {
    args: Record<string, unknown>;
    result: ReturnType<typeof deferred<string>>;
  }[] = [];
  let agent: string | null = initialAgent;
  let savedAgents = structuredClone(defaultAgents);
  let catalogReads = 0;
  const adapter = createChatAdapter({
    context: () => context,
    routing: () => ({
      agents: defaultAgents,
      catalog: { profiles: [model], defaultId: model.id, selectedId: null },
    }),
    agent: () => agent,
    accepted: (id, refs) => {
      sent[id] = refs;
      agent = null;
    },
    target: () => {},
    running: () => {},
    error: () => {},
    invoke: async <T>(
      command: string,
      args?: Record<string, unknown>,
    ): Promise<T> => {
      if (command === "agent_catalog") {
        catalogReads++;
        return structuredClone(savedAgents) as T;
      }
      if (command === "cancel_assistant") {
        calls.at(-1)?.result.reject(new Error("已停止"));
        return undefined as T;
      }
      const result = deferred<string>();
      calls.push({ args: args!.request as Record<string, unknown>, result });
      return (await result.promise) as T;
    },
  });
  const core = new LocalRuntimeCore(
    { adapters: { chatModel: adapter } },
    undefined,
  );
  const thread = core.threads.getMainThreadRuntimeCore();
  return {
    a,
    b,
    context,
    draft,
    flush,
    sent,
    calls,
    thread,
    get catalogReads() {
      return catalogReads;
    },
    setAgents(agents: typeof defaultAgents) {
      savedAgents = structuredClone(agents);
    },
  };
}
