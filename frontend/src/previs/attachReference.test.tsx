import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { Theme } from "@radix-ui/themes";
import type { Asset } from "../model";
import { newProject } from "../model";
import { attachPrevisVideo } from "./attachReference";
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
  generated: true,
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
test("setting a reference delegates one atomic write and never imports an asset", async () => {
  const steps: string[] = [];
  await attachPrevisVideo({
    flush: async () => {
      steps.push("flush");
    },
    attach: async () => {
      steps.push("attach");
    },
    refresh: async () => {
      steps.push("refresh");
    },
  });
  expect(steps).toEqual(["flush", "attach", "refresh"]);
});
test("host errors do not report a saved reference", async () => {
  let refreshed = false;
  await expect(
    attachPrevisVideo({
      flush: async () => {},
      attach: async () => {
        throw Error("Shot deleted");
      },
      refresh: async () => {
        refreshed = true;
      },
    }),
  ).rejects.toThrow("Shot deleted");
  expect(refreshed).toBe(false);
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
        savedAssetId: null,
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
  expect(html.match(/<button/g)).toHaveLength(2);
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
        savedAssetId: null,
      },
      {
        id: "old",
        fileId: "shot",
        asset: { ...asset, name: "Old result", path: "/old.mp4" },
        savedAssetId: null,
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
  expect(html.match(/<button/g)).toHaveLength(2);
});
