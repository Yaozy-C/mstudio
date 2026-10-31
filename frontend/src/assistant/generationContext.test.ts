import { expect, test } from "bun:test";
import { fixture, until } from "./chatAdapter.fixture";

test("sent chat carries referenced result media without either task's old prompt", async () => {
  const f = fixture(true);
  f.context.canvas!.task!.prompt = "OLD_COMPOSER_PROMPT";
  f.context.canvas!.referencedTask = {
    ...f.context.canvas!.task!,
    key: "referenced-task",
    title: "Pack · storyboard",
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
  result.resolve({ text: "完成", status: "completed" });
  await sending;
});

test("Agent dialogue never reads or consumes the media-generation draft", async () => {
  const f = fixture(true);
  f.context.canvas!.task!.inputs = [
    {
      key: "b",
      assetId: "b",
      purpose: "Unrelated generation input",
      role: "reference",
    },
  ];
  f.thread.composer.setText("根据本次资料写脚本");
  const original = structuredClone(f.context.canvas!.task);
  const sending = f.thread.composer.send();
  f.flush.resolve();
  await until(() => f.calls.length === 1);
  expect(f.calls[0].args.attachments).toEqual([f.a]);
  expect(f.calls[0].args.prompt).not.toContain("Unrelated generation input");
  expect(f.context.canvas!.task).toEqual(original);
  f.calls[0].result.resolve({ text: "完成", status: "completed" });
  await sending;
});
