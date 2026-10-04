import { expect, test } from "bun:test";
import { mergeValue } from "./merge";
import { newProject, type Project } from "../model";
import { createProjectAutosave } from "../workspace/projectAutosave";
import { commandError } from "../errors/commands";
import { normalizeError } from "../errors/catalog";
test("parallel entity edits merge while overlapping changes and reorderings conflict", () => {
  const base = {
    nodes: [
      { id: "a", text: "A" },
      { id: "b", text: "B" },
    ],
  };
  const local = { nodes: [{ id: "a", text: "local A" }, base.nodes[1]] };
  const remote = { nodes: [base.nodes[0], { id: "b", text: "remote B" }] };
  expect(mergeValue(base, local, remote)).toEqual({
    nodes: [
      { id: "a", text: "local A" },
      { id: "b", text: "remote B" },
    ],
  });
  expect(() =>
    mergeValue(base, local, {
      nodes: [{ id: "a", text: "other A" }, base.nodes[1]],
    }),
  ).toThrow("/nodes/a/text");
  expect(() => mergeValue(base, local, { nodes: [base.nodes[1]] })).toThrow(
    "/nodes/a",
  );
  const ordered = { nodes: [...base.nodes, { id: "c", text: "C" }] };
  expect(() =>
    mergeValue(
      ordered,
      { nodes: [ordered.nodes[1], ordered.nodes[0], ordered.nodes[2]] },
      { nodes: [ordered.nodes[0], ordered.nodes[2], ordered.nodes[1]] },
    ),
  ).toThrow("order");
});

test("save conflict requires an explicit decision and preserves independent remote fields", async () => {
  const base = { ...newProject("conflict"), storageVersion: 1 };
  const remote = {
    ...base,
    brief: "agent",
    name: "remote name",
    storageVersion: 2,
  };
  let current: Project = base;
  const save = createProjectAutosave(
    base,
    async (local, previous) => {
      try {
        return mergeValue(previous, local, remote) as Project;
      } catch (error) {
        throw new Error(commandError("save_project", error));
      }
    },
    (p) => {
      current = p;
    },
  );
  save.update({ ...base, brief: "human" });
  await expect(save.flush()).rejects.toThrow();
  expect(save.isConflicted()).toBe(true);
  expect(save.isDirty()).toBe(true);
  save.resolve(remote, "local");
  await save.flush();
  expect(current.brief).toBe("human");
  expect(current.name).toBe("remote name");
  expect(save.isConflicted()).toBe(false);
  expect(save.isDirty()).toBe(false);
  try {
    mergeValue(base, { ...base, brief: "human" }, remote);
  } catch (error) {
    expect(normalizeError(error).code).toBe("PROJECT_CONFLICT");
  }
});
test("autosave rebases edits made during a write and ignores older remote snapshots", async () => {
  const base = { ...newProject("merge"), storageVersion: 1 };
  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  let value: Project = base;
  const writes: unknown[] = [];
  const save = createProjectAutosave(
    base,
    async (snapshot, previous) => {
      writes.push([snapshot, previous]);
      if (writes.length === 1) await gate;
      return {
        ...snapshot,
        name: "remote name",
        storageVersion: writes.length + 1,
      };
    },
    (project) => {
      value = project;
    },
  );
  save.update({ ...base, brief: "first" });
  const pending = save.flush();
  await Promise.resolve();
  save.update({ ...base, brief: "second" });
  release();
  await pending;
  expect(value.brief).toBe("second");
  expect(value.name).toBe("remote name");
  expect(value.storageVersion).toBe(3);
  save.accept({ ...base, name: "stale event", storageVersion: 2 });
  expect(value.name).toBe("remote name");
  expect(save.isDirty()).toBe(false);
});
