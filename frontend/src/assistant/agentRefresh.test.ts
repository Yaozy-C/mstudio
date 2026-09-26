import { expect, test } from "bun:test";
import { defaultAgents } from "../agents/catalog";
import { fixture, until } from "./chatAdapter.fixture";

function saved(revision: number, enabled = true) {
  return defaultAgents.map((a) =>
    a.id === "concept"
      ? {
          ...a,
          revision,
          enabled,
          name: `已更新 ${revision}`,
          instructions: "新的职责",
        }
      : a,
  );
}

test("an open project sends with saved profile despite stale UI catalog", async () => {
  const f = fixture();
  f.setAgents(saved(20));
  f.flush.resolve();
  f.thread.composer.setText("用新的职责工作");
  const sending = f.thread.composer.send();
  await until(() => f.calls.length === 1);
  expect(f.catalogReads).toBe(1);
  expect(f.calls[0].args.agentRevision).toBe(20);
  expect(f.calls[0].args.agentName).toBe("已更新 20");
  f.calls[0].result.resolve("完成");
  await sending;
});

test("retry refreshes the profile while retaining original task and attachments", async () => {
  const f = fixture(true);
  f.flush.resolve();
  f.thread.composer.setText("修改原图");
  const sending = f.thread.composer.send();
  await until(() => f.calls.length === 1);
  f.calls[0].result.reject(new Error("网络失败"));
  await sending;
  f.setAgents(saved(21));
  f.draft.items = [f.b];
  f.thread.composer.setText("下一条草稿");
  const userId = f.thread.messages.find((m) => m.role === "user")!.id;
  const retry = f.thread.startRun({
    parentId: userId,
    sourceId: null,
    runConfig: {},
  });
  await until(() => f.calls.length === 2);
  expect(f.catalogReads).toBe(2);
  expect(f.calls[1].args.agentRevision).toBe(21);
  expect(f.calls[1].args.messageContext).toEqual(
    f.calls[0].args.messageContext,
  );
  expect(f.calls[1].args.attachments).toEqual([f.a]);
  expect(f.draft.items).toEqual([f.b]);
  expect(f.thread.composer.text).toBe("下一条草稿");
  f.calls[1].result.resolve("完成");
  await retry;
});

test("a newly disabled agent is rejected before submitting or consuming references", async () => {
  const f = fixture();
  f.setAgents(saved(22, false));
  f.flush.resolve();
  f.thread.composer.setText("不要提交给停用角色");
  await f.thread.composer.send();
  expect(f.catalogReads).toBe(1);
  expect(f.calls).toHaveLength(0);
  expect(f.sent).toEqual({});
  expect(f.draft.items).toEqual([f.a]);
});
