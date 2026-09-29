import { expect, test } from "bun:test";
import { newProject } from "../model";
import { createScript } from "../creative/script";
import { defaultAgent, workTarget, taskFocus } from "./workContext";
test("workspace never implicitly selects a specialist", () => {
  expect(defaultAgent({ view: "script" })).toBe("coordinator");
  expect(defaultAgent({ view: "storyboard" })).toBe("coordinator");
  expect(defaultAgent({ view: "film" })).toBe("coordinator");
  expect(defaultAgent()).toBe("coordinator");
});
test("script focus carries exact object and rejects deleted paragraphs", () => {
  const project = createScript(newProject("Scripts"));
  const id = project.nodes.at(-1)!.id;
  expect(workTarget(project, { view: "script", screenplayId: id })).toBe(id);
  expect(() =>
    workTarget(project, { view: "script", screenplayId: "missing" }),
  ).toThrow("移除");
  expect(() =>
    workTarget(project, {
      view: "script",
      screenplayId: id,
      paragraphId: "missing",
    }),
  ).toThrow("段落");
  expect(workTarget(project, { view: "script" })).toBeNull();
  expect(workTarget(project, { view: "film", screenplayId: id })).toBeNull();
});

test("an explicitly referenced script overrides the visible script", () => {
  let project = createScript(newProject("Scripts"));
  const first = project.nodes.at(-1)!.id;
  const legacy = createScript(newProject("Legacy"));
  project = { ...project, nodes: [...project.nodes, ...legacy.nodes] };
  const second = project.nodes.at(-1)!.id;
  expect(
    taskFocus(
      project,
      { view: "script", screenplayId: first },
      null,
      [{ kind: "node", id: second }],
      false,
    ),
  ).toEqual({ target: second, agent: "coordinator" });
  expect(
    taskFocus(
      project,
      { view: "script", screenplayId: first },
      first,
      [{ kind: "node", id: second }],
      false,
    ).target,
  ).toBe(first);
  expect(
    taskFocus(project, { view: "script", screenplayId: first }, first, [], true)
      .agent,
  ).toBe("coordinator");
});
