import { expect, test } from "bun:test";
import { Theme } from "@radix-ui/themes";
import { renderToStaticMarkup } from "react-dom/server";
import { makeClip, splitClip } from "../model";
import { CanvasCard } from "./CanvasCard";
import { timelineAssetIds } from "./canvasIndex";
import { fixture } from "./fixtures.test-helper";
import { productionItems } from "./items";
import { MediaReferenceStrip } from "../assistant/MediaReferenceStrip";
import { attachmentInput } from "./attachmentInput";
import { createTask } from "./tasks";
import type { AttachmentDraft } from "../assistant/useAttachments";
import type { ProductionController } from "./useProduction";

test("usage tracks actual clips through split, removal, replacement and undo", () => {
  const p = fixture();
  const video = p.assets.find((a) => a.id === "v")!;
  const clip = makeClip(video);
  const original = [clip];
  const split = splitClip(original, clip.id, 2);
  expect(split).toHaveLength(2);
  expect([...timelineAssetIds(split)]).toEqual(["v"]);
  expect(timelineAssetIds(split.slice(1)).has("v")).toBe(true);
  expect(timelineAssetIds([]).has("v")).toBe(false);
  expect(timelineAssetIds(original).has("v")).toBe(true);
  expect([...timelineAssetIds([{ ...clip, assetId: "replacement" }])]).toEqual([
    "replacement",
  ]);
  // Script references and generated takes alone do not mean used in the edit.
  expect(productionItems(p).some((item) => item.assetId === "v")).toBe(true);
  expect(timelineAssetIds(p.clips).size).toBe(0);
});

test("image/audio clips count as used even when their tracks are hidden or muted", () => {
  const p = fixture();
  p.tracks[0].hidden = true;
  p.tracks[1].muted = true;
  p.clips = [
    makeClip(p.assets[0]),
    makeClip({ ...p.assets[1], kind: "audio" }),
  ];
  expect([...timelineAssetIds(p.clips)]).toEqual(["a", "b"]);
});

test("card usage is visible independently of selection and explicit references", () => {
  const p = fixture();
  const item = productionItems(p).find((i) => i.assetId === "v")!;
  const asset = p.assets.find((a) => a.id === item.assetId)!;
  const canvas = {
    selected: [item.key],
    referenced: new Set([item.key]),
  } as unknown as ProductionController;
  const render = (inTimeline: boolean, compact = false) =>
    renderToStaticMarkup(
      <Theme>
        <CanvasCard
          item={item}
          asset={asset}
          project={p}
          canvas={canvas}
          compact={compact}
          inTimeline={inTimeline}
          preview={() => {}}
          collect={() => {}}
          onAdd={() => {}}
        />
      </Theme>,
    );
  expect(render(false)).not.toContain("已加入时间线");
  expect(render(true)).toContain("已加入时间线");
  expect(render(true, true)).toContain("已加入时间线");
  expect(render(true)).not.toContain("已确认");
});

test("chat labels distinguish a source asset from two uses of the same asset", () => {
  const p = fixture();
  const asset = p.assets.find((a) => a.id === "v")!;
  p.clips = [
    { ...makeClip(asset), id: "first", trimOut: 4 },
    { ...makeClip(asset), id: "second", trimOut: 4, start: 6 },
  ];
  const task = createTask(p, []);
  task.inputs = [
    attachmentInput(p, { kind: "asset", id: asset.id })!,
    attachmentInput(p, { kind: "clip", id: "first" })!,
    attachmentInput(p, { kind: "clip", id: "second" })!,
  ];
  const canvas = {
    task,
    composerMode: "agent",
    media: { models: [] },
    modelPreferences: {},
  } as unknown as ProductionController;
  const html = renderToStaticMarkup(
    <Theme>
      <MediaReferenceStrip
        canvas={canvas}
        project={p}
        draft={{} as AttachmentDraft}
        extrasOnly
      />
    </Theme>,
  );
  expect(html).toContain("0–4s · v");
  expect(html).toContain("6–10s · v");
  expect(html.match(/<small>时间线片段<\/small>/g)).toHaveLength(2);
  expect(html).not.toContain('class="reference-role"');
});
