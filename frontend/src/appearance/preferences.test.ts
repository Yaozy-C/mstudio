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
  expect(
    parseAppearance('{"surface":"white","contrast":"high","theme":"dark"}'),
  ).toEqual({ surface: "white", contrast: "high" });
});

test("cool slate preferences survive reload decoding", () => {
  expect(parseAppearance('{"surface":"slate","contrast":"high"}')).toEqual({
    surface: "slate",
    contrast: "high",
  });
});
