import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import type { ContextType } from "react";
import { Context } from "../previs/context";
import { fixture } from "./fixtures.test-helper";
import { productionItems } from "./items";
import { ShotCardHeader } from "./ShotCardHeader";
import { ShotCardText } from "./ShotCardText";

test("shot headers carry the numbered title and previews, with playback only for a saved shot video", () => {
  const p = fixture();
  const item = productionItems(p).find((i) => i.kind === "script")!;
  const value = { workspace: { exports: [] } } as unknown as NonNullable<
    ContextType<typeof Context>
  >;
  const render = () =>
    renderToStaticMarkup(
      <Context.Provider value={value}>
        <ShotCardHeader item={item} project={p} />
        <ShotCardText item={item} project={p} />
      </Context.Provider>,
    );
  expect(render()).toContain("SHOT 01");
  expect(render()).not.toContain("播放预演");
  expect(render()).toMatch(/<header[^>]*>[\s\S]*三维预演[\s\S]*<\/header>/);
  expect(render().match(/三维预演/g)).toHaveLength(1);
  const video = {
    id: "previs",
    fileId: "other",
    asset: { id: "video" },
  } as (typeof value.workspace.exports)[number];
  value.workspace.exports = [video];
  expect(render()).not.toContain("播放预演");
  video.fileId = item.nodeId!;
  expect(render()).toContain("播放预演");
});
