import { expect, test } from "bun:test";
import { newProject } from "../model";
import { initialStudioView } from "./initialStudioView";

test("empty and material-only projects open the concept board", () => {
  const project = newProject("Empty");
  expect(initialStudioView(project)).toBe("concepts");
  project.assets.push({ id: "reference" } as (typeof project.assets)[number]);
  expect(initialStudioView(project)).toBe("concepts");
});

test("existing work and explicit script tasks retain the script entry", () => {
  const project = newProject("Existing");
  expect(initialStudioView(project, { agentId: "writer", text: "Write" })).toBe(
    "script",
  );
  project.nodes.push({
    id: "script",
    kind: "screenplay",
  } as (typeof project.nodes)[number]);
  expect(initialStudioView(project)).toBe("script");
  expect(
    initialStudioView(project, { agentId: "concept", text: "Explore" }),
  ).toBe("concepts");
});
