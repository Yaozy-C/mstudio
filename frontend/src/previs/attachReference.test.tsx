import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { Theme } from "@radix-ui/themes";
import type { Asset } from "../model";
import { newProject } from "../model";
import { attachPrevisVideo, withPrevisReference } from "./attachReference";
import { PrevisExports } from "./PrevisExports";
import type { PrevisEditorState } from "./usePrevisEditor";

const asset: Asset = {
  id: "shared-video",
  name: "Previs",
  path: "/test.mp4",
  preview: "",
  kind: "video",
  duration: 2.7,
  width: 720,
  height: 1280,
  hasAudio: false,
  inLibrary: true,
};
function project() {
  const p = newProject("Reference test");
  p.nodes = [
    {
      id: "shot",
      kind: "shot",
      title: "Shot",
      text: "",
      x: 0,
      y: 0,
      shot: { screenplayId: "script", order: 1, duration: 2.7, dialogue: "" },
      references: [{ assetId: "photo", purpose: "Product appearance" }],
    },
  ];
  return p;
}
test("one action imports a private render and persists its full-duration shot reference", async () => {
  let p = project();
  const steps: string[] = [];
  const attach = () =>
    attachPrevisVideo({
      fileId: "shot",
      videoId: "private-render",
      share: async (id) => {
        steps.push(id);
        return { asset };
      },
      change: (fn) => {
        p = fn(p);
        steps.push("change");
      },
      flush: async () => {
        steps.push("flush");
      },
      refresh: async () => {
        steps.push("refresh");
      },
    });
  await attach();
  expect(steps).toEqual([
    "flush",
    "private-render",
    "change",
    "flush",
    "refresh",
  ]);
  expect(p.assets).toEqual([asset]);
  expect(p.nodes[0].references).toEqual([
    { assetId: "photo", purpose: "Product appearance" },
    {
      assetId: asset.id,
      purpose: "预演动作、构图与运镜参考；商品外观以实物参考为准",
      start: 0,
      end: 2.7,
    },
  ]);
  p.nodes[0].references![1].start = 0.5;
  await attach();
  expect(p.assets).toHaveLength(1);
  expect(p.nodes[0].references).toHaveLength(2);
  expect(p.nodes[0].references![1].start).toBe(0.5);
});
test("invalid or missing targets cannot receive a silent or broken reference", () => {
  expect(() => withPrevisReference(project(), "deleted-shot", asset)).toThrow(
    "镜头已删除",
  );
  expect(() =>
    withPrevisReference(project(), "shot", { ...asset, duration: 0 }),
  ).toThrow("预演视频不可用");
});
test("save failures propagate instead of reporting a successfully persisted reference", async () => {
  let flushes = 0;
  await expect(
    attachPrevisVideo({
      fileId: "shot",
      videoId: "video",
      share: async () => ({ asset }),
      change: () => {},
      flush: async () => {
        if (++flushes === 2) throw Error("Save failed");
      },
      refresh: async () => {},
    }),
  ).rejects.toThrow("Save failed");
});
test("unshared renders have an enabled add-reference action without an Agent-sharing prerequisite", () => {
  const p = project();
  const state = {
    exports: [
      {
        id: "video",
        fileId: "shot",
        fileRevision: 0,
        modelRevisions: {},
        asset,
        sharedAssetId: null,
      },
    ],
    project: p,
    workspace: { files: [] },
    busy: null,
    fileId: "shot",
    linkingId: "",
  } as unknown as PrevisEditorState;
  const html = renderToStaticMarkup(
    <Theme>
      <PrevisExports state={state} />
    </Theme>,
  );
  expect(html).toContain("设为参考");
  expect(html).not.toContain("保存视频到");
  expect(html).not.toContain("场景已有修改");
  expect(html).not.toContain("checkbox");
  expect(html.match(/<button/g)).toHaveLength(1);
  expect(html).not.toContain("提供给 Agent");
  const button = html.match(
    /<button[^>]*class="async-button primary"[^>]*>/,
  )?.[0];
  expect(button).toBeDefined();
  expect(button).not.toContain("disabled");
});

test("a shot displays only its latest video even when loading an older stacked workspace", () => {
  const state = {
    exports: [
      {
        id: "latest",
        fileId: "shot",
        asset: { ...asset, name: "Latest result", path: "/latest.mp4" },
        sharedAssetId: null,
      },
      {
        id: "old",
        fileId: "shot",
        asset: { ...asset, name: "Old result", path: "/old.mp4" },
        sharedAssetId: null,
      },
    ],
    project: project(),
    busy: null,
    fileId: "shot",
    linkingId: "",
  } as unknown as PrevisEditorState;
  const html = renderToStaticMarkup(
    <Theme>
      <PrevisExports state={state} />
    </Theme>,
  );
  expect(html).toContain("Latest result");
  expect(html).not.toContain("Old result");
  expect(html.match(/<video/g)).toHaveLength(1);
  expect(html.match(/<button/g)).toHaveLength(1);
});
