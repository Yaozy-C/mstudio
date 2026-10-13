import { expect, test } from "bun:test";
import { bridge } from "./bridge";
import { fixture } from "./production/fixtures.test-helper";
import type { Project, ProjectSummary } from "./model";

test("preview library summaries keep authoritative documents available for get, save and delete", async () => {
  const previous = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
  const values = new Map<string, string>();
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, value),
    },
  });
  try {
    const document = fixture();
    await bridge("create_project", { document });
    const summaries = await bridge<ProjectSummary[]>("list_projects");
    expect(summaries[0]).toMatchObject({
      id: document.id,
      width: document.width,
      clipCount: document.clips.length,
    });
    expect(summaries[0]).not.toHaveProperty("document");
    expect(
      await bridge<Project>("get_project", { projectId: document.id }),
    ).toEqual(document);
    const changed = { ...document, name: "Changed" };
    await bridge("save_project", { base: document, document: changed });
    expect(
      (await bridge<Project>("get_project", { projectId: document.id })).name,
    ).toBe("Changed");
    await bridge("delete_project", { id: document.id });
    expect(await bridge<ProjectSummary[]>("list_projects")).toEqual([]);
  } finally {
    if (previous) Object.defineProperty(globalThis, "localStorage", previous);
    else Reflect.deleteProperty(globalThis, "localStorage");
  }
});
