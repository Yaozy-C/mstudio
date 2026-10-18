import { expect, test } from "bun:test";
import type { ContextType } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { newProject } from "../model";
import { setLanguage } from "../i18n";
import { Context } from "./context";
import { newModel, type PrevisExport } from "./types";
import { PrevisVideoDetail } from "./PrevisVideoDetail";

const video: PrevisExport = {
  id: "export",
  fileId: "shot",
  fileRevision: 1,
  modelRevisions: {},
  signature: "sig",
  asset: {
    id: "video",
    name: "Saved previs",
    path: "/saved.mp4",
    preview: "",
    kind: "video",
    duration: 2.7,
    width: 720,
    height: 1280,
    hasAudio: false,
  },
  savedAssetId: null,
};
function render({
  orphan = false,
  missing = false,
  stale = false,
  linked = false,
} = {}) {
  const project = newProject("Previs browser");
  if (!orphan)
    project.nodes = [
      {
        id: "shot",
        title: "Shot one",
        kind: "shot",
        x: 0,
        y: 0,
        text: "",
        shot: { screenplayId: "script", order: 1, duration: 2.7, dialogue: "" },
        references: linked ? [{ assetId: "video", purpose: "Motion" }] : [],
      },
    ];
  const value = {
    project,
    tasks: [],
    workspace: {
      files: [{ ...newModel("Scene"), id: "shot", revision: stale ? 2 : 1 }],
      exports: [video],
      messages: [],
    },
  } as unknown as NonNullable<ContextType<typeof Context>>;
  return renderToStaticMarkup(
    <Context.Provider value={value}>
      <PrevisVideoDetail
        video={{ ...video, asset: { ...video.asset, missing } }}
      />
    </Context.Provider>,
  );
}

test("a saved previs is directly playable with its actual size, duration and reference actions", () => {
  const html = render();
  expect(html).toContain('src="/saved.mp4"');
  expect(html).toContain("controls=");
  expect(html).toContain("2.70 秒");
  expect(html).toContain("720 × 1280");
  expect(html).toContain("设为参考");
  expect(html).toContain("保存为项目素材");
});

test("removed shots keep their saved videos playable without offering an invalid reference", () => {
  const html = render({ orphan: true });
  expect(html).toContain("镜头已移除，视频已保留");
  expect(html).toContain("<video");
  expect(html.match(/<button[^>]*disabled=""/g)).toHaveLength(2);
  expect(html).toContain("保存为项目素材");
});

test("a changed source is identified without hiding the previous playable render", () => {
  const html = render({ stale: true });
  expect(html).toContain("场景已有修改；此视频为上次保存的预演。");
  expect(html).toContain("<video");
});

test("missing media displays a recovery message and disables media sharing", () => {
  const html = render({ missing: true });
  expect(html).not.toContain("<video");
  expect(html).toContain("预演视频不可用，请重新生成。");
  expect(html.match(/<button[^>]*disabled=""/g)).toHaveLength(2);
});

test("the panel's actions and state messages are localized", () => {
  setLanguage("en");
  try {
    const html = render({ orphan: true });
    expect(html).toContain("Shot removed; video retained");
    expect(html).toContain("View or edit previs");
    expect(html).toContain("Save to project media");
  } finally {
    setLanguage("zh-CN");
  }
});
