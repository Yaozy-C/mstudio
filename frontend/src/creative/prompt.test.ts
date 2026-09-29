import { expect, test } from "bun:test";
import { newProject } from "../model";
import { applyOperations, inspectProject } from "../assistant/projectCommands";
import { editShotText } from "./shotText";
import { promptStale } from "./prompt";
import { registeredInput, requestPrompt } from "../creation/generationInput";
import { shotBasis } from "./document";

const saveShotPrompt = (
  p: ReturnType<typeof newProject>,
  id: string,
  prompt: string,
) =>
  applyOperations(p, p.revision ?? 0, [
    { op: "update_node", id, shot: { prompt } },
  ]);
function fixture() {
  return applyOperations(newProject("film"), 0, [
    {
      op: "add_node",
      id: "screenplay",
      kind: "screenplay",
      title: "午餐",
      text: "",
      screenplay: {
        script: [
          {
            id: "para",
            title: "Pack",
            action: "Pack",
            dialogue: "",
            sound: "Live",
          },
        ],
      },
    },
    {
      op: "add_node",
      id: "shot",
      kind: "shot",
      title: "装包",
      text: "打开包",
      shot: {
        screenplayId: "screenplay",
        scriptId: "para",
        order: 1,
        duration: 6,
        dialogue: "Let's pack.",
      },
    },
  ]);
}
test("prompt persists independently, is readable by Agent and survives save/reload", () => {
  const p = fixture();
  const q = saveShotPrompt(
    p,
    "shot",
    "Close-up. Open the bag, then hold still.",
  );
  const reloaded = JSON.parse(JSON.stringify(q));
  expect(reloaded.nodes[1].shot.prompt).toBe(
    "Close-up. Open the bag, then hold still.",
  );
  expect(reloaded.nodes[1].text).toBe("打开包");
  expect(reloaded.nodes[1].shot.dialogue).toBe("Let's pack.");
  expect(promptStale(reloaded, reloaded.nodes[1])).toBe(false);
  expect(
    inspectProject(q, { nodeIds: ["shot"], fields: ["prompt"] }).details?.[0]
      .shot?.prompt?.text,
  ).toBe(q.nodes[1].shot?.prompt);
  expect(shotBasis(q.nodes[1])).not.toBe(shotBasis(p.nodes[1]));
});
test("script and screenplay edits flag a saved prompt without overwriting it; canvas movement does not", () => {
  const p = saveShotPrompt(fixture(), "shot", "Keep the chosen camera.");
  const q = editShotText(p, "shot", "action", "慢慢打开包");
  expect(promptStale(q, q.nodes[1])).toBe(true);
  expect(q.nodes[1].shot?.prompt).toBe(p.nodes[1].shot?.prompt);
  const r = applyOperations(p, 0, [
    {
      op: "update_node",
      id: "screenplay",
      screenplay: { script: [{ id: "para", sound: "Music" }] },
    },
  ]);
  expect(promptStale(r, r.nodes[1])).toBe(true);
  const moved = applyOperations(p, 0, [
    { op: "update_node", id: "shot", x: 400 },
  ]);
  expect(promptStale(moved, moved.nodes[1])).toBe(false);
  const fresh = saveShotPrompt(q, "shot", "New slow action, same camera.");
  expect(promptStale(fresh, fresh.nodes[1])).toBe(false);
});
test("Agent writes the same prompt field, validates it atomically, and clears stale state against the new script", () => {
  const p = saveShotPrompt(fixture(), "shot", "Old prompt.");
  const q = applyOperations(p, 0, [
    {
      op: "update_node",
      id: "shot",
      text: "推近并打开包",
      shot: { prompt: "Push in as hands open the bag." },
    },
  ]);
  expect(promptStale(q, q.nodes[1])).toBe(false);
  expect(q.nodes[1].shot?.dialogue).toBe("Let's pack.");
  expect(q.nodes[1].shot?.prompt).toBe("Push in as hands open the bag.");
  expect(() =>
    applyOperations(q, 0, [
      { op: "update_node", id: "shot", shot: { prompt: "x".repeat(12001) } },
    ]),
  ).toThrow();
  expect(q.nodes[1].shot?.prompt).toBe("Push in as hands open the bag.");
  const cleared = saveShotPrompt(q, "shot", "");
  expect(cleared.nodes[1].shot?.prompt).toBe("");
  expect(() => registeredInput(model, cleared, "", [], [])).toThrow();
});
test("the complete prompt preview is byte-for-byte the submitted prompt, including reference roles", () => {
  const p = {
    ...fixture(),
    creation: {
      intent: "Lunch",
      essential: "Keep the bag",
      preserve: "No faces",
      stage: "production" as const,
    },
    assets: [
      {
        id: "img",
        name: "image",
        kind: "image" as const,
        path: "",
        preview: "",
        width: 100,
        height: 100,
        duration: 0,
        hasAudio: false,
      },
    ],
  };
  const refs = [{ assetId: "img", purpose: "bag appearance" }];
  const text = "Hands-only shot. Open and pack.";
  const preview = requestPrompt(p, text, refs);
  expect(
    registeredInput(model, p, text, refs, [
      {
        assetId: "img",
        kind: "image",
        role: "reference",
        url: "https://example.test/image",
      },
    ]).prompt,
  ).toBe(preview);
  expect(preview).toContain("Image 1: bag appearance");
  expect(preview).not.toContain("打开包");
  expect(preview).not.toContain("整片目标");
});

const model = {
  id: "h3",
  name: "H3",
  plugin: "fal",
  kind: "video" as const,
  endpoint: "minimax/h3/reference-to-video",
  params: { duration: 6 },
  enabled: true,
};
