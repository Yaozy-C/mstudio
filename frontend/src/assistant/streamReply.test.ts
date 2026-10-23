import { expect, test } from "bun:test";
import { streamReply, type AgentProgress } from "./streamReply";
import type { bridge } from "../bridge";

function fixture() {
  let receive!: (event: AgentProgress) => void;
  let resolve!: (reply: unknown) => void;
  let reject!: (error: Error) => void;
  let subscribed = false;
  let closed = false;
  const request = { projectId: "p", clientTurnId: "t" };
  const response = new Promise<unknown>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  const invoke = (async () => {
    expect(subscribed).toBe(true);
    return response;
  }) as typeof bridge;
  const controller = new AbortController();
  const stream = streamReply(
    invoke,
    request,
    controller.signal,
    async (handler) => {
      receive = handler;
      subscribed = true;
      return () => {
        closed = true;
      };
    },
  );
  return {
    stream,
    resolve: (reply: { text: string; status: string }) => resolve(reply),
    malformed: (reply: unknown) => resolve(reply),
    abort: () => controller.abort(),
    wait: () => receive({ projectId: "p", turnId: "t", kind: "turn/waiting" }),
    end: (status = "completed", error?: string) =>
      receive({
        projectId: "p",
        turnId: "t",
        kind: "turn/end",
        payload: { status, error },
      }),
    incompleteEnd: () =>
      receive({ projectId: "p", turnId: "t", kind: "turn/end" }),
    reject: (e: Error) => reject(e),
    closed: () => closed,
    emit: (text: string, turnId = "t") =>
      receive({
        projectId: "p",
        turnId,
        kind: "assistant/partial",
        payload: { text },
      }),
  };
}
test("streams before completion, ignores another turn and closes subscription", async () => {
  const f = fixture();
  const first = f.stream.next();
  await Bun.sleep(0);
  f.emit("foreign", "other");
  f.emit("第一段");
  expect((await first).value).toBe("第一段");
  const last = f.stream.next();
  f.resolve({ text: "第一段和结尾", status: "completed" });
  expect((await last).value).toBe("第一段和结尾");
  expect((await f.stream.next()).done).toBe(true);
  expect(f.closed()).toBe(true);
});
test("failed request retains delivered prefix and always unsubscribes", async () => {
  const f = fixture();
  const first = f.stream.next();
  await Bun.sleep(0);
  f.emit("保留这段");
  expect((await first).value).toBe("保留这段");
  const next = f.stream.next();
  f.reject(new Error("connection lost"));
  await expect(next).rejects.toThrow("connection lost");
  expect(f.closed()).toBe(true);
});

test("a string reply is rejected instead of being accepted as a completed response", async () => {
  const f = fixture();
  const next = f.stream.next();
  await Bun.sleep(0);
  f.malformed("unstructured reply");
  await expect(next).rejects.toThrow("Agent 返回格式无效");
  expect(f.closed()).toBe(true);
});

test("a terminal event without a status cannot finish a waiting response successfully", async () => {
  const f = fixture();
  const first = f.stream.next();
  await Bun.sleep(0);
  f.resolve({ text: "Waiting", status: "waiting" });
  await first;
  const next = f.stream.next();
  f.incompleteEnd();
  await expect(next).rejects.toThrow("Agent 结束事件缺少有效状态");
  expect(f.closed()).toBe(true);
});

test("a failed reply stays failed when its progress event was not received", async () => {
  const f = fixture();
  const first = f.stream.next();
  await Bun.sleep(0);
  f.resolve({ text: "Saved partial answer", status: "failed" });
  expect((await first).value).toBe("Saved partial answer");
  await expect(f.stream.next()).rejects.toThrow("回答未完成");
  expect(f.closed()).toBe(true);
});

test("clears completed progress before streaming the final answer", async () => {
  const f = fixture();
  const first = f.stream.next();
  await Bun.sleep(0);
  f.emit("正在读取参考");
  expect((await first).value).toBe("正在读取参考");
  const reset = f.stream.next();
  f.emit("");
  expect((await reset).value).toBe("");
  const final = f.stream.next();
  f.resolve({ text: "已保存分镜", status: "completed" });
  expect((await final).value).toBe("已保存分镜");
  expect((await f.stream.next()).done).toBe(true);
});

test("parent wait expiry does not finish the stream; background result continues it", async () => {
  const f = fixture();
  const first = f.stream.next();
  await Bun.sleep(0);
  f.resolve({ text: "Waiting for subtasks", status: "waiting" });
  expect((await first).value).toBe("Waiting for subtasks");
  let ended = false;
  const next = f.stream.next().then((v) => {
    ended = true;
    return v;
  });
  await Bun.sleep(0);
  expect(ended).toBe(false);
  expect(f.closed()).toBe(false);
  f.emit("Here is the saved script");
  expect((await next).value).toBe("Here is the saved script");
  const end = f.stream.next();
  f.end();
  expect((await end).done).toBe(true);
  expect(f.closed()).toBe(true);
});

test("a fast continuation cannot be replaced by the late initial wait reply", async () => {
  const f = fixture();
  const next = f.stream.next();
  await Bun.sleep(0);
  f.emit("Final script");
  f.end();
  f.resolve({ text: "Old waiting text", status: "waiting" });
  expect((await next).value).toBe("Final script");
  expect((await f.stream.next()).done).toBe(true);
});

test("background failure is surfaced after a waiting reply", async () => {
  const f = fixture();
  const first = f.stream.next();
  await Bun.sleep(0);
  f.resolve({ text: "Waiting", status: "waiting" });
  await first;
  const next = f.stream.next();
  f.end("failed", "Result delivery failed");
  await expect(next).rejects.toThrow("Result delivery failed");
  expect(f.closed()).toBe(true);
});

test("an ongoing continuation cannot be replaced by the late initial wait reply", async () => {
  const f = fixture();
  const next = f.stream.next();
  await Bun.sleep(0);
  f.wait();
  f.emit("Receiving the script");
  expect((await next).value).toBe("Receiving the script");
  const result = f.stream.next();
  f.resolve({ text: "Old waiting text", status: "waiting" });
  await Bun.sleep(0);
  f.emit("Final script");
  expect((await result).value).toBe("Final script");
  const end = f.stream.next();
  f.end();
  expect((await end).done).toBe(true);
});

test("stopping a parked parent releases the stream without another model response", async () => {
  const f = fixture();
  const first = f.stream.next();
  await Bun.sleep(0);
  f.resolve({ text: "Waiting", status: "waiting" });
  await first;
  const next = f.stream.next();
  f.abort();
  await expect(next).rejects.toThrow("已停止");
  expect(f.closed()).toBe(true);
});

test("cancellation from another window is reported as cancellation", async () => {
  const f = fixture();
  const first = f.stream.next();
  await Bun.sleep(0);
  f.resolve({ text: "Waiting", status: "waiting" });
  await first;
  const next = f.stream.next();
  f.end("cancelled", "Stopped");
  await expect(next).rejects.toMatchObject({ name: "AbortError" });
  expect(f.closed()).toBe(true);
});
