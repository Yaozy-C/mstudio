import { expect, test } from "bun:test";
import { commentRegion, imagePoint, imageReviewTask } from "./annotations";
import { readCommentDraft, saveCommentDraft } from "./commentDraft";
import type { Asset } from "../../model";

test("annotation coordinates follow the image through zoom, pan and letterboxing", () => {
  const viewport = { width: 800, height: 600 };
  const natural = { width: 400, height: 800 };
  expect(
    imagePoint({ x: 250, y: 0 }, viewport, natural, {
      scale: 0.75,
      x: 0,
      y: 0,
    }),
  ).toEqual({ x: 0, y: 0 });
  expect(
    imagePoint({ x: 100, y: 300 }, viewport, natural, {
      scale: 0.75,
      x: 0,
      y: 0,
    }),
  ).toBeNull();
  expect(
    imagePoint({ x: 300, y: 550 }, viewport, natural, {
      scale: 2,
      x: 100,
      y: -150,
    }),
  ).toEqual({ x: 0.25, y: 0.75 });
});

test("reverse dragging and edge clicks create bounded, nonempty regions", () => {
  const region = commentRegion({ x: 0.8, y: 0.7 }, { x: 0.2, y: 0.1 });
  expect(region.x).toBe(0.2);
  expect(region.y).toBe(0.1);
  expect(region.width).toBeCloseTo(0.6);
  expect(region.height).toBeCloseTo(0.6);
  expect(commentRegion({ x: 1, y: 1 }, { x: 1, y: 1 })).toEqual({
    x: 0.975,
    y: 0.975,
    width: 0.025,
    height: 0.025,
  });
});

test("edit handoff separates the original from marked reference and preserves each comment", () => {
  const task = imageReviewTask(
    { id: "original", name: "bag" } as Asset,
    { id: "marked" } as Asset,
    [
      {
        id: "one",
        x: 0.2,
        y: 0.1,
        width: 0.3,
        height: 0.4,
        text: "提手自然垂落",
      },
    ],
  );
  expect(task.refs?.map((ref) => ref.id)).toEqual(["original"]);
  expect(task.agentId).toBe("image");
  expect(task.text).toContain('mstudio_read_image(assetId="marked")');
  expect(task.text).toContain(
    "不要把内部批注说明图添加到生成任务的 references",
  );
  expect(task.text).toContain("不要把批注图当作编辑底图");
  expect(task.text).toContain("x=20%, y=10%, 宽=30%, 高=40%：提手自然垂落");
  expect(task.text).toContain("输出干净的新图片");
});

test("comment draft rejects malformed and out-of-bounds regions", () => {
  const previous = globalThis.localStorage;
  const values = new Map<string, string>();
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, value),
      removeItem: (key: string) => values.delete(key),
    },
  });
  try {
    const valid = {
      id: "one",
      x: 0,
      y: 0,
      width: 0.2,
      height: 0.2,
      text: "keep",
    };
    values.set(
      "draft",
      JSON.stringify([
        valid,
        { ...valid, x: 2 },
        { ...valid, width: -1 },
        null,
      ]),
    );
    expect(readCommentDraft("draft")).toEqual([valid]);
    saveCommentDraft("draft", []);
    expect(readCommentDraft("draft")).toEqual([]);
    values.set("draft", "broken");
    expect(readCommentDraft("draft")).toEqual([]);
  } finally {
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: previous,
    });
  }
});
