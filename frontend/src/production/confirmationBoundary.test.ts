import { expect, test } from "bun:test";
import { fixture } from "./fixtures.test-helper";
import { requestTask, runsOf } from "./requestTask";
import { editTask } from "./taskEditing";
import { saveTask } from "./document";
import { applyOperations } from "../assistant/projectCommands";

for (const kind of ["image", "video"] as const) {
  for (const purpose of kind === "image" ? [undefined, "asset"] : [undefined]) {
    test(`${kind}/${purpose ?? "standalone"} blocks duplicates and requires user confirmation`, () => {
      const p = fixture();
      const context = {
        callId: "first",
        turnId: "turn",
        turn: {
          projectId: p.id,
          models: {
            image: "model",
            video: "model",
            execution: "automatic" as const,
          },
          instruction: "Generate",
        },
      };
      const op = {
        op: "request_generation",
        mediaKind: kind,
        generationPurpose: purpose,
        text: "Original prompt",
        references: [{ assetId: "a", role: "reference", purpose: "Product" }],
      };
      const first = requestTask(p, op, context);
      const task = runsOf(first)[0];
      expect(task.status).toBe("AWAITING_CONFIRMATION");
      expect(requestTask(first, op, context)).toBe(first);
      expect(() =>
        requestTask(first, op, {
          ...context,
          callId: "again",
          turnId: "later",
        }),
      ).toThrow(task.key);
      const regenerated = applyOperations(
        first,
        [{ op: "regenerate_generation", taskKey: task.key }],
        { ...context, callId: "regenerate" },
      );
      expect(runsOf(regenerated)).toHaveLength(1);
      expect(runsOf(regenerated)[0].status).toBe("AWAITING_CONFIRMATION");
      const ready = saveTask(first, { ...task, status: "READY" });
      expect(
        editTask(ready, task.key, { text: task.prompt }).production!.drafts![
          task.key
        ].status,
      ).toBe("READY");
      for (const changes of [{ text: "New prompt" }, { references: [] }]) {
        const changed = editTask(ready, task.key, changes);
        expect(changed.production!.drafts![task.key].status).toBe(
          "AWAITING_CONFIRMATION",
        );
        expect(runsOf(changed)).toHaveLength(1);
      }
      expect(
        runsOf(
          requestTask(
            first,
            { ...op, text: "Different composition" },
            { ...context, callId: "variant" },
          ),
        ),
      ).toHaveLength(2);
      const uiOnly = saveTask(first, {
        ...task,
        inputs: task.inputs.map((i) => ({ ...i, key: "different-ui-key" })),
      });
      expect(() =>
        requestTask(uiOnly, op, { ...context, callId: "ui-change" }),
      ).toThrow(task.key);
    });
  }
}
