import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { AgentTools } from "./AgentTools";
import { defaultAgents } from "./catalog";

test("video analyst has no tool controls while other roles retain them", () => {
  const analyst = defaultAgents.find((p) => p.id === "video-analyst")!;
  expect(analyst.toolIds).toEqual([]);
  expect(analyst.skillIds).toEqual(["video-analysis"]);
  const html = renderToStaticMarkup(
    <AgentTools profile={analyst} onChange={() => {}} />,
  );
  expect(html).not.toContain('type="checkbox"');
  const writer = defaultAgents.find((p) => p.id === "writer")!;
  expect(
    renderToStaticMarkup(<AgentTools profile={writer} onChange={() => {}} />),
  ).toContain('type="checkbox"');
});

test("source-material reading is a separate tool assigned only to the three research roles by default", () => {
  expect(
    defaultAgents
      .filter((a) => a.toolIds.includes("material-read"))
      .map((a) => a.id),
  ).toEqual(["concept", "writer", "assets"]);
  const image = defaultAgents.find((a) => a.id === "image")!;
  expect(image.toolIds).toContain("project-assets");
  expect(image.toolIds).not.toContain("material-read");
  const html = renderToStaticMarkup(
    <AgentTools profile={image} onChange={() => {}} />,
  );
  expect(html).toContain("读取创作素材");
});
