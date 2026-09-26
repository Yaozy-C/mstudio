import { expect, test } from "bun:test";
import { mentionOptions } from "./mentionOptions";
import { newProject, makeClip, type Asset } from "../model";
import { defaultAgents } from "../agents/catalog";
const defaultAgent = defaultAgents[0];

test("unified mentions preserve object identities across script, shot, asset and clip", () => {
  const p = newProject("test");
  const asset: Asset = {
    id: "a",
    name: "Lunch",
    kind: "video",
    path: "local",
    preview: "",
    duration: 6,
    width: 100,
    height: 100,
    hasAudio: true,
  };
  p.assets = [asset];
  p.clips = [{ ...makeClip(asset), id: "clip", start: 12 }];
  p.nodes = [
    { id: "plan", kind: "plan", title: "Lunch 脚本", text: "", x: 0, y: 0 },
    { id: "shot", kind: "shot", title: "Lunch 镜头", text: "", x: 0, y: 0 },
  ];
  const matches = mentionOptions(p, [defaultAgent], "lunch");
  expect(matches.map((o) => o.key)).toEqual([
    "node:plan",
    "node:shot",
    "asset:a",
    "clip:clip",
  ]);
  expect(
    mentionOptions(p, [defaultAgent], "", "@").every(
      (o) => !!o.ref && !o.agent,
    ),
  ).toBe(true);
  expect(
    mentionOptions(p, [defaultAgent], "", "$").every(
      (o) => !!o.agent && !o.ref,
    ),
  ).toBe(true);
  expect(matches.at(-1)?.title).toContain("12–18s");
  expect(mentionOptions(p, [], "时间线")[0].ref).toEqual({
    kind: "clip",
    id: "clip",
  });
  expect(mentionOptions(p, [], "脚本")[0].key).toBe("node:plan");
  expect(mentionOptions(p, [], "not found")).toHaveLength(0);
  expect(
    mentionOptions(p, [{ ...defaultAgent, enabled: false }], "").every(
      (o) => !o.agent,
    ),
  ).toBe(true);
  expect(
    mentionOptions(p, [defaultAgent], defaultAgent.name, "$")[0].agent?.id,
  ).toBe(defaultAgent.id);
});
