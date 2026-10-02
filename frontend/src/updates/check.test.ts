import { expect, test } from "bun:test";
import { compareRelease } from "./check";
const release = (tag_name: string) => ({
  tag_name,
  draft: false,
  prerelease: false,
});
test("compares numeric versions and stable releases after prereleases", () => {
  expect(compareRelease("0.9.0", release("v0.10.0")).status).toBe("available");
  expect(compareRelease("0.1.0", release("v0.1.0")).status).toBe("current");
  expect(compareRelease("1.0.0", release("v0.9.9")).status).toBe("current");
  expect(compareRelease("1.0.0-beta.1", release("1.0.0")).status).toBe(
    "available",
  );
});
test("unpublished and invalid responses never claim current", () => {
  expect(compareRelease("0.1.0", null)).toEqual({ status: "unpublished" });
  for (const value of [
    {},
    release("nightly"),
    release("v1.0.0-beta"),
    { ...release("v1.0.0"), draft: true },
    { ...release("v1.0.0"), prerelease: true },
  ]) {
    expect(() => compareRelease("0.1.0", value)).toThrow();
  }
});
