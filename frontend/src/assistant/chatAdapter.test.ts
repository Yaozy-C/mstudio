import { historyMessages } from "./historyMessages";
import { expect, test } from "bun:test";
import { ExportedMessageRepository } from "@assistant-ui/core/internal";
import { fixture, until } from "./chatAdapter.fixture";

test("send clears text and refs before saving or replying; completion keeps the next draft", async () => {
  const f = fixture();
  f.thread.composer.setText("根据图片写脚本");
  const sending = f.thread.composer.send();
  await until(() => Object.keys(f.sent).length === 1);
  expect(f.thread.composer.text).toBe("");
  expect(f.draft.items).toEqual([]);
  expect(f.draft.targetNodeId).toBeNull();
  expect(Object.values(f.sent)[0].map((r) => r.id)).toEqual(["a"]);
  expect(f.calls).toHaveLength(0);
  f.thread.composer.setText("下一条草稿");
  // Re-attaching the same asset belongs to the next draft.
  f.draft.items = [f.a, f.b];
  f.draft.targetNodeId = "shot-b";
  f.flush.resolve();
  await until(() => f.calls.length === 1);
  expect(f.calls[0].args.attachments).toEqual([f.a]);
  expect(f.calls[0].args.agentId).toBe("concept");
  f.calls[0].result.resolve("完成");
  await sending;
  await until(() => f.thread.messages.at(-1)?.status?.type !== "running");
  expect(f.thread.composer.text).toBe("下一条草稿");
  expect(f.draft.items).toEqual([f.a, f.b]);
  expect(f.draft.targetNodeId).toBe("shot-b");
});
test("failed canvas request retries original context without consuming the next selection", async () => {
  const f = fixture(true);
  const original = f.context.canvas!.task!;
  f.thread.composer.setText("修改这张图片");
  const sending = f.thread.composer.send();
  await until(() => Object.keys(f.sent).length === 1);
  expect(f.context.canvas!.task!.inputs).toEqual([]);
  f.context.canvas!.task = {
    ...original,
    key: "selected-b",
    ownerId: "shot-b",
    inputs: [{ key: "b", assetId: "b", purpose: "参考图", role: "reference" }],
  };
  f.context.canvas!.modelPreferences = { image: "image-b" };
  f.draft.items = [f.a, f.b];
  f.thread.composer.setText("下一条草稿");
  f.flush.resolve();
  await until(() => f.calls.length === 1);
  const first = f.calls[0];
  first.result.reject(new Error("模拟网络失败"));
  await sending;
  await until(() => f.thread.messages.at(-1)?.status?.type !== "running");
  const userId = f.thread.messages.find((m) => m.role === "user")!.id;
  const retry = f.thread.startRun({
    parentId: userId,
    sourceId: null,
    runConfig: {},
  });
  await until(() => f.calls.length === 2);
  const second = f.calls[1];
  const {
    clientTurnId: _firstId,
    resumeTurnId: _firstResume,
    ...firstRequest
  } = first.args;
  const {
    clientTurnId: _secondId,
    resumeTurnId: secondResume,
    ...secondRequest
  } = second.args;
  expect(secondResume).toBe(_firstId);
  expect(secondRequest).toEqual(firstRequest);
  expect(f.context.canvas!.task!.key).toBe("selected-b");
  expect(f.draft.items).toEqual([f.a, f.b]);
  expect(f.thread.composer.text).toBe("下一条草稿");
  second.result.resolve("重试完成");
  await retry;
  expect(f.draft.items).toEqual([f.a, f.b]);
  expect(f.thread.composer.text).toBe("下一条草稿");
});
test("stopping keeps sent references on their message and leaves the new draft alone", async () => {
  const f = fixture();
  f.flush.resolve();
  f.thread.composer.setText("处理图片");
  const sending = f.thread.composer.send();
  await until(() => f.calls.length === 1);
  f.draft.items = [f.b];
  f.thread.composer.setText("下一条");
  f.thread.cancelRun();
  await sending;
  await until(() => f.thread.messages.at(-1)?.status?.type !== "running");
  expect(Object.values(f.sent)[0].map((r) => r.id)).toEqual(["a"]);
  expect(f.draft.items).toEqual([f.b]);
  expect(f.thread.composer.text).toBe("下一条");
});
test("reopened failed history retries its saved request and preserves the current composer", async () => {
  const f = fixture(true);
  const saved = {
    refs: [{ ...f.a, title: "原图片", assetId: "a", mediaKind: "image" }],
    agentId: "concept",
    targetNodeId: "original-shot",
    selectedNodeId: "original-shot",
    selection: { clipId: "original-clip", time: 3 },
    production: {
      projectId: f.context.project.id,
      models: { image: "original-image-model" },
      instruction: "原请求",
    },
  };
  const restored = historyMessages([
    {
      id: 1,
      role: "user",
      content: "原请求",
      payload: [{ type: "text", text: "原请求", attachments: saved.refs }],
      attribution: { turnId: "past", agentId: "concept", request: saved },
    },
    {
      id: 2,
      role: "assistant",
      content: "",
      payload: "",
      attribution: { turnId: "past", status: "failed", error: "模型回答超时" },
    },
  ]);
  f.thread.import(ExportedMessageRepository.fromArray(restored));
  expect(f.thread.messages).toHaveLength(2);
  expect(f.thread.messages[1].status).toEqual({
    type: "incomplete",
    reason: "error",
    error: "模型回答超时",
  });
  expect(f.thread.messages[0].metadata.custom.attachments).toEqual(saved.refs);
  f.thread.composer.setText("新的草稿");
  f.draft.items = [f.b];
  f.flush.resolve();
  const retry = f.thread.startRun({
    parentId: "history-1",
    sourceId: null,
    runConfig: {},
  });
  await until(() => f.calls.length === 1);
  expect(f.calls[0].args.messageContext).toEqual(saved);
  expect(f.calls[0].args.attachments).toEqual([f.a]);
  expect(f.calls[0].args.agentId).toBe("concept");
  expect(f.calls[0].args.selectedNodeId).toBe("original-shot");
  expect(f.context.canvas!.task!.key).toBe("selected-a");
  expect(f.draft.items).toEqual([f.b]);
  expect(f.thread.composer.text).toBe("新的草稿");
  f.calls[0].result.resolve("已完成");
  await retry;
  expect(f.thread.composer.text).toBe("新的草稿");
});
test("unselected agent uses coordinator and dismissed context stays absent", async () => {
  const f = fixture(false, null);
  f.context.work = {
    view: "script",
    screenplayId: "old-screenplay",
    paragraphId: "p1",
  };
  f.draft.targetNodeId = null;
  f.draft.omitWork = true;
  f.thread.composer.setText("聊聊新的想法");
  const sending = f.thread.composer.send();
  f.flush.resolve();
  await until(() => f.calls.length === 1);
  expect(f.calls[0].args.agentId).toBe("coordinator");
  expect(f.calls[0].args.taskNodeId).toBeNull();
  expect(f.calls[0].args.selectedNodeId).toBeNull();
  expect((f.calls[0].args.messageContext as { work: unknown }).work).toEqual({
    view: "script",
  });
  f.calls[0].result.resolve("好的");
  await sending;
});

test("media modes cannot invoke the Agent or consume its draft/context", async () => {
  for (const mode of ["image", "video", "reference"] as const) {
    const f = fixture(true);
    f.context.canvas!.composerMode = mode;
    f.thread.composer.setText("直接生成");
    f.flush.resolve();
    await f.thread.composer.send();
    expect(f.calls).toHaveLength(0);
    expect(f.sent).toEqual({});
    expect(f.draft.items).toEqual([f.a]);
  }
});

test("explicit new task survives retry without following the next draft", async () => {
  const f = fixture();
  f.context.newTask = true;
  f.thread.composer.setText("新的剪辑任务");
  const sending = f.thread.composer.send();
  await until(() => Object.keys(f.sent).length === 1);
  f.context.newTask = false;
  f.flush.resolve();
  await until(() => f.calls.length === 1);
  expect(f.calls[0].args.newTask).toBe(true);
  f.calls[0].result.reject(new Error("模拟取消"));
  await sending;
  await until(() => f.thread.messages.at(-1)?.status?.type !== "running");
  const userId = f.thread.messages.find((m) => m.role === "user")!.id;
  const retry = f.thread.startRun({
    parentId: userId,
    sourceId: null,
    runConfig: {},
  });
  await until(() => f.calls.length === 2);
  expect(f.calls[1].args.newTask).toBe(true);
  expect(f.calls[1].args.resumeTurnId).toBe(f.calls[0].args.clientTurnId);
  f.calls[1].result.resolve("完成");
  await retry;
});
