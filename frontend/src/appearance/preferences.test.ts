import { expect, test } from "bun:test";
import { parseAppearance } from "./preferences";

test("missing, corrupted and unsupported preferences retain safe defaults", () => {
  for (const raw of [
    null,
    "{bad",
    "null",
    '{"surface":"unknown","contrast":3}',
  ]) {
    expect(parseAppearance(raw)).toEqual({
      surface: "paper",
      contrast: "standard",
    });
  }
});
test("saved appearance survives decoding without accepting unrelated settings", () => {
  for (const surface of ["white", "slate"] as const)
    expect(
      parseAppearance(
        JSON.stringify({ surface, contrast: "high", theme: "dark" }),
      ),
    ).toEqual({ surface, contrast: "high" });
});
