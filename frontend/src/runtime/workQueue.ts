// A rolling, keyed pool: slow jobs occupy only their own slot. Closing drops
// queued work; callers still own cleanup of operations already dispatched.
export function workQueue(concurrency: number) {
  if (!Number.isInteger(concurrency) || concurrency < 1)
    throw new Error("Invalid concurrency");
  const keys = new Set<string>();
  const pending: { key: string; run: () => Promise<void> }[] = [];
  let active = 0;
  let closed = false;
  function pump() {
    while (!closed && active < concurrency && pending.length) {
      const { key, run } = pending.shift()!;
      active++;
      void Promise.resolve()
        .then(run)
        .catch(() => {
          // Each operation reports its domain error before leaving the pool.
        })
        .finally(() => {
          active--;
          keys.delete(key);
          pump();
        });
    }
  }
  return {
    add(key: string, run: () => Promise<void>) {
      if (closed || keys.has(key)) return false;
      keys.add(key);
      pending.push({ key, run });
      pump();
      return true;
    },
    has: (key: string) => keys.has(key),
    close() {
      closed = true;
      for (const { key } of pending) keys.delete(key);
      pending.length = 0;
    },
  };
}
