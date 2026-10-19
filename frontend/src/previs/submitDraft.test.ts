import { expect, test } from "bun:test";
import type { Asset } from "../model";
import { submitPrevisDraft } from "./submitDraft";

const photo = {
  id: "photo",
  name: "Bag.png",
  kind: "image",
  path: "/assets/bag.png",
} as Asset;

test("submitting transfers photos to the request and clears the composer while the model is still running", async () => {
  let prompt = "根据图片生成预演";
  let items = [photo];
  let finish!: (accepted: boolean) => void;
  let sent: { text: string; images: Asset[] } | undefined;
  const pending = submitPrevisDraft(
    prompt,
    {
      items,
      clear: () => {
        items = [];
      },
      restore: (value) => {
        items = value;
      },
    },
    (text) => {
      prompt = text;
    },
    (text, images) => {
      sent = { text, images };
      return new Promise<boolean>((resolve) => {
        finish = resolve;
      });
    },
    () => {},
  );
  expect(prompt).toBe("");
  expect(items).toEqual([]);
  expect(sent).toEqual({ text: "根据图片生成预演", images: [photo] });
  // The host accepted the message; success/failure of generation is tracked separately.
  finish(true);
  await pending;
  expect(prompt).toBe("");
  expect(items).toEqual([]);
});

test("a request rejected before acceptance restores text and photos for retry", async () => {
  let prompt = "生成";
  let items = [photo];
  await submitPrevisDraft(
    prompt,
    {
      items,
      clear: () => {
        items = [];
      },
      restore: (value) => {
        items = value;
      },
    },
    (text) => {
      prompt = text;
    },
    async () => false,
    () => {},
  );
  expect(prompt).toBe("生成");
  expect(items).toEqual([photo]);
});
