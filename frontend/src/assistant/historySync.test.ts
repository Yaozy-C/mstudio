import { expect, test } from "bun:test";
import type { ThreadMessageLike } from "@assistant-ui/react";
import { historySync } from "./historySync";

const message = (turn: string, text: string): ThreadMessageLike => ({
  role: "assistant",
  content: text,
  metadata: { custom: { turnId: turn } },
});
function fixture(initial = [message("old", "旧记录")]) {
  let state = { isRunning: false, messages: initial };
  const pending: ((messages: ThreadMessageLike[]) => void)[] = [];
  const applied: ThreadMessageLike[][] = [];
  const sync = historySync({
    state: () => state,
    read: () => new Promise((resolve) => pending.push(resolve)),
    apply: (messages) => {
      applied.push(messages);
      state = { ...state, messages };
    },
    error: (error) => {
      throw error;
    },
  });
  return {
    sync,
    pending,
    applied,
    setState: (next: typeof state) => {
      state = next;
    },
  };
}

test("restores newer persisted turns missing from the open chat", async () => {
  const f = fixture();
  const refresh = f.sync.refresh();
  const saved = [message("old", "旧记录"), message("new", "最新记录")];
  f.pending[0]!(saved);
  await refresh;
  expect(f.applied).toEqual([saved]);
});

test("updates a persisted partial answer without adding a duplicate", async () => {
  const f = fixture([message("same", "正在处理")]);
  const refresh = f.sync.refresh();
  f.pending[0]!([message("same", "已完成")]);
  await refresh;
  expect(f.applied[0]).toHaveLength(1);
  expect(f.applied[0]![0]!.content).toBe("已完成");
});

test("does not replace messages when a new run starts during the read", async () => {
  const f = fixture();
  const refresh = f.sync.refresh();
  f.setState({ isRunning: true, messages: [message("live", "正在生成")] });
  f.pending[0]!([message("old", "数据库记录")]);
  await refresh;
  expect(f.applied).toHaveLength(0);
});

test("ignores late reads and reads completed after disposal", async () => {
  const f = fixture();
  const first = f.sync.refresh();
  const second = f.sync.refresh();
  f.pending[0]!([message("old", "过时响应")]);
  await first;
  expect(f.applied).toHaveLength(0);
  f.sync.dispose();
  f.pending[1]!([message("old", "离开项目后的响应")]);
  await second;
  expect(f.applied).toHaveLength(0);
});

test("preserves a local failed send that never reached persistence", async () => {
  const f = fixture([message("unsaved", "待重试")]);
  const refresh = f.sync.refresh();
  f.pending[0]!([message("old", "旧记录")]);
  await refresh;
  expect(f.applied).toHaveLength(0);
});

test("unchanged history with different runtime IDs does not reset the thread", async () => {
  const f = fixture([
    {
      ...message("same", "回答"),
      id: "local",
      status: { type: "complete", reason: "stop" },
    },
  ]);
  const refresh = f.sync.refresh();
  f.pending[0]!([{ ...message("same", "回答"), id: "history-1" }]);
  await refresh;
  expect(f.applied).toHaveLength(0);
});
