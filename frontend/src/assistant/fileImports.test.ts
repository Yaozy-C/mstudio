import { expect, test } from "bun:test";
import { importFileName, transferFiles } from "./fileImports";

test("accepts images, videos, audio and documents while preserving names", () => {
  for (const name of [
    "照片.PNG",
    "使用视频.mov",
    "口播.wav",
    "说明.md",
    "数据.csv",
    "要求.json",
    "参考.pdf",
  ]) {
    expect(importFileName({ name, type: "", size: 100 })).toBe(name);
  }
  expect(importFileName({ name: "", type: "image/png", size: 100 })).toBe(
    "粘贴的文件.png",
  );
});

test("rejects empty, unsupported and oversized files before transferring", () => {
  for (const file of [
    { name: "应用.exe", type: "", size: 100 },
    { name: "空.txt", type: "text/plain", size: 0 },
    { name: "说明.txt", type: "text/plain", size: 120_001 },
    { name: "文档.pdf", type: "application/pdf", size: 12 * 1024 * 1024 + 1 },
    { name: "视频.mp4", type: "video/mp4", size: 1024 ** 3 + 1 },
  ])
    expect(() => importFileName(file)).toThrow();
});

function data(
  files: File[],
  items: { kind: string; getAsFile: () => File | null }[],
) {
  return { files, items } as unknown as DataTransfer;
}
test("does not duplicate a file exposed through both clipboard APIs", () => {
  const file = new File(["reference"], "brief.txt");
  expect(
    transferFiles(data([file], [{ kind: "file", getAsFile: () => file }])),
  ).toEqual([file]);
});

test("collects all file items and leaves ordinary text alone", () => {
  const one = new File(["one"], "one.txt");
  const two = new File(["two"], "two.txt");
  expect(
    transferFiles(
      data(
        [],
        [
          { kind: "string", getAsFile: () => null },
          { kind: "file", getAsFile: () => one },
          { kind: "file", getAsFile: () => two },
          { kind: "file", getAsFile: () => null },
        ],
      ),
    ),
  ).toEqual([one, two]);
  expect(
    transferFiles(data([], [{ kind: "string", getAsFile: () => null }])),
  ).toEqual([]);
});
