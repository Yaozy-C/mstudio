import { test, expect } from "bun:test";
import { defaultAgents } from "../agents/catalog";
const defaultAgent = defaultAgents[0];
import { matchingAgents, mentionQuery, resolveMention } from "./mentions";
const catalog = {
  defaultId: "default",
  selectedId: null,
  profiles: [
    {
      id: "default",
      name: "Default",
      endpoint: "http://localhost:1",
      model: "base",
      adapter: "openai-compatible" as const,
      inputs: { image: false, audio: false, video: false, document: false },
      hasKey: false,
    },
    {
      id: "review-model",
      name: "Review",
      endpoint: "http://localhost:2",
      model: "review",
      adapter: "openai-compatible" as const,
      inputs: { image: true, audio: false, video: false, document: false },
      hasKey: false,
    },
  ],
};
const reviewer = {
  ...defaultAgent,
  id: "reviewer",
  name: "审片员",
  skillIds: [],
  toolIds: ["project-read"],
};
test("switching Agent preserves the conversation model and independent tools", () => {
  const agents = [defaultAgent, reviewer];
  const route = resolveMention(agents, catalog, "reviewer");
  expect(route.agent.id).toBe("reviewer");
  expect(route.model.id).toBe("default");
  const selected = { ...catalog, selectedId: "review-model" };
  expect(resolveMention(agents, selected, "reviewer").model.id).toBe(
    "review-model",
  );
  expect(resolveMention(agents, selected, null).model.id).toBe("review-model");
  expect(route.agent.toolIds).toEqual(["project-read"]);
  expect(resolveMention(agents, catalog, null).agent.id).toBe("coordinator");
  expect(resolveMention(agents, catalog, null).model.id).toBe("default");
});
test("missing/disabled/unconfigured references fail instead of falling back", () => {
  expect(() => resolveMention([defaultAgent], catalog, "missing")).toThrow();
  expect(() =>
    resolveMention(
      [defaultAgent, { ...reviewer, enabled: false }],
      catalog,
      reviewer.id,
    ),
  ).toThrow();
  expect(() =>
    resolveMention(
      [defaultAgent, reviewer],
      { ...catalog, selectedId: "deleted" },
      reviewer.id,
    ),
  ).toThrow();
  expect(() =>
    resolveMention(
      [defaultAgent, reviewer],
      { ...catalog, profiles: [] },
      reviewer.id,
    ),
  ).toThrow();
});
test("mention search supports Chinese and spaces without treating email as a mention", () => {
  expect(mentionQuery("请 @审片", 5)).toEqual({
    start: 2,
    end: 5,
    query: "审片",
    symbol: "@",
  });
  expect(mentionQuery("@创作 Agent", 9)?.query).toBe("创作 Agent");
  expect(mentionQuery("contact@example.com", 19)).toBeNull();
  expect(mentionQuery("@审片\n新行", 6)).toBeNull();
  expect(
    matchingAgents(
      [defaultAgent, reviewer, { ...reviewer, id: "off", enabled: false }],
      "审片",
    ).map((a) => a.id),
  ).toEqual(["reviewer"]);
});

test("specialists resolve independently through the project model", () => {
  expect(defaultAgents.map((a) => a.id)).toEqual([
    "coordinator",
    "concept",
    "storyboard",
    "storyboard-artist",
    "production",
    "editor",
    "reviewer",
    "colorist",
    "transition-designer",
  ]);
  for (const a of defaultAgents.filter((a) => a.enabled))
    expect(resolveMention(defaultAgents, catalog, a.id).agent).toBe(a);
  expect(defaultAgents.find((a) => a.id === "reviewer")!.enabled).toBe(false);
  expect(() => resolveMention(defaultAgents, catalog, "reviewer")).toThrow();
  expect(defaultAgents.find((a) => a.id === "reviewer")!.toolIds).not.toContain(
    "project-edit",
  );
});

test("dollar selects agents while at-sign selects elements without matching embedded symbols", () => {
  expect(mentionQuery("请 $分镜", 5)).toEqual({
    start: 2,
    end: 5,
    query: "分镜",
    symbol: "$",
  });
  expect(mentionQuery("价格 US$20", 8)).toBeNull();
  expect(mentionQuery("@镜头 $审片", 7)?.symbol).toBe("$");
});
