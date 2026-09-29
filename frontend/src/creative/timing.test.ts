import { expect, test } from "bun:test";
import { retimeScript, scriptDuration } from "./timing";
import { patchScript } from "./scriptOperations";
import {
  createScript,
  updateParagraph,
  splitParagraph,
  scriptChanged,
} from "./script";
import { newProject } from "../model";
test("script retiming preserves exact total, positive durations and paragraph identity", () => {
  const script = patchScript(
    [],
    [
      { id: "a", duration: 4 },
      { id: "b", duration: 7 },
      { id: "c", duration: 6 },
    ],
  );
  const next = retimeScript(script, 30);
  expect(scriptDuration(next)).toBe(30);
  expect(next.map((s) => s.id)).toEqual(["a", "b", "c"]);
  expect(next[1].duration!).toBeGreaterThan(next[0].duration!);
  expect(scriptDuration(retimeScript(script, 0.03))).toBe(0.03);
  expect(retimeScript(script, NaN)).toBe(script);
  expect(retimeScript(script, 0)).toBe(script);
  expect(script[0].duration).toBe(4);
  expect(patchScript(script, [{ id: "a", action: "Open" }])[0].duration).toBe(
    4,
  );
  for (const duration of [0, -1, Infinity, "5", 3601])
    expect(() => patchScript(script, [{ id: "a", duration }])).toThrow();
});
test("script time edits mark linked shots stale without retiming existing media", () => {
  let p = createScript(newProject("time"));
  const screenplay = p.nodes[0],
    id = screenplay.screenplay!.script![0].id;
  p = updateParagraph(p, screenplay.id, id, { action: "Pack", duration: 8 });
  p = splitParagraph(p, screenplay.id, id, 2);
  expect(p.nodes[1].shot!.duration).toBe(4);
  const clips = p.clips;
  p = updateParagraph(p, screenplay.id, id, { duration: 12 });
  expect(scriptChanged(p, p.nodes[1])).toBe(true);
  expect(p.nodes[1].shot!.duration).toBe(4);
  expect(p.clips).toBe(clips);
});
