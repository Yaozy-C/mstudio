import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { Theme } from "@radix-ui/themes";
import { CanvasCard } from "./CanvasCard";
import { asset, fixture } from "./fixtures.test-helper";
import { draftItems } from "./items";
import { mediaCardSize } from "./mediaCardSize";
import type { ProductionController } from "./useProduction";

test("landscape and portrait cards preserve the original picture ratio and use original pixels", () => {
  for (const [width, height] of [
    [1672, 941],
    [941, 1672],
    [1024, 1024],
    [2400, 600],
  ]) {
    const p = fixture();
    const image = {
      ...asset("picture"),
      path: "/original.png",
      preview: "/thumbnail.jpg",
      width,
      height,
    };
    p.assets = [image];
    p.nodes = [
      {
        id: "picture",
        kind: "asset",
        title: "图片",
        assetId: image.id,
        text: "",
        x: 0,
        y: 0,
      },
    ];
    const [item] = draftItems(p);
    expect((item.height - 54) / (item.width - 26)).toBeCloseTo(height / width);
    const html = renderToStaticMarkup(
      <Theme>
        <CanvasCard
          item={item}
          asset={image}
          compact={false}
          project={p}
          canvas={
            {
              selected: [],
              referenced: new Set(),
            } as unknown as ProductionController
          }
          preview={() => {}}
          collect={() => {}}
          onAdd={() => {}}
        />
      </Theme>,
    );
    expect(html).toContain("height:auto");
    expect(html).toContain('src="/original.png"');
    expect(html).not.toContain("/thumbnail.jpg");
    expect(html).toContain(`width="${width}" height="${height}"`);
  }
});

test("invalid picture dimensions are rejected rather than inventing an aspect ratio", () => {
  for (const width of [0, -1, Infinity, NaN])
    expect(() => mediaCardSize({ ...asset("bad"), width })).toThrow("尺寸");
});
