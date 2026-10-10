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
