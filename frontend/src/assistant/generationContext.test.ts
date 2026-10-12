import { expect, test } from "bun:test";
import { fixture, until } from "./chatAdapter.fixture";

test("sent chat carries referenced result media without either task's old prompt", async () => {
  const f = fixture(true);
  f.context.canvas!.task!.prompt = "OLD_COMPOSER_PROMPT";
  f.context.canvas!.referencedTask = {
    ...f.context.canvas!.task!,
    key: "referenced-task",
    prompt: "OLD_RESULT_PROMPT",
    instruction: "OLD_RESULT_INSTRUCTION",
    status: "COMPLETED",
    resultAssetId: "b",
  };
  f.thread.composer.setText("按这张图做下一个镜头");
  f.flush.resolve();
  const sending = f.thread.composer.send();
  await until(() => f.calls.length === 1);
  const { args, result } = f.calls[0];
  expect(args.prompt).toContain("按这张图做下一个镜头");
  expect(args.prompt).toContain("referenced-task");
  expect(args.prompt).not.toContain("OLD_");
  expect(args.attachments).toEqual([f.a, f.b]);
  result.resolve("完成");
  await sending;
});
