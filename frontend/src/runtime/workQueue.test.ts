import { expect, test } from "bun:test";
import { workQueue } from "./workQueue";
const tick = () => new Promise((resolve) => setTimeout(resolve, 0));
function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}
test("100 jobs use bounded rolling slots; one slow job does not hold the next batch", async () => {
  const pool = workQueue(4);
  const slow = deferred();
  const done: number[] = [];
  let active = 0,
    maximum = 0;
  for (let i = 0; i < 100; i++)
    pool.add(String(i), async () => {
      active++;
      maximum = Math.max(maximum, active);
      if (i === 0) await slow.promise;
      else await Promise.resolve();
      done.push(i);
      active--;
    });
  await tick();
  expect(maximum).toBe(4);
  expect(done).toHaveLength(99);
  expect(done).not.toContain(0);
  slow.resolve();
  await tick();
  expect(new Set(done).size).toBe(100);
});
test("duplicate dispatch, failure, and shutdown do not leak or replay queued work", async () => {
  const pool = workQueue(1);
  const slow = deferred();
  let calls = 0;
  expect(
    pool.add("one", async () => {
      await slow.promise;
      throw new Error("network");
    }),
  ).toBe(true);
  expect(
    pool.add("one", async () => {
      calls++;
    }),
  ).toBe(false);
  pool.add("two", async () => {
    calls++;
  });
  await tick();
  pool.close();
  slow.resolve();
  await tick();
  expect(calls).toBe(0);
  expect(pool.has("one")).toBe(false);
  expect(pool.add("three", async () => {})).toBe(false);
});
test("slow result downloads do not consume status query slots", async () => {
  const queries = workQueue(4),
    imports = workQueue(2);
  const slow = deferred();
  let statuses = 0,
    downloads = 0;
  for (let i = 0; i < 100; i++)
    queries.add(String(i), async () => {
      statuses++;
      imports.add(String(i), async () => {
        downloads++;
        await slow.promise;
      });
    });
  await tick();
  expect(statuses).toBe(100);
  expect(downloads).toBe(2);
  slow.resolve();
  await tick();
  expect(downloads).toBe(100);
});
